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

    // YouTube Data API (for channel info lookup)
    pub youtube_api_key: Option<String>,

    // Security
    pub session_secret: Secret<String>,
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

            youtube_api_key: config.get("youtube_api_key").ok(),

            session_secret: Secret::new(config.get("session_secret")?),
        })
    }
}
