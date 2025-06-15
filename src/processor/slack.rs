use hmac::{Hmac, Mac};
use sha2::Sha256;
use hex;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

pub fn verify_slack_signature(
    signing_secret: &str,
    body: &str,
    timestamp: &str,
    signature: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();
    
    let request_timestamp: u64 = timestamp.parse()?;
    
    if current_time.abs_diff(request_timestamp) > 60 * 5 {
        return Ok(false);
    }
    
    let base_string = format!("v0:{}:{}", timestamp, body);
    
    let mut mac = HmacSha256::new_from_slice(signing_secret.as_bytes())?;
    mac.update(base_string.as_bytes());
    let result = mac.finalize();
    let calculated_signature = format!("v0={}", hex::encode(result.into_bytes()));
    
    Ok(calculated_signature == signature)
}

pub fn parse_command_text(text: &str) -> Result<crate::types::AttendanceAction, String> {
    match text.trim().to_lowercase().as_str() {
        "in" => Ok(crate::types::AttendanceAction::In),
        "break" => Ok(crate::types::AttendanceAction::Break),
        "back" => Ok(crate::types::AttendanceAction::Back),
        "out" => Ok(crate::types::AttendanceAction::Out),
        _ => Err(format!("Unknown action: {}. Use: in, break, back, or out", text)),
    }
}