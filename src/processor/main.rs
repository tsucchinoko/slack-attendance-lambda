mod notion;
mod slack;
mod types;

use aws_lambda_events::event::sqs::SqsEvent;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use chrono::{Utc, Local, Datelike, FixedOffset};
use serde::{Deserialize, Serialize};
use types::*;

#[derive(Debug, Serialize, Deserialize)]
struct SqsMessage {
    command: SlackCommand,
    timestamp: String,
}

async fn function_handler(event: LambdaEvent<SqsEvent>) -> Result<(), Error> {
    // Process each SQS message
    for record in event.payload.records {
        if let Some(body) = record.body {
            // Parse the SQS message
            let sqs_message: SqsMessage = serde_json::from_str(&body)?;
            let command = sqs_message.command;
            
            // Process the command
            let response_text = if command.text.trim() == "report" {
                handle_report(&command).await?
            } else {
                handle_attendance(&command).await?
            };
            
            // Send delayed response to Slack
            send_delayed_response(&command.response_url, &response_text).await?;
        }
    }
    
    Ok(())
}

async fn send_delayed_response(response_url: &str, text: &str) -> Result<(), Error> {
    let client = reqwest::Client::new();
    
    let response = SlackResponse {
        response_type: "in_channel".to_string(),
        text: text.to_string(),
    };
    
    client
        .post(response_url)
        .json(&response)
        .send()
        .await?;
    
    Ok(())
}

async fn handle_attendance(command: &SlackCommand) -> Result<String, Error> {
    let action = match slack::parse_command_text(&command.text) {
        Ok(a) => a,
        Err(e) => return Ok(e),
    };

    let notion_client = notion::NotionClient::new(
        std::env::var("NOTION_API_KEY")?,
        std::env::var("NOTION_DATABASE_ID")?,
    );

    let now = Utc::now();
    let jst_offset = FixedOffset::east_opt(9 * 3600).unwrap(); // JST = UTC+9
    let jst = now.with_timezone(&jst_offset);
    
    let record = AttendanceRecord {
        user_id: command.user_id.clone(),
        user_name: command.user_name.clone(),
        action: action.clone(),
        timestamp: jst, // JSTの時刻をそのまま保存
        date: jst.format("%Y-%m-%d").to_string(),
    };

    notion_client.create_attendance_record(&record).await?;

    let action_text = match action {
        AttendanceAction::In => "出勤",
        AttendanceAction::Break => "休憩開始",
        AttendanceAction::Back => "休憩終了",
        AttendanceAction::Out => "退勤",
    };

    Ok(format!(
        "{} さんが {} しました ({})",
        command.user_name,
        action_text,
        jst.format("%Y-%m-%d %H:%M:%S")
    ))
}

async fn handle_report(command: &SlackCommand) -> Result<String, Error> {
    let notion_client = notion::NotionClient::new(
        std::env::var("NOTION_API_KEY")?,
        std::env::var("NOTION_DATABASE_ID")?,
    );

    let now = Local::now();
    let report = notion_client
        .get_monthly_report(&command.user_id, now.year(), now.month())
        .await?;

    Ok(format!("{} さんの月次レポート:\n{}", command.user_name, report))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}