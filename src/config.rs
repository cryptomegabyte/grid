// Configuration management for the grid trading bot

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    pub kraken_ws_url: String,
    pub trading_pair: String,
    pub grid_levels: usize,
    pub grid_spacing: f64,
    pub min_price_change: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketConfig {
    pub trend_threshold: f64,        // Percentage change to detect trends
    pub volatility_threshold: f64,   // Volatility threshold for state detection
    pub price_history_size: usize,   // Number of prices to keep for analysis
}

impl Default for MarketConfig {
    fn default() -> Self {
        Self {
            trend_threshold: 0.005,      // 0.5%
            volatility_threshold: 0.02,  // 2%
            price_history_size: 50,      // INCREASED: 50 bars for better trend detection (was 10)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub enable_price_logging: bool,
    pub enable_signal_logging: bool,
    pub enable_state_change_logging: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub trading: TradingConfig,
    pub market: MarketConfig,
    pub logging: LoggingConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            trading: TradingConfig {
                kraken_ws_url: "wss://ws.kraken.com".to_string(),
                trading_pair: "XRP/GBP".to_string(),
                grid_levels: 5,
                grid_spacing: 0.01,
                min_price_change: 0.001,
            },
            market: MarketConfig {
                trend_threshold: 0.005,      // 0.5%
                volatility_threshold: 0.02,  // 2%
                price_history_size: 10,
            },
            logging: LoggingConfig {
                enable_price_logging: true,
                enable_signal_logging: true,
                enable_state_change_logging: true,
            },
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)
            .map_err(|e| ConfigError::FileRead(e.to_string()))?;
        
        let config: Config = toml::from_str(&content)
            .map_err(|e| ConfigError::Parse(e.to_string()))?;
        
        config.validate()?;
        Ok(config)
    }

    /// Save configuration to a TOML file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::Serialize(e.to_string()))?;
        
        fs::write(path, content)
            .map_err(|e| ConfigError::FileWrite(e.to_string()))?;
        
        Ok(())
    }

    /// Load configuration from file, or create default if file doesn't exist
    pub fn load_or_create<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        if path.as_ref().exists() {
            Self::from_file(path)
        } else {
            let config = Self::default();
            config.to_file(&path)?;
            println!("📁 Created default config file: {}", path.as_ref().display());
            Ok(config)
        }
    }

    /// Validate configuration values
    fn validate(&self) -> Result<(), ConfigError> {
        if self.trading.grid_levels == 0 {
            return Err(ConfigError::Validation("grid_levels must be greater than 0".to_string()));
        }
        
        if self.trading.grid_spacing <= 0.0 {
            return Err(ConfigError::Validation("grid_spacing must be positive".to_string()));
        }
        
        if self.trading.min_price_change < 0.0 {
            return Err(ConfigError::Validation("min_price_change must be non-negative".to_string()));
        }
        
        if self.market.trend_threshold <= 0.0 {
            return Err(ConfigError::Validation("trend_threshold must be positive".to_string()));
        }
        
        if self.market.volatility_threshold <= 0.0 {
            return Err(ConfigError::Validation("volatility_threshold must be positive".to_string()));
        }
        
        if self.market.price_history_size == 0 {
            return Err(ConfigError::Validation("price_history_size must be greater than 0".to_string()));
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    FileRead(String),
    
    #[error("Failed to write config file: {0}")]
    FileWrite(String),
    
    #[error("Failed to parse config: {0}")]
    Parse(String),
    
    #[error("Failed to serialize config: {0}")]
    Serialize(String),
    
    #[error("Configuration validation error: {0}")]
    Validation(String),
}