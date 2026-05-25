use std::fs;

#[derive(serde::Deserialize, Clone)]
pub struct ApiConfig {
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub server: ServerConfig,
    pub frontend: FrontendConfig,
    #[serde(default)]
    pub jwt: Option<JwtConfig>,
    #[serde(default)]
    _email: Option<EmailConfig>,
}

#[derive(serde::Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_acquire_timeout_secs")]
    pub acquire_timeout_secs: u64,
    #[serde(default = "default_idle_timeout_secs")]
    pub idle_timeout_secs: u64,
    #[serde(default = "default_max_lifetime_secs")]
    pub max_lifetime_secs: u64,
    #[serde(default = "default_connect_timeout_secs")]
    pub connect_timeout_secs: u64,
}

fn default_max_connections() -> u32 {
    100
}

fn default_acquire_timeout_secs() -> u64 {
    30
}

fn default_idle_timeout_secs() -> u64 {
    600
}

fn default_max_lifetime_secs() -> u64 {
    1800
}

fn default_connect_timeout_secs() -> u64 {
    10
}

#[derive(serde::Deserialize, Clone)]
pub struct RedisConfig {
    #[serde(default = "default_redis_url")]
    pub url: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct ServerConfig {
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_app_env")]
    pub app_env: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct FrontendConfig {
    #[serde(default = "default_frontend_url")]
    pub url: String,
    /// 静态文件目录路径，用于 SPA 托管。None 则不托管静态文件
    #[serde(default)]
    pub static_dir: Option<String>,
}

#[derive(serde::Deserialize, Clone)]
pub struct JwtConfig {
    pub secret: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct EmailConfig {
    _api_key: String,
}

fn default_redis_url() -> String {
    "redis://127.0.0.1:6379".to_string()
}

fn default_bind_address() -> String {
    "0.0.0.0:3000".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_app_env() -> String {
    "production".to_string()
}

fn default_frontend_url() -> String {
    "http://localhost:5173".to_string()
}

impl ApiConfig {
    pub fn load(path: &str) -> anyhow::Result<ApiConfig> {
        let config = fs::read_to_string(path)?;
        let cfg: ApiConfig = toml::from_str(&config)?;
        Ok(cfg)
    }
}
