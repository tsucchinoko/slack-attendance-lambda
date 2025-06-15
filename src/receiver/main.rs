use lambda_http::{run, service_fn, Error, Request, Response, Body};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

// Import shared types and local modules
#[path = "../types.rs"]
mod types;
mod slack;

use types::{SlackCommand, SlackResponse};

#[derive(Debug, Serialize)]
struct SqsMessage {
    command: SlackCommand,
    timestamp: String,
}

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

    // Send message to SQS
    let sqs_message = SqsMessage {
        command: command.clone(),
        timestamp: slack_timestamp.to_string(),
    };

    send_to_sqs(sqs_message).await?;

    // Return immediate response to Slack
    let response = SlackResponse {
        response_type: "in_channel".to_string(),
        text: format!("コマンドを受け付けました。処理中です... ⏳"),
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&response)?))?)
}

async fn send_to_sqs(message: SqsMessage) -> Result<(), Error> {
    let config = aws_config::load_from_env().await;
    let sqs_client = aws_sdk_sqs::Client::new(&config);
    
    let queue_url = std::env::var("SQS_QUEUE_URL")?;
    let message_body = serde_json::to_string(&message)?;

    sqs_client
        .send_message()
        .queue_url(queue_url)
        .message_body(message_body)
        .send()
        .await?;

    Ok(())
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