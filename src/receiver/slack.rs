use hmac::{Hmac, Mac};
use sha2::Sha256;

pub fn verify_slack_signature(
    signing_secret: &str,
    body: &str,
    timestamp: &str,
    signature: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    // Check timestamp to prevent replay attacks (5 minutes)
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let slack_timestamp: u64 = timestamp.parse()?;
    
    if (current_time as i64 - slack_timestamp as i64).abs() > 300 {
        return Ok(false);
    }
    
    // Create signature base string
    let sig_basestring = format!("v0:{}:{}", timestamp, body);
    
    // Create HMAC
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(signing_secret.as_bytes())?;
    mac.update(sig_basestring.as_bytes());
    
    // Get computed signature
    let computed_signature = format!("v0={}", hex::encode(mac.finalize().into_bytes()));
    
    Ok(computed_signature == signature)
}