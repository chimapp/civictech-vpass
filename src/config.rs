use secrecy::Secret;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub base_url: String,
    pub host: String,
    pub port: u16,

    // YouTube OAuth
    pub youtube_client_id: String,
    pub youtube_client_secret: Secret<String>,

    // YouTube Data API (for channel info lookup)
    pub youtube_api_key: Option<String>,

    // Taiwan Digital Wallet Issuer API
    pub issuer_api_url: Option<String>,
    pub issuer_access_token: Option<Secret<String>>,

    // Taiwan Digital Wallet Verifier API (OIDVP)
    pub verifier_api_url: Option<String>,
    pub verifier_access_token: Option<Secret<String>>,

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
            host: config.get("host").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: config.get("port")?,

            youtube_client_id: config.get("youtube_client_id")?,
            youtube_client_secret: Secret::new(config.get("youtube_client_secret")?),

            youtube_api_key: config.get("youtube_api_key").ok(),

            issuer_api_url: config.get("issuer_api_url").ok(),
            issuer_access_token: config
                .get::<String>("issuer_access_token")
                .ok()
                .map(Secret::new),

            verifier_api_url: config.get("verifier_api_url").ok(),
            verifier_access_token: config
                .get::<String>("verifier_access_token")
                .ok()
                .map(Secret::new),

            session_secret: Secret::new(config.get("session_secret")?),
        })
    }
}
