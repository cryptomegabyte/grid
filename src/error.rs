//! Comprehensive error handling for the Grid Trading Bot
//! 
//! This module provides a unified error type that replaces Box<dyn Error>
//! throughout the application with context-rich, actionable error messages.

use std::fmt;
use std::io;

/// Main error type for the grid trading bot
#[derive(Debug)]
pub enum TradingError {
    // Configuration errors
    ConfigNotFound(String),
    ConfigParse(String),
    ConfigValidation(String),
    ConfigMissing(String),
    
    // Database errors
    DatabaseConnection(String),
    DatabaseQuery(String),
    DatabaseMigration(String),
    DatabaseConstraint(String),
    
    // API errors
    ApiConnection(String),
    ApiAuthentication(String),
    ApiRateLimit(String),
    ApiResponse(String),
    ApiTimeout(String),
    
    // Validation errors
    ValidationFailed(String),
    InvalidParameter(String, String), // (parameter_name, reason)
    InvalidStrategy(String),
    InsufficientFunds(f64, f64), // (required, available)
    
    // Strategy errors
    StrategyNotFound(String),
    StrategyLoadFailed(String),
    StrategyParseFailed(String),
    
    // Trading errors
    OrderFailed(String),
    OrderRejected(String),
    InsufficientLiquidity(String),
    MarketClosed(String),
    
    // IO errors
    FileNotFound(String),
    FileRead(String),
    FileWrite(String),
    DirectoryCreate(String),
    
    // Network errors
    NetworkUnavailable(String),
    ConnectionTimeout(String),
    
    // General errors
    Internal(String),
    NotImplemented(String),
}

impl TradingError {
    /// Get a user-friendly error message with helpful context
    pub fn user_message(&self) -> String {
        match self {
            TradingError::ConfigNotFound(path) => {
                format!(
                    "Configuration file not found: {}\n\n\
                    �� Quick fix:\n\
                    1. Run: grid-bot init\n\
                    2. Edit config.toml with your API keys\n\
                    3. Try again",
                    path
                )
            }
            TradingError::ConfigValidation(msg) => {
                format!(
                    "Configuration validation error: {}\n\n\
                    💡 Check config.toml for:\n\
                    - Valid API keys (not placeholders)\n\
                    - Positive numeric values\n\
                    - Proper range values",
                    msg
                )
            }
            TradingError::DatabaseConnection(msg) => {
                format!(
                    "Database connection failed: {}\n\n\
                    💡 Try:\n\
                    1. Run: grid-bot init\n\
                    2. Check data/ directory permissions\n\
                    3. Ensure disk space available",
                    msg
                )
            }
            TradingError::ApiAuthentication(msg) => {
                format!(
                    "API authentication failed: {}\n\n\
                    💡 Check:\n\
                    - API key is correct\n\
                    - API secret is correct\n\
                    - Keys have trading permissions\n\
                    - API endpoint is correct",
                    msg
                )
            }
            TradingError::InsufficientFunds(required, available) => {
                format!(
                    "Insufficient funds for operation\n\
                    Required: £{:.2}\n\
                    Available: £{:.2}\n\n\
                    💡 Either:\n\
                    - Increase your account balance\n\
                    - Reduce position size with --capital flag",
                    required, available
                )
            }
            TradingError::StrategyNotFound(pair) => {
                format!(
                    "Strategy not found for pair: {}\n\n\
                    💡 Options:\n\
                    1. Run: grid-bot optimize pair {}\n\
                    2. Check: grid-bot strategy list\n\
                    3. Import existing strategy",
                    pair, pair
                )
            }
            TradingError::ApiRateLimit(msg) => {
                format!(
                    "API rate limit exceeded: {}\n\n\
                    💡 Please wait before retrying\n\
                    Rate limits typically reset within 1-5 minutes",
                    msg
                )
            }
            _ => self.to_string(),
        }
    }
    
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            TradingError::ApiTimeout(_)
                | TradingError::ConnectionTimeout(_)
                | TradingError::NetworkUnavailable(_)
                | TradingError::ApiRateLimit(_)
        )
    }
    
    /// Get error category for logging/metrics
    pub fn category(&self) -> &'static str {
        match self {
            TradingError::ConfigNotFound(_)
            | TradingError::ConfigParse(_)
            | TradingError::ConfigValidation(_)
            | TradingError::ConfigMissing(_) => "config",
            
            TradingError::DatabaseConnection(_)
            | TradingError::DatabaseQuery(_)
            | TradingError::DatabaseMigration(_)
            | TradingError::DatabaseConstraint(_) => "database",
            
            TradingError::ApiConnection(_)
            | TradingError::ApiAuthentication(_)
            | TradingError::ApiRateLimit(_)
            | TradingError::ApiResponse(_)
            | TradingError::ApiTimeout(_) => "api",
            
            TradingError::ValidationFailed(_)
            | TradingError::InvalidParameter(_, _)
            | TradingError::InvalidStrategy(_)
            | TradingError::InsufficientFunds(_, _) => "validation",
            
            TradingError::StrategyNotFound(_)
            | TradingError::StrategyLoadFailed(_)
            | TradingError::StrategyParseFailed(_) => "strategy",
            
            TradingError::OrderFailed(_)
            | TradingError::OrderRejected(_)
            | TradingError::InsufficientLiquidity(_)
            | TradingError::MarketClosed(_) => "trading",
            
            TradingError::FileNotFound(_)
            | TradingError::FileRead(_)
            | TradingError::FileWrite(_)
            | TradingError::DirectoryCreate(_) => "io",
            
            TradingError::NetworkUnavailable(_)
            | TradingError::ConnectionTimeout(_) => "network",
            
            TradingError::Internal(_) | TradingError::NotImplemented(_) => "internal",
        }
    }
}

impl fmt::Display for TradingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TradingError::ConfigNotFound(path) => {
                write!(f, "Configuration file not found: {}", path)
            }
            TradingError::ConfigParse(msg) => {
                write!(f, "Configuration parse error: {}", msg)
            }
            TradingError::ConfigValidation(msg) => {
                write!(f, "Configuration validation error: {}", msg)
            }
            TradingError::ConfigMissing(field) => {
                write!(f, "Missing required configuration: {}", field)
            }
            
            TradingError::DatabaseConnection(msg) => {
                write!(f, "Database connection error: {}", msg)
            }
            TradingError::DatabaseQuery(msg) => {
                write!(f, "Database query error: {}", msg)
            }
            TradingError::DatabaseMigration(msg) => {
                write!(f, "Database migration error: {}", msg)
            }
            TradingError::DatabaseConstraint(msg) => {
                write!(f, "Database constraint violation: {}", msg)
            }
            
            TradingError::ApiConnection(msg) => {
                write!(f, "API connection error: {}", msg)
            }
            TradingError::ApiAuthentication(msg) => {
                write!(f, "API authentication failed: {}", msg)
            }
            TradingError::ApiRateLimit(msg) => {
                write!(f, "API rate limit exceeded: {}", msg)
            }
            TradingError::ApiResponse(msg) => {
                write!(f, "API response error: {}", msg)
            }
            TradingError::ApiTimeout(msg) => {
                write!(f, "API timeout: {}", msg)
            }
            
            TradingError::ValidationFailed(msg) => {
                write!(f, "Validation failed: {}", msg)
            }
            TradingError::InvalidParameter(param, reason) => {
                write!(f, "Invalid parameter '{}': {}", param, reason)
            }
            TradingError::InvalidStrategy(msg) => {
                write!(f, "Invalid strategy: {}", msg)
            }
            TradingError::InsufficientFunds(required, available) => {
                write!(
                    f,
                    "Insufficient funds: required £{:.2}, available £{:.2}",
                    required, available
                )
            }
            
            TradingError::StrategyNotFound(pair) => {
                write!(f, "Strategy not found for pair: {}", pair)
            }
            TradingError::StrategyLoadFailed(msg) => {
                write!(f, "Failed to load strategy: {}", msg)
            }
            TradingError::StrategyParseFailed(msg) => {
                write!(f, "Failed to parse strategy: {}", msg)
            }
            
            TradingError::OrderFailed(msg) => {
                write!(f, "Order failed: {}", msg)
            }
            TradingError::OrderRejected(msg) => {
                write!(f, "Order rejected: {}", msg)
            }
            TradingError::InsufficientLiquidity(msg) => {
                write!(f, "Insufficient liquidity: {}", msg)
            }
            TradingError::MarketClosed(msg) => {
                write!(f, "Market closed: {}", msg)
            }
            
            TradingError::FileNotFound(path) => {
                write!(f, "File not found: {}", path)
            }
            TradingError::FileRead(msg) => {
                write!(f, "File read error: {}", msg)
            }
            TradingError::FileWrite(msg) => {
                write!(f, "File write error: {}", msg)
            }
            TradingError::DirectoryCreate(msg) => {
                write!(f, "Directory creation error: {}", msg)
            }
            
            TradingError::NetworkUnavailable(msg) => {
                write!(f, "Network unavailable: {}", msg)
            }
            TradingError::ConnectionTimeout(msg) => {
                write!(f, "Connection timeout: {}", msg)
            }
            
            TradingError::Internal(msg) => {
                write!(f, "Internal error: {}", msg)
            }
            TradingError::NotImplemented(msg) => {
                write!(f, "Not implemented: {}", msg)
            }
        }
    }
}

impl std::error::Error for TradingError {}

// Conversion implementations for common error types

impl From<io::Error> for TradingError {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => TradingError::FileNotFound(err.to_string()),
            io::ErrorKind::PermissionDenied => TradingError::FileRead(err.to_string()),
            io::ErrorKind::TimedOut => TradingError::ConnectionTimeout(err.to_string()),
            io::ErrorKind::ConnectionRefused => TradingError::NetworkUnavailable(err.to_string()),
            _ => TradingError::Internal(format!("IO error: {}", err)),
        }
    }
}

impl From<rusqlite::Error> for TradingError {
    fn from(err: rusqlite::Error) -> Self {
        match err {
            rusqlite::Error::SqliteFailure(_, Some(msg)) => {
                if msg.contains("UNIQUE constraint") {
                    TradingError::DatabaseConstraint(msg)
                } else if msg.contains("FOREIGN KEY constraint") {
                    TradingError::DatabaseConstraint(msg)
                } else {
                    TradingError::DatabaseQuery(msg)
                }
            }
            rusqlite::Error::QueryReturnedNoRows => {
                TradingError::DatabaseQuery("Query returned no rows".to_string())
            }
            _ => TradingError::DatabaseQuery(err.to_string()),
        }
    }
}

impl From<serde_json::Error> for TradingError {
    fn from(err: serde_json::Error) -> Self {
        TradingError::StrategyParseFailed(format!("JSON parse error: {}", err))
    }
}

impl From<toml::de::Error> for TradingError {
    fn from(err: toml::de::Error) -> Self {
        TradingError::ConfigParse(format!("TOML parse error: {}", err))
    }
}

impl From<reqwest::Error> for TradingError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            TradingError::ApiTimeout(err.to_string())
        } else if err.is_connect() {
            TradingError::ApiConnection(err.to_string())
        } else if err.is_status() {
            TradingError::ApiResponse(err.to_string())
        } else {
            TradingError::ApiConnection(err.to_string())
        }
    }
}

impl From<crate::cli_config::CliConfigError> for TradingError {
    fn from(err: crate::cli_config::CliConfigError) -> Self {
        use crate::cli_config::CliConfigError;
        match err {
            CliConfigError::FileNotFound(path) => TradingError::ConfigNotFound(path),
            CliConfigError::FileRead(msg) => TradingError::FileRead(msg),
            CliConfigError::Parse(msg) => TradingError::ConfigParse(msg),
            CliConfigError::Validation(msg) => TradingError::ConfigValidation(msg),
            CliConfigError::NotInitialized(msg) => TradingError::ConfigNotFound(msg),
        }
    }
}

impl From<String> for TradingError {
    fn from(msg: String) -> Self {
        TradingError::Internal(msg)
    }
}

impl From<&str> for TradingError {
    fn from(msg: &str) -> Self {
        TradingError::Internal(msg.to_string())
    }
}

impl From<std::time::SystemTimeError> for TradingError {
    fn from(err: std::time::SystemTimeError) -> Self {
        TradingError::Internal(format!("System time error: {}", err))
    }
}

/// Result type alias using TradingError
pub type TradingResult<T> = Result<T, TradingError>;

/// Helper macro for creating context-rich errors
#[macro_export]
macro_rules! trading_error {
    (config_not_found, $path:expr) => {
        TradingError::ConfigNotFound($path.to_string())
    };
    (invalid_param, $param:expr, $reason:expr) => {
        TradingError::InvalidParameter($param.to_string(), $reason.to_string())
    };
    (insufficient_funds, $required:expr, $available:expr) => {
        TradingError::InsufficientFunds($required, $available)
    };
    (strategy_not_found, $pair:expr) => {
        TradingError::StrategyNotFound($pair.to_string())
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = TradingError::ConfigNotFound("config.toml".to_string());
        assert!(err.to_string().contains("config.toml"));
    }

    #[test]
    fn test_error_category() {
        let err = TradingError::ConfigValidation("test".to_string());
        assert_eq!(err.category(), "config");
        
        let err = TradingError::DatabaseQuery("test".to_string());
        assert_eq!(err.category(), "database");
        
        let err = TradingError::ApiTimeout("test".to_string());
        assert_eq!(err.category(), "api");
    }

    #[test]
    fn test_retryable() {
        let err = TradingError::ApiTimeout("test".to_string());
        assert!(err.is_retryable());
        
        let err = TradingError::ConfigNotFound("test".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_user_message() {
        let err = TradingError::InsufficientFunds(100.0, 50.0);
        let msg = err.user_message();
        assert!(msg.contains("100.00"));
        assert!(msg.contains("50.00"));
        assert!(msg.contains("💡"));
    }

    #[test]
    fn test_io_conversion() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "test");
        let trading_err: TradingError = io_err.into();
        assert!(matches!(trading_err, TradingError::FileNotFound(_)));
    }
}
