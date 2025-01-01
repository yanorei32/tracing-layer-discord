#[derive(Debug)]
pub(crate) enum WorkerRequest {
    Post(serde_json::Value),
    Shutdown,
}

pub(crate) trait TracingLevelExt {
    fn as_emoji(&self) -> &'static str;
    fn as_color(&self) -> u32;
}

impl TracingLevelExt for tracing::Level {
    fn as_emoji(&self) -> &'static str {
        match *self {
            tracing::Level::TRACE => ":mag:",
            tracing::Level::DEBUG => ":bug:",
            tracing::Level::INFO => ":information_source:",
            tracing::Level::WARN => ":warning:",
            tracing::Level::ERROR => ":x:",
        }
    }

    fn as_color(&self) -> u32 {
        match *self {
            tracing::Level::TRACE => 0x1abc9c,
            tracing::Level::DEBUG => 0x1abc9c,
            tracing::Level::INFO => 0x57f287,
            tracing::Level::WARN => 0xe67e22,
            tracing::Level::ERROR => 0xed4245,
        }
    }
}
