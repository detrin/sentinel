use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub smtp: SmtpConfig,
    pub server: ServerConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_address: String,
}

#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub scripts_dir: String,
    pub script_timeout_seconds: u64,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let _ = dotenvy::dotenv();

        Ok(Config {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:sentinel.db".to_string()),
            smtp: SmtpConfig {
                host: env::var("SMTP_HOST")?,
                port: env::var("SMTP_PORT")?.parse()?,
                username: env::var("SMTP_USERNAME")?,
                password: env::var("SMTP_PASSWORD")?,
                from: env::var("SMTP_FROM")?,
            },
            server: ServerConfig {
                bind_address: env::var("BIND_ADDRESS")
                    .unwrap_or_else(|_| "0.0.0.0:3000".to_string()),
            },
            security: SecurityConfig {
                scripts_dir: env::var("SCRIPTS_DIR")
                    .unwrap_or_else(|_| "./scripts".to_string()),
                script_timeout_seconds: env::var("SCRIPT_TIMEOUT_SECONDS")
                    .unwrap_or_else(|_| "60".to_string())
                    .parse()?,
            },
        })
    }
}
