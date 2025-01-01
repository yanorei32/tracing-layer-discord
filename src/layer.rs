use serde::ser::{SerializeMap, Serializer};
use serde_json::Value;
use tracing::{Event, Subscriber};
use tracing_bunyan_formatter::JsonStorage;
use tracing_subscriber::{layer::Context, Layer};

use crate::message::PayloadMessageType;
use crate::worker::{BackgroundWorker, WorkerMessage};
use crate::{config::DiscordConfig, message::MessagePayload, worker::worker, ChannelSender};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Layer for forwarding tracing events to Discord.
pub struct DiscordLayer {
    app_name: String,

    /// Configure the layer's connection to the Discord Webhook API.
    config: DiscordConfig,

    /// An unbounded sender, which the caller must send `WorkerMessage::Shutdown` in order to cancel
    /// worker's receive-send loop.
    discord_sender: ChannelSender,
}

impl DiscordLayer {
    /// Create a new layer for forwarding messages to Discord, using a specified
    /// configuration. This method spawns a task onto the tokio runtime to begin sending tracing
    /// events to Discord.
    ///
    /// Returns the tracing_subscriber::Layer impl to add to a registry, an unbounded-mpsc sender
    /// used to shutdown the background worker, and a future to spawn as a task on a tokio runtime
    /// to initialize the worker's processing and sending of HTTP requests to the Discord API.
    pub(crate) fn new(app_name: String, config: DiscordConfig) -> (DiscordLayer, BackgroundWorker) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        let layer = DiscordLayer {
            app_name,
            config,
            discord_sender: tx.clone(),
        };

        let worker = BackgroundWorker {
            sender: tx,
            handle: Arc::new(Mutex::new(Some(tokio::spawn(worker(rx))))),
        };

        (layer, worker)
    }

    /// Create a new builder for DiscordLayer.
    pub fn builder(app_name: String) -> DiscordLayerBuilder {
        DiscordLayerBuilder::new(app_name)
    }
}

/// A builder for creating a Discord layer.
///
/// Several methods expose initialization of optional filtering mechanisms, along with Discord
/// configuration that defaults to searching in the local environment variables.
pub struct DiscordLayerBuilder {
    app_name: String,
    config: Option<DiscordConfig>,
}

impl DiscordLayerBuilder {
    pub(crate) fn new(app_name: String) -> Self {
        Self {
            app_name,
            config: None,
        }
    }

    /// Configure the layer's connection to the Discord Webhook API.
    pub fn discord_config(mut self, config: DiscordConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Create a DiscordLayer and its corresponding background worker to (async) send the messages.
    pub fn build(self) -> (DiscordLayer, BackgroundWorker) {
        DiscordLayer::new(
            self.app_name,
            self.config.unwrap_or_else(DiscordConfig::new_from_env),
        )
    }
}

impl<S> Layer<S> for DiscordLayer
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let current_span = ctx.lookup_current();
        let mut event_visitor = JsonStorage::default();
        event.record(&mut event_visitor);

        let format = || {
            const KEYWORDS: [&str; 2] = ["message", "error"];

            let target = event.metadata().target();

            // Extract the "message" field, if provided. Fallback to the target, if missing.
            let message = event_visitor
                .values()
                .get("message")
                .and_then(|v| match v {
                    Value::String(s) => Some(s.as_str()),
                    _ => None,
                })
                .or_else(|| {
                    event_visitor.values().get("error").and_then(|v| match v {
                        Value::String(s) => Some(s.as_str()),
                        _ => None,
                    })
                })
                .unwrap_or("No message");

            let mut metadata_buffer = Vec::new();
            let mut serializer = serde_json::Serializer::new(&mut metadata_buffer);
            let mut map_serializer = serializer.serialize_map(None)?;
            // Add all the other fields associated with the event, expect the message we
            // already used.

            for (key, value) in event_visitor
                .values()
                .iter()
                .filter(|(&key, _)| !KEYWORDS.contains(&key))
            {
                map_serializer.serialize_entry(key, value)?;
            }

            // Add all the fields from the current span, if we have one.
            if let Some(span) = &current_span {
                let extensions = span.extensions();
                if let Some(visitor) = extensions.get::<JsonStorage>() {
                    for (key, value) in visitor.values() {
                        map_serializer.serialize_entry(key, value)?;
                    }
                }
            }
            map_serializer.end()?;

            let span = match &current_span {
                Some(span) => span.metadata().name(),
                None => "",
            };

            let metadata = {
                let data: HashMap<String, serde_json::Value> =
                    serde_json::from_slice(metadata_buffer.as_slice()).unwrap();
                serde_json::to_string_pretty(&data).unwrap()
            };

            Ok(Self::format_payload(
                self.app_name.as_str(),
                message,
                event,
                target,
                span,
                metadata,
            ))
        };

        let result: Result<PayloadMessageType, Box<dyn std::error::Error>> = format();
        if let Ok(formatted) = result {
            let payload = MessagePayload::new(formatted, self.config.webhook_url.clone());

            if let Err(e) = self.discord_sender.send(WorkerMessage::Data(payload)) {
                tracing::error!(err = %e, "failed to send discord payload to given channel")
            };
        }
    }
}

impl DiscordLayer {
    fn format_payload(
        app_name: &str,
        message: &str,
        event: &Event,
        target: &str,
        span: &str,
        metadata: String,
    ) -> PayloadMessageType {
        let event_level = event.metadata().level();

        let event_level_emoji = match *event_level {
            tracing::Level::TRACE => ":mag:",
            tracing::Level::DEBUG => ":bug:",
            tracing::Level::INFO => ":information_source:",
            tracing::Level::WARN => ":warning:",
            tracing::Level::ERROR => ":x:",
        };

        let event_level_color = match *event_level {
            tracing::Level::TRACE => 0x1abc9c,
            tracing::Level::DEBUG => 0x1abc9c,
            tracing::Level::INFO => 0x57f287,
            tracing::Level::WARN => 0xe67e22,
            tracing::Level::ERROR => 0xed4245,
        };

        let source_file = event.metadata().file().unwrap_or("Unknown");
        let source_line = event.metadata().line().unwrap_or(0);

        // Maximum characters allowed for a Discord field value
        const MAX_FIELD_VALUE_CHARS: usize = 1024 - 15;
        const MAX_ERROR_MESSAGE_CHARS: usize = 2048 - 15;

        let (message, _overflowed) = message.split_at(MAX_ERROR_MESSAGE_CHARS);

        let mut discord_embed = serde_json::json!({
            "title": format!("{} - {} {}", app_name, event_level_emoji, event_level),
            "description": format!("```rust\n{}\n```", message),
            "fields": [
                {
                    "name": "Target Span",
                    "value": format!("`{}::{}`", target, span),
                    "inline": true
                },
                {
                    "name": "Source",
                    "value": format!("`{}#L{}`", source_file, source_line),
                    "inline": true
                },
            ],
            "footer": {
                "text": app_name
            },
            "color": event_level_color,
        });

        // Check if metadata exceeds the limit
        if metadata.len() <= MAX_FIELD_VALUE_CHARS {
            // Metadata fits within a single field
            discord_embed["fields"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({
                    "name": "Metadata",
                    "value": format!("```json\n{}\n```", metadata),
                    "inline": false
                }));
        } else {
            // Metadata exceeds the limit, split into multiple fields
            let mut remaining_metadata = metadata;
            let mut chunk_number = 1;
            while !remaining_metadata.is_empty() {
                let chunk = remaining_metadata
                    .chars()
                    .take(MAX_FIELD_VALUE_CHARS)
                    .collect::<String>();

                remaining_metadata = remaining_metadata
                    .chars()
                    .skip(MAX_FIELD_VALUE_CHARS)
                    .collect();

                discord_embed["fields"]
                    .as_array_mut()
                    .unwrap()
                    .push(serde_json::json!({
                        "name": format!("Metadata ({})", chunk_number),
                        "value": format!("```json\n{}\n```", chunk),
                        "inline": false
                    }));

                chunk_number += 1;
            }
        }

        PayloadMessageType::EmbedNoText(vec![discord_embed])
    }
}
