mod notion;
mod slack;
mod types;

use lambda_http::{run, service_fn, Error, Request, Response, Body};
use chrono::{Utc, Local, Datelike, FixedOffset};
use std::collections::HashMap;
use types::*;

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Get body as string
    let body_bytes = event.body().to_vec();
    let body_string = String::from_utf8(body_bytes)?;
    
    // Get headers
    let slack_signature = event.headers()
        .get("X-Slack-Signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let slack_timestamp = event.headers()
        .get("X-Slack-Request-Timestamp")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let signing_secret = std::env::var("SLACK_SIGNING_SECRET")?;

    // Verify Slack signature
    if !slack::verify_slack_signature(&signing_secret, &body_string, slack_timestamp, slack_signature)? {
        return Ok(Response::builder()
            .status(401)
            .header("Content-Type", "text/plain")
            .body(Body::from("Unauthorized"))?);
    }

    // Parse form data
    let params: HashMap<String, String> = serde_urlencoded::from_str(&body_string)?;
    
    let command = SlackCommand {
        token: params.get("token").unwrap_or(&String::new()).clone(),
        team_id: params.get("team_id").unwrap_or(&String::new()).clone(),
        team_domain: params.get("team_domain").unwrap_or(&String::new()).clone(),
        channel_id: params.get("channel_id").unwrap_or(&String::new()).clone(),
        channel_name: params.get("channel_name").unwrap_or(&String::new()).clone(),
        user_id: params.get("user_id").unwrap_or(&String::new()).clone(),
        user_name: params.get("user_name").unwrap_or(&String::new()).clone(),
        command: params.get("command").unwrap_or(&String::new()).clone(),
        text: params.get("text").unwrap_or(&String::new()).clone(),
        response_url: params.get("response_url").unwrap_or(&String::new()).clone(),
        trigger_id: params.get("trigger_id").unwrap_or(&String::new()).clone(),
    };

    let response_text = if command.text.trim() == "report" {
        handle_report(&command).await?
    } else {
        handle_attendance(&command).await?
    };

    let response = SlackResponse {
        response_type: "in_channel".to_string(),
        text: response_text,
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&response)?))?)
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