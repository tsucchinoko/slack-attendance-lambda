use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SlackCommand {
    pub token: String,
    pub team_id: String,
    pub team_domain: String,
    pub channel_id: String,
    pub channel_name: String,
    pub user_id: String,
    pub user_name: String,
    pub command: String,
    pub text: String,
    pub response_url: String,
    pub trigger_id: String,
}

#[derive(Debug, Serialize)]
pub struct SlackResponse {
    pub response_type: String,
    pub text: String,
}

#[derive(Debug, Serialize, Clone)]
pub enum AttendanceAction {
    #[serde(rename = "in")]
    In,
    #[serde(rename = "break")]
    Break,
    #[serde(rename = "back")]
    Back,
    #[serde(rename = "out")]
    Out,
}

#[derive(Debug, Serialize)]
pub struct AttendanceRecord {
    pub user_id: String,
    pub user_name: String,
    pub action: AttendanceAction,
    pub timestamp: DateTime<FixedOffset>,
    pub date: String,
}

#[derive(Debug, Serialize)]
pub struct NotionPageRequest {
    pub parent: NotionParent,
    pub properties: NotionProperties,
}

#[derive(Debug, Serialize)]
pub struct NotionParent {
    pub database_id: String,
}

#[derive(Debug, Serialize)]
pub struct NotionProperties {
    #[serde(rename = "ユーザーID")]
    pub user_id: NotionTitle,
    #[serde(rename = "ユーザー名")]
    pub user_name: NotionRichText,
    #[serde(rename = "アクション")]
    pub action: NotionSelect,
    #[serde(rename = "タイムスタンプ")]
    pub timestamp: NotionDate,
    #[serde(rename = "日付")]
    pub date: NotionRichText,
}

#[derive(Debug, Serialize)]
pub struct NotionTitle {
    pub title: Vec<NotionTextContent>,
}

#[derive(Debug, Serialize)]
pub struct NotionRichText {
    pub rich_text: Vec<NotionTextContent>,
}

#[derive(Debug, Serialize)]
pub struct NotionTextContent {
    pub text: NotionText,
}

#[derive(Debug, Serialize)]
pub struct NotionText {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct NotionSelect {
    pub select: NotionOption,
}

#[derive(Debug, Serialize)]
pub struct NotionOption {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct NotionDate {
    pub date: NotionDateValue,
}

#[derive(Debug, Serialize)]
pub struct NotionDateValue {
    pub start: String,
}