use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmailDoc {
    pub id: String,
    pub subject: String,
    pub from: String,
    pub to: String,
    pub date: String, // ISO 8601
    pub labels: Vec<String>,
    pub body_text: String,
    pub body_html: String,
    pub has_attachment: bool,
    pub attachments: serde_json::Value,
}
