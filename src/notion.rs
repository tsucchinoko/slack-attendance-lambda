use crate::types::*;
use reqwest::Client;

pub struct NotionClient {
    client: Client,
    api_key: String,
    database_id: String,
}

impl NotionClient {
    pub fn new(api_key: String, database_id: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            database_id,
        }
    }

    pub async fn create_attendance_record(
        &self,
        record: &AttendanceRecord,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let properties = NotionProperties {
            user_id: NotionTitle {
                title: vec![NotionTextContent {
                    text: NotionText {
                        content: record.user_id.clone(),
                    },
                }],
            },
            user_name: NotionRichText {
                rich_text: vec![NotionTextContent {
                    text: NotionText {
                        content: record.user_name.clone(),
                    },
                }],
            },
            action: NotionSelect {
                select: NotionOption {
                    name: match &record.action {
                        AttendanceAction::In => "出勤".to_string(),
                        AttendanceAction::Break => "休憩入り".to_string(),
                        AttendanceAction::Back => "休憩戻り".to_string(),
                        AttendanceAction::Out => "退勤".to_string(),
                    },
                },
            },
            timestamp: NotionDate {
                date: NotionDateValue {
                    start: record.timestamp.to_rfc3339(),
                },
            },
            date: NotionRichText {
                rich_text: vec![NotionTextContent {
                    text: NotionText {
                        content: record.date.clone(),
                    },
                }],
            },
        };

        let request_body = NotionPageRequest {
            parent: NotionParent {
                database_id: self.database_id.clone(),
            },
            properties,
        };

        let response = self
            .client
            .post("https://api.notion.com/v1/pages")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Notion-Version", "2022-06-28")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Notion API error: {}", error_text).into());
        }

        Ok(())
    }

    pub async fn get_monthly_report(
        &self,
        user_id: &str,
        year: i32,
        month: u32,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let _start_date = format!("{}-{:02}-01", year, month);
        let _end_date = if month == 12 {
            format!("{}-01-01", year + 1)
        } else {
            format!("{}-{:02}-01", year, month + 1)
        };

        let filter = serde_json::json!({
            "and": [
                {
                    "property": "ユーザーID",
                    "title": {
                        "equals": user_id
                    }
                },
                {
                    "property": "日付",
                    "rich_text": {
                        "contains": format!("{}-{:02}", year, month)
                    }
                }
            ]
        });

        let request_body = serde_json::json!({
            "filter": filter,
            "sorts": [
                {
                    "property": "タイムスタンプ",
                    "direction": "ascending"
                }
            ]
        });

        let response = self
            .client
            .post(format!("https://api.notion.com/v1/databases/{}/query", self.database_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Notion-Version", "2022-06-28")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Notion API error: {}", error_text).into());
        }

        let data: serde_json::Value = response.json().await?;
        
        Ok(format!("Monthly report for {}/{}: {} records found", year, month, 
            data["results"].as_array().map(|a| a.len()).unwrap_or(0)))
    }
}