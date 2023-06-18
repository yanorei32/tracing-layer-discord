use std::collections::HashMap;
use tracing::{info, instrument, warn};
use tracing_subscriber::{layer::SubscriberExt, Registry};

use tracing_layer_discord::DiscordLayer;

#[instrument]
pub async fn create_user(id: u64) {
    app_users_webhook(id).await;
    info!(param = id, "A user was created");
}

#[instrument]
pub async fn app_users_webhook(id: u64) {
    let h = serde_json::from_str::<HashMap<String, String>>(
        r#"
   {
  "method": "POST",
  "requestId": "798b92eb-0ed3-4a0b-8749-c3dc54423c93",
  "uri": "/v1/users/webhook",
  "environment": "dev",
}
    "#,
    )
    .unwrap();
    warn!(
        ?h,
        r#"error parsing user event by webhook handler: failed to parse event metadata: none found"#
    );
}

#[instrument]
pub async fn controller() {
    info!("Orphan event without a parent span");
    app_users_webhook(2).await;
    // tokio::join!(create_user(2), create_user(4), create_user(6));
}

#[tokio::main]
async fn main() {
    let (discord_layer, background_worker) = DiscordLayer::builder("test-app".to_string(), Default::default()).build();
    let subscriber = Registry::default().with(discord_layer);
    tracing::subscriber::set_global_default(subscriber).unwrap();
    controller().await;
    background_worker.shutdown().await;
}
