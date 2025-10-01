// CLI Configuration Module - Extended config for grid-bot CLI
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Complete CLI configuration structure matching config.toml.example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    pub api: ApiConfig,
    pub trading: TradingDefaults,
    pub optimization: OptimizationConfig,
    pub backtesting: BacktestingConfig,
    pub monitoring: MonitoringConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub api_key: String,
    pub api_secret: String,
    #[serde(default = "default_rest_url")]
    pub rest_url: String,
    #[serde(default = "default_ws_url")]
    pub ws_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingDefaults {
    #[serde(default = "default_capital")]
    pub default_capital: f64,
    #[serde(default = "default_grid_levels")]
    pub default_grid_levels: usize,
    #[serde(default = "default_grid_spacing")]
    pub default_grid_spacing: f64,
    #[serde(default = "default_max_position")]
    pub max_position_size: f64,
    #[serde(default = "default_max_drawdown")]
    pub max_drawdown: f64,
    #[serde(default = "default_stop_loss")]
    pub stop_loss: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    #[serde(default = "default_iterations")]
    pub default_iterations: usize,
    #[serde(default = "default_strategy")]
    pub default_strategy: String,
    #[serde(default = "default_target_metric")]
    pub target_metric: String,
    #[serde(default = "default_grid_levels_range")]
    pub grid_levels_range: [usize; 2],
    #[serde(default = "default_grid_spacing_range")]
    pub grid_spacing_range: [f64; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestingConfig {
    #[serde(default = "default_lookback_days")]
    pub default_lookback_days: usize,
    #[serde(default = "default_transaction_fee")]
    pub transaction_fee: f64,
    #[serde(default = "default_slippage")]
    pub slippage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    #[serde(default = "default_check_interval")]
    pub check_interval_seconds: u64,
    #[serde(default = "default_true")]
    pub alert_on_error: bool,
    #[serde(default = "default_true")]
    pub alert_on_large_drawdown: bool,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_true")]
    pub log_to_file: bool,
    #[serde(default = "default_log_dir")]
    pub log_directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_path")]
    pub db_path: String,
    #[serde(default = "default_backup_interval")]
    pub backup_interval_hours: u64,
}

// Default value functions
fn default_rest_url() -> String { "https://api.kraken.com".to_string() }
fn default_ws_url() -> String { "wss://ws.kraken.com".to_string() }
fn default_capital() -> f64 { 500.0 }
fn default_grid_levels() -> usize { 10 }
fn default_grid_spacing() -> f64 { 0.02 }
fn default_max_position() -> f64 { 0.1 }
fn default_max_drawdown() -> f64 { 0.20 }
fn default_stop_loss() -> f64 { 0.05 }
fn default_iterations() -> usize { 100 }
fn default_strategy() -> String { "random-search".to_string() }
fn default_target_metric() -> String { "sharpe".to_string() }
fn default_grid_levels_range() -> [usize; 2] { [5, 20] }
fn default_grid_spacing_range() -> [f64; 2] { [0.01, 0.05] }
fn default_lookback_days() -> usize { 365 }
fn default_transaction_fee() -> f64 { 0.0026 }
fn default_slippage() -> f64 { 0.001 }
fn default_check_interval() -> u64 { 60 }
fn default_true() -> bool { true }
fn default_log_level() -> String { "info".to_string() }
fn default_log_dir() -> String { "logs".to_string() }
fn default_db_path() -> String { "data/grid_bot.db".to_string() }
fn default_backup_interval() -> u64 { 24 }

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            db_path: default_db_path(),
            backup_interval_hours: default_backup_interval(),
        }
    }
}

impl CliConfig {
    /// Load configuration from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, CliConfigError> {
        Self::from_file_with_options(path, false)
    }

    /// Load configuration from a file with options
    pub fn from_file_with_options<P: AsRef<Path>>(path: P, skip_api_keys: bool) -> Result<Self, CliConfigError> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| CliConfigError::FileRead(e.to_string()))?;
        
        let config: Self = toml::from_str(&content)
            .map_err(|e| CliConfigError::Parse(e.to_string()))?;
        
        config.validate(skip_api_keys)?;
        Ok(config)
    }

    /// Load configuration or return error with helpful message
    pub fn load_or_error<P: AsRef<Path>>(path: P) -> Result<Self, CliConfigError> {
        let path_ref = path.as_ref();
        
        if !path_ref.exists() {
            return Err(CliConfigError::NotInitialized(
                format!(
                    "Config file not found: {}\n\
                     Run: grid-bot init\n\
                     Then edit config.toml with your API keys",
                    path_ref.display()
                )
            ));
        }

        Self::from_file(path_ref)
    }

    /// Validate configuration (optionally skip API key validation for backtesting)
    fn validate(&self, skip_api_keys: bool) -> Result<(), CliConfigError> {
        // Check API keys are not placeholders (skip for backtesting)
        if !skip_api_keys {
            if self.api.api_key.contains("YOUR_API_KEY") {
                return Err(CliConfigError::Validation(
                    "API key not configured. Edit config.toml with your Kraken API key".to_string()
                ));
            }

            if self.api.api_secret.contains("YOUR_API_SECRET") {
                return Err(CliConfigError::Validation(
                    "API secret not configured. Edit config.toml with your Kraken API secret".to_string()
                ));
            }
        }

        // Validate numeric ranges
        if self.trading.default_capital <= 0.0 {
            return Err(CliConfigError::Validation(
                "default_capital must be positive".to_string()
            ));
        }

        if self.trading.default_grid_levels == 0 {
            return Err(CliConfigError::Validation(
                "default_grid_levels must be greater than 0".to_string()
            ));
        }

        if self.trading.default_grid_spacing <= 0.0 || self.trading.default_grid_spacing >= 1.0 {
            return Err(CliConfigError::Validation(
                "default_grid_spacing must be between 0 and 1".to_string()
            ));
        }

        if self.trading.max_position_size <= 0.0 || self.trading.max_position_size > 1.0 {
            return Err(CliConfigError::Validation(
                "max_position_size must be between 0 and 1".to_string()
            ));
        }

        Ok(())
    }

    /// Check if API keys are configured
    pub fn has_valid_api_keys(&self) -> bool {
        !self.api.api_key.contains("YOUR_API_KEY") &&
        !self.api.api_secret.contains("YOUR_API_SECRET") &&
        !self.api.api_key.is_empty() &&
        !self.api.api_secret.is_empty()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CliConfigError {
    #[error("Config file not found: {0}")]
    FileNotFound(String),
    
    #[error("Failed to read config file: {0}")]
    FileRead(String),
    
    #[error("Failed to parse config: {0}")]
    Parse(String),
    
    #[error("Configuration validation error: {0}")]
    Validation(String),
    
    #[error("Configuration not initialized:\n{0}")]
    NotInitialized(String),
}
