/// Configuration describing how to forward tracing events to Discord.
pub struct DiscordConfig {
    pub(crate) webhook_url: String,
}

impl DiscordConfig {
    pub fn new(webhook_url: String) -> Self {
        Self { webhook_url }
    }

    /// Create a new config for forwarding messages to Discord using configuration
    /// available in the environment.
    ///
    /// Required env vars:
    ///   * SLACK_WEBHOOK_URL
    pub fn new_from_env() -> Self {
        Self::new(std::env::var("DISCORD_WEBHOOK_URL").expect("discord webhook url in env"))
    }
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self::new_from_env()
    }
}
