use serde::Serialize;

/// The message sent to Discord. The logged record being "drained" will be
/// converted into this format.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct MessagePayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    embeds: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing)]
    webhook_url: String,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum PayloadMessageType {
    TextNoEmbed(String),
    TextWithEmbed(String, Vec<serde_json::Value>),
    EmbedNoText(Vec<serde_json::Value>),
}

impl MessagePayload {
    pub(crate) fn new(payload: PayloadMessageType, webhook_url: String) -> Self {
        let text;
        let embed;
        match payload {
            PayloadMessageType::TextNoEmbed(t) => {
                text = Some(t);
                embed = None;
            }
            PayloadMessageType::TextWithEmbed(t, e) => {
                text = Some(t);
                embed = Some(e);
            }
            PayloadMessageType::EmbedNoText(e) => {
                text = None;
                embed = Some(e);
            }
        }
        Self {
            content: text,
            embeds: embed,
            webhook_url,
        }
    }
}

impl MessagePayload {
    pub fn webhook_url(&self) -> &str {
        self.webhook_url.as_str()
    }
}
