use crate::types::*;
use reqwest::Client;
use chrono::Timelike;

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
                    start: record.timestamp.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
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
        let results = data["results"].as_array().ok_or("No results found")?;
        
        // 日付ごとの勤務記録を格納
        use std::collections::HashMap;
        use chrono::{DateTime, FixedOffset, Utc};
        
        #[derive(Debug)]
        struct DayRecord {
            in_time: Option<DateTime<Utc>>,
            out_time: Option<DateTime<Utc>>,
            breaks: Vec<(DateTime<Utc>, Option<DateTime<Utc>>)>,
        }
        
        let mut daily_records: HashMap<String, DayRecord> = HashMap::new();
        
        // レコードを日付ごとに整理
        for result in results {
            let properties = &result["properties"];
            
            // 日付を取得
            let date = properties["日付"]["rich_text"][0]["text"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string();
            
            // タイムスタンプを取得
            let timestamp_str = properties["タイムスタンプ"]["date"]["start"]
                .as_str()
                .unwrap_or("");
            let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .ok();
            
            // アクションを取得
            let action = properties["アクション"]["select"]["name"]
                .as_str()
                .unwrap_or("");
            
            if let Some(ts) = timestamp {
                let record = daily_records.entry(date.clone()).or_insert(DayRecord {
                    in_time: None,
                    out_time: None,
                    breaks: Vec::new(),
                });
                
                match action {
                    "出勤" => record.in_time = Some(ts),
                    "退勤" => record.out_time = Some(ts),
                    "休憩入り" => record.breaks.push((ts, None)),
                    "休憩戻り" => {
                        if let Some(last_break) = record.breaks.last_mut() {
                            if last_break.1.is_none() {
                                last_break.1 = Some(ts);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        
        // 勤務時間を計算
        let mut total_work_minutes = 0i64;
        let mut work_days = 0;
        let mut report_lines = Vec::new();
        
        let mut dates: Vec<_> = daily_records.keys().collect();
        dates.sort();
        
        for date in dates {
            let record = &daily_records[date];
            
            if let (Some(in_time), Some(out_time)) = (record.in_time, record.out_time) {
                work_days += 1;
                
                // 勤務時間を計算（分単位）
                let work_duration = out_time - in_time;
                let mut work_minutes = work_duration.num_minutes();
                
                // 休憩時間を引く
                let mut break_minutes = 0i64;
                for (break_start, break_end) in &record.breaks {
                    if let Some(end) = break_end {
                        let break_duration = *end - *break_start;
                        break_minutes += break_duration.num_minutes();
                    }
                }
                
                work_minutes -= break_minutes;
                total_work_minutes += work_minutes;
                
                // 日本時間に変換して表示
                let jst_offset = FixedOffset::east_opt(9 * 3600).unwrap();
                let in_time_jst = in_time.with_timezone(&jst_offset);
                let out_time_jst = out_time.with_timezone(&jst_offset);
                
                report_lines.push(format!(
                    "{}  {:02}:{:02} - {:02}:{:02}  勤務: {}時間{}分  休憩: {}時間{}分",
                    date,
                    in_time_jst.hour(), in_time_jst.minute(),
                    out_time_jst.hour(), out_time_jst.minute(),
                    work_minutes / 60, work_minutes % 60,
                    break_minutes / 60, break_minutes % 60
                ));
            }
        }
        
        // 合計時間を計算
        let total_hours = total_work_minutes / 60;
        let total_minutes = total_work_minutes % 60;
        
        let mut report = format!(
            "{}年{}月の勤怠レポート\n\n",
            year, month
        );
        
        if report_lines.is_empty() {
            report.push_str("勤務記録がありません");
        } else {
            report.push_str(&report_lines.join("\n"));
            report.push_str(&format!(
                "\n\n合計: {}日勤務  {}時間{}分",
                work_days, total_hours, total_minutes
            ));
        }
        
        Ok(report)
    }
}