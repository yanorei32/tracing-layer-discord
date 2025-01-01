use crate::message::MessagePayload;
use crate::{ChannelReceiver, ChannelSender};
use debug_print::debug_println;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

/// Maximum number of retries for failed requests
const MAX_RETRIES: usize = 10;

/// Provides a background worker task that sends the messages generated by the
/// layer.
pub(crate) async fn worker(mut rx: ChannelReceiver) {
    let client = reqwest::Client::new();
    while let Some(message) = rx.recv().await {
        match message {
            WorkerMessage::Data(payload) => {
                let payload_str = serde_json::to_string(&payload)
                    .expect("failed to deserialize discord payload, this is a bug");

                debug_println!("sending discord message: {}", payload_str);

                let mut retries = 0;
                while retries < MAX_RETRIES {
                    match client
                        .post(payload.webhook_url())
                        .header("Content-Type", "application/json")
                        .body(payload_str.clone())
                        .send()
                        .await
                    {
                        Ok(res) => {
                            debug_println!("discord message sent: {:?}", &res);
                            let res_text = res.text().await.unwrap();
                            debug_println!("discord message response: {}", res_text);
                            break; // Success, break out of the retry loop
                        }
                        Err(e) => {
                            println!("ERROR: failed to send discord message: {}", e);
                        }
                    };

                    // Exponential backoff - increase the delay between retries
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    retries += 1;
                }
            }
            WorkerMessage::Shutdown => {
                break;
            }
        }
    }
}

/// This worker manages a background async task that schedules the network requests to send traces
/// to the Discord on the running tokio runtime.
///
/// Ensure to invoke `.startup()` before, and `.teardown()` after, your application code runs. This
/// is required to ensure proper initialization and shutdown.
///
/// `tracing-layer-discord` synchronously generates payloads to send to the Discord API using the
/// tracing events from the global subscriber. However, all network requests are offloaded onto
/// an unbuffered channel and processed by a provided future acting as an asynchronous worker.
#[derive(Debug, Clone)]
pub struct BackgroundWorker {
    pub(crate) sender: ChannelSender,
    pub(crate) handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl BackgroundWorker {
    /// Initiate the worker's shutdown sequence.
    ///
    /// Without invoking`.teardown()`, your application may exit before all Discord messages can be
    /// sent.
    pub async fn shutdown(self) {
        match self.sender.send(WorkerMessage::Shutdown) {
            Ok(..) => {
                debug_println!("discord worker shutdown");
            }
            Err(e) => {
                println!(
                    "ERROR: failed to send shutdown message to discord worker: {}",
                    e
                );
            }
        }
        let mut guard = self.handle.lock().await;
        if let Some(handle) = guard.take() {
            let _ = handle.await;
        } else {
            println!("ERROR: worker handle is already dropped");
        }
    }
}

#[derive(Debug)]
pub(crate) enum WorkerMessage {
    Data(MessagePayload),
    Shutdown,
}
