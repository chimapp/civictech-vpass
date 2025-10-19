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
        // Load .env file if it exists (for local development)
        let _ = dotenvy::dotenv();

        let config = config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()?;

        Ok(Self {
            database_url: config.get("database_url")?,
            base_url: config.get("base_url")?,
            port: config.get("port")?,

            youtube_client_id: config.get("youtube_client_id")?,
            youtube_client_secret: Secret::new(config.get("youtube_client_secret")?),

            twitch_client_id: config.get("twitch_client_id").unwrap_or_default(),
            twitch_client_secret: Secret::new(
                config.get("twitch_client_secret").unwrap_or_default(),
            ),

            session_secret: Secret::new(config.get("session_secret")?),
            encryption_key: Secret::new(config.get("encryption_key")?),

            subscription_check_schedule: config
                .get("subscription_check_schedule")
                .unwrap_or_else(|_| "0 0 */6 * * *".to_string()), // Every 6 hours by default
        })
    }
}
