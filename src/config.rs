use secrecy::Secret;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub base_url: String,
    pub port: u16,

    // YouTube OAuth
    pub youtube_client_id: String,
    pub youtube_client_secret: Secret<String>,

    // Twitch OAuth
    pub twitch_client_id: String,
    pub twitch_client_secret: Secret<String>,

    // Security
    pub session_secret: Secret<String>,
    pub encryption_key: Secret<String>,

    // Cron
    pub subscription_check_schedule: String,
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        // TODO: T011 - Implement full configuration loading
        // - Load from .env file
        // - Validate all required fields are present
        // - Provide helpful error messages for missing config
        todo!("Implement configuration loading from environment")
    }
}
