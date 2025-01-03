use tokio::sync::{mpsc, oneshot};
use tracing::{Event, Subscriber};
use tracing_bunyan_formatter::JsonStorage;
use tracing_subscriber::{layer::Context, Layer};

use crate::model::{TracingLevelExt, WorkerRequest};
use crate::worker::worker;

const MAX_FIELD_VALUE_LENGTH: usize = 1024;
const MAX_DESCRIPTION_LENGTH: usize = 2048;

/// Layer for forwarding tracing events to Discord.
pub struct DiscordLayer {
    app_name: String,
    message_tx: mpsc::UnboundedSender<WorkerRequest>,
}

impl DiscordLayer {
    /// Create a new layer for forwarding messages to Discord, using a specified
    /// configuration. This method spawns a task onto the tokio runtime to begin sending tracing
    /// events to Discord.
    ///
    /// Returns the tracing_subscriber::Layer impl to add to a registry, an unbounded-mpsc sender
    /// used to the background worker, and a future to spawn as a task on a tokio runtime
    /// to initialize the worker's processing and sending of HTTP requests to the Discord API.
    pub fn new(app_name: &str, webhook_url: &str) -> (DiscordLayer, Shutdowner) {
        let (shutdowned_tx, shutdowned_rx) = oneshot::channel();
        let (message_tx, message_rx) = tokio::sync::mpsc::unbounded_channel();

        let webhook_url = webhook_url.to_string();
        tokio::spawn(async move { worker(&webhook_url, message_rx, shutdowned_tx).await });

        (
            DiscordLayer {
                app_name: app_name.to_string(),
                message_tx: message_tx.clone(),
            },
            Shutdowner {
                shutdown_rx: shutdowned_rx,
                message_tx: message_tx.clone(),
            },
        )
    }
}

#[derive(Debug)]
pub struct Shutdowner {
    shutdown_rx: oneshot::Receiver<()>,
    message_tx: mpsc::UnboundedSender<WorkerRequest>,
}

impl Shutdowner {
    pub async fn shutdown(self) {
        self.message_tx.send(WorkerRequest::Shutdown).unwrap();
        let _ = self.shutdown_rx.await;
    }
}

fn truncate(s: &str, len: usize) -> String {
    if s.len() <= len {
        return s.to_string();
    }

    format!("{}…", &s[..len - 1])
}

impl<S> Layer<S> for DiscordLayer
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut event_record = JsonStorage::default();
        event.record(&mut event_record);

        let values = event_record.values();

        #[allow(clippy::manual_unwrap_or)]
        let heading = if let Some(message) = values.get("message").and_then(|v| v.as_str()) {
            message
        } else if let Some(error) = values.get("error").and_then(|v| v.as_str()) {
            error
        } else {
            "No message"
        };

        let heading = truncate(heading, MAX_DESCRIPTION_LENGTH);

        let span_name = ctx
            .lookup_current()
            .map(|span| span.metadata().name())
            .unwrap_or_default();

        let level = event.metadata().level();

        let src_file = event.metadata().file().unwrap_or("Unknown");
        let src_line = event.metadata().line().unwrap_or_default();

        let mut fields = vec![
            serde_json::json!({
                "name": "Target Span",
                "value": format!("`{}`", truncate(
                    &format!("{}::{}", event.metadata().target(), span_name),
                    MAX_FIELD_VALUE_LENGTH - 2,
                )),
                "inline": true,
            }),
            serde_json::json!({
                "name": "Source",
                "value": format!("`{}`", truncate(
                    &format!("`{}#L{}`", src_file, src_line),
                    MAX_FIELD_VALUE_LENGTH - 2
                )),
                "inline": true,
            }),
        ];

        for (key, value) in values {
            fields.push(serde_json::json!({
                "name": format!("Meta/{}", key),
                "value": format!("`{}`", truncate(
                    &value.to_string(),
                    MAX_FIELD_VALUE_LENGTH - 2
                )),
                "inline": true,
            }));
        }

        if let Some(span) = &ctx.lookup_current() {
            let extensions = span.extensions();
            if let Some(visitor) = extensions.get::<JsonStorage>() {
                for (key, value) in visitor.values() {
                    fields.push(serde_json::json!({
                        "name": format!("SpanExt/{}", key),
                        "value": format!("`{}`", truncate(
                            &value.to_string(),
                            MAX_FIELD_VALUE_LENGTH - 2
                        )),
                        "inline": true,
                    }));
                }
            }
        }

        let query = serde_json::json!({
            "embeds": [{
                "title": format!("{} {} ({})", level.as_emoji(), level, self.app_name),
                "description": heading,
                "fields": fields,
                "footer": { "text": self.app_name },
                "color": level.as_color(),
            }]
        });

        let _ = self.message_tx.send(WorkerRequest::Post(query));
    }
}
