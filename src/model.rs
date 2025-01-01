#[derive(Debug)]
pub(crate) enum WorkerRequest {
    Post(serde_json::Value),
    Shutdown,
}
