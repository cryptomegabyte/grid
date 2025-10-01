//! Pre-flight validation module for the Grid Trading Bot
//! 
//! Performs comprehensive checks before executing trading operations
//! to ensure system readiness and prevent errors.

use crate::{CliConfig, Strategy};
use tracing::{info, warn, error};
use std::time::Duration;

/// Validation result with detailed findings
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub passed: bool,
    pub checks: Vec<ValidationCheck>,
}

#[derive(Debug, Clone)]
pub struct ValidationCheck {
    pub name: String,
    pub passed: bool,
    pub message: String,
    pub level: ValidationLevel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationLevel {
    Critical,  // Must pass for operation to proceed
    Warning,   // Should pass, but operation can continue
    Info,      // Informational only
}

impl ValidationResult {
    pub fn new() -> Self {
        ValidationResult {
            passed: true,
            checks: Vec::new(),
        }
    }

    pub fn add_check(&mut self, check: ValidationCheck) {
        if !check.passed && check.level == ValidationLevel::Critical {
            self.passed = false;
        }
        self.checks.push(check);
    }

    pub fn critical_failures(&self) -> Vec<&ValidationCheck> {
        self.checks
            .iter()
            .filter(|c| !c.passed && c.level == ValidationLevel::Critical)
            .collect()
    }

    pub fn warnings(&self) -> Vec<&ValidationCheck> {
        self.checks
            .iter()
            .filter(|c| !c.passed && c.level == ValidationLevel::Warning)
            .collect()
    }

    pub fn display(&self) {
        info!("🔍 Pre-flight Validation");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        for check in &self.checks {
            let icon = if check.passed {
                "✅"
            } else {
                match check.level {
                    ValidationLevel::Critical => "❌",
                    ValidationLevel::Warning => "⚠️",
                    ValidationLevel::Info => "ℹ️",
                }
            };

            info!("{} {} - {}", icon, check.name, check.message);
        }

        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        if !self.passed {
            let failures = self.critical_failures();
            error!("❌ Validation failed: {} critical issue(s)", failures.len());
            for failure in failures {
                error!("   • {}: {}", failure.name, failure.message);
            }
        } else {
            let warnings = self.warnings();
            if !warnings.is_empty() {
                warn!("⚠️  {} warning(s) detected", warnings.len());
                for warning in warnings {
                    warn!("   • {}: {}", warning.name, warning.message);
                }
            }
            info!("✅ All critical checks passed");
        }
    }
}

/// Pre-flight validator for trading operations
pub struct PreFlightValidator {
    config: CliConfig,
}

impl PreFlightValidator {
    pub fn new(config: CliConfig) -> Self {
        PreFlightValidator { config }
    }

    /// Run full validation suite
    pub async fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Configuration checks
        result.add_check(self.check_config());
        result.add_check(self.check_api_keys());

        // Network checks
        if let Some(check) = self.check_network_connectivity().await {
            result.add_check(check);
        }

        // Database checks
        if let Some(check) = self.check_database() {
            result.add_check(check);
        }

        result
    }

    /// Validate for live trading (stricter checks)
    pub async fn validate_for_trading(&self, capital: f64) -> ValidationResult {
        let mut result = self.validate_all().await;

        // Additional trading-specific checks
        result.add_check(self.check_capital(capital));
        result.add_check(self.check_api_authentication().await);

        result
    }

    /// Validate for backtesting (relaxed checks)
    pub async fn validate_for_backtesting(&self) -> ValidationResult {
        let mut result = ValidationResult::new();

        result.add_check(self.check_config());
        
        if let Some(check) = self.check_database() {
            result.add_check(check);
        }

        result
    }

    /// Validate strategy parameters
    pub fn validate_strategy(&self, strategy: &Strategy) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Grid levels check
        if strategy.grid_levels < 2 {
            result.add_check(ValidationCheck {
                name: "Grid Levels".to_string(),
                passed: false,
                message: format!("Grid levels ({}) must be at least 2", strategy.grid_levels),
                level: ValidationLevel::Critical,
            });
        } else if strategy.grid_levels > 50 {
            result.add_check(ValidationCheck {
                name: "Grid Levels".to_string(),
                passed: false,
                message: format!("Grid levels ({}) exceeds recommended maximum (50)", strategy.grid_levels),
                level: ValidationLevel::Warning,
            });
        } else {
            result.add_check(ValidationCheck {
                name: "Grid Levels".to_string(),
                passed: true,
                message: format!("{} levels configured", strategy.grid_levels),
                level: ValidationLevel::Info,
            });
        }

        // Grid spacing check
        if strategy.grid_spacing <= 0.0 {
            result.add_check(ValidationCheck {
                name: "Grid Spacing".to_string(),
                passed: false,
                message: "Grid spacing must be positive".to_string(),
                level: ValidationLevel::Critical,
            });
        } else if strategy.grid_spacing > 0.2 {
            result.add_check(ValidationCheck {
                name: "Grid Spacing".to_string(),
                passed: false,
                message: format!("Grid spacing ({:.2}%) is very wide, may miss opportunities", strategy.grid_spacing * 100.0),
                level: ValidationLevel::Warning,
            });
        } else {
            result.add_check(ValidationCheck {
                name: "Grid Spacing".to_string(),
                passed: true,
                message: format!("{:.2}%", strategy.grid_spacing * 100.0),
                level: ValidationLevel::Info,
            });
        }

        // Price range check
        if strategy.upper_price <= strategy.lower_price {
            result.add_check(ValidationCheck {
                name: "Price Range".to_string(),
                passed: false,
                message: "Upper price must be greater than lower price".to_string(),
                level: ValidationLevel::Critical,
            });
        } else {
            let range_pct = ((strategy.upper_price - strategy.lower_price) / strategy.lower_price) * 100.0;
            result.add_check(ValidationCheck {
                name: "Price Range".to_string(),
                passed: true,
                message: format!("£{:.4} - £{:.4} ({:.1}% range)", strategy.lower_price, strategy.upper_price, range_pct),
                level: ValidationLevel::Info,
            });
        }

        // Capital check
        if strategy.capital <= 0.0 {
            result.add_check(ValidationCheck {
                name: "Capital".to_string(),
                passed: false,
                message: "Capital must be positive".to_string(),
                level: ValidationLevel::Critical,
            });
        } else if strategy.capital < 100.0 {
            result.add_check(ValidationCheck {
                name: "Capital".to_string(),
                passed: false,
                message: format!("Capital (£{:.2}) is quite low, consider minimum £100", strategy.capital),
                level: ValidationLevel::Warning,
            });
        } else {
            result.add_check(ValidationCheck {
                name: "Capital".to_string(),
                passed: true,
                message: format!("£{:.2}", strategy.capital),
                level: ValidationLevel::Info,
            });
        }

        // Position size per grid level
        let position_per_level = strategy.capital / strategy.grid_levels as f64;
        if position_per_level < 10.0 {
            result.add_check(ValidationCheck {
                name: "Position Size".to_string(),
                passed: false,
                message: format!("Position per level (£{:.2}) may be too small for efficient trading", position_per_level),
                level: ValidationLevel::Warning,
            });
        } else {
            result.add_check(ValidationCheck {
                name: "Position Size".to_string(),
                passed: true,
                message: format!("£{:.2} per level", position_per_level),
                level: ValidationLevel::Info,
            });
        }

        result
    }

    // Individual check methods

    fn check_config(&self) -> ValidationCheck {
        // Config is already loaded if we got here
        ValidationCheck {
            name: "Configuration".to_string(),
            passed: true,
            message: "Loaded successfully".to_string(),
            level: ValidationLevel::Critical,
        }
    }

    fn check_api_keys(&self) -> ValidationCheck {
        let has_keys = self.config.has_valid_api_keys();
        
        ValidationCheck {
            name: "API Keys".to_string(),
            passed: has_keys,
            message: if has_keys {
                "Configured".to_string()
            } else {
                "Not configured (required for live trading)".to_string()
            },
            level: ValidationLevel::Warning, // Warning for backtest, will be critical for live trading
        }
    }

    fn check_capital(&self, capital: f64) -> ValidationCheck {
        if capital <= 0.0 {
            ValidationCheck {
                name: "Capital".to_string(),
                passed: false,
                message: "Capital must be positive".to_string(),
                level: ValidationLevel::Critical,
            }
        } else if capital < 100.0 {
            ValidationCheck {
                name: "Capital".to_string(),
                passed: false,
                message: format!("£{:.2} is quite low, consider minimum £100", capital),
                level: ValidationLevel::Warning,
            }
        } else {
            ValidationCheck {
                name: "Capital".to_string(),
                passed: true,
                message: format!("£{:.2} allocated", capital),
                level: ValidationLevel::Info,
            }
        }
    }

    async fn check_network_connectivity(&self) -> Option<ValidationCheck> {
        // Quick connectivity check to a reliable endpoint
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .ok()?;

        match client.get("https://api.kraken.com/0/public/Time").send().await {
            Ok(_) => Some(ValidationCheck {
                name: "Network".to_string(),
                passed: true,
                message: "API reachable".to_string(),
                level: ValidationLevel::Warning,
            }),
            Err(_) => Some(ValidationCheck {
                name: "Network".to_string(),
                passed: false,
                message: "Cannot reach Kraken API".to_string(),
                level: ValidationLevel::Warning,
            }),
        }
    }

    fn check_database(&self) -> Option<ValidationCheck> {
        use crate::Database;

        let db_path = &self.config.database.db_path;
        
        if !std::path::Path::new(db_path).exists() {
            return Some(ValidationCheck {
                name: "Database".to_string(),
                passed: false,
                message: format!("Database not found at {}", db_path),
                level: ValidationLevel::Warning,
            });
        }

        match Database::new(db_path) {
            Ok(db) => match db.health_check() {
                Ok(true) => Some(ValidationCheck {
                    name: "Database".to_string(),
                    passed: true,
                    message: "Healthy".to_string(),
                    level: ValidationLevel::Info,
                }),
                _ => Some(ValidationCheck {
                    name: "Database".to_string(),
                    passed: false,
                    message: "Health check failed".to_string(),
                    level: ValidationLevel::Warning,
                }),
            },
            Err(_) => Some(ValidationCheck {
                name: "Database".to_string(),
                passed: false,
                message: "Cannot connect".to_string(),
                level: ValidationLevel::Warning,
            }),
        }
    }

    async fn check_api_authentication(&self) -> ValidationCheck {
        if !self.config.has_valid_api_keys() {
            return ValidationCheck {
                name: "API Authentication".to_string(),
                passed: false,
                message: "API keys not configured".to_string(),
                level: ValidationLevel::Critical,
            };
        }

        // For now, we'll just check if keys are present
        // Full authentication test would require actual API call
        ValidationCheck {
            name: "API Authentication".to_string(),
            passed: true,
            message: "Keys present (live auth not tested in dry-run)".to_string(),
            level: ValidationLevel::Info,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::new();
        assert!(result.passed);

        result.add_check(ValidationCheck {
            name: "Test".to_string(),
            passed: true,
            message: "OK".to_string(),
            level: ValidationLevel::Info,
        });
        assert!(result.passed);

        result.add_check(ValidationCheck {
            name: "Fail".to_string(),
            passed: false,
            message: "Failed".to_string(),
            level: ValidationLevel::Critical,
        });
        assert!(!result.passed);
    }

    #[test]
    fn test_strategy_validation() {
        use crate::Strategy;
        use crate::CliConfig;
        use crate::cli_config::*;

        // Create a minimal test config with correct field names
        let config = CliConfig {
            api: ApiConfig {
                api_key: "test_key".to_string(),
                api_secret: "test_secret".to_string(),
                rest_url: "https://api.kraken.com".to_string(),
                ws_url: "wss://ws.kraken.com".to_string(),
            },
            trading: TradingDefaults {
                default_capital: 1000.0,
                default_grid_levels: 10,
                default_grid_spacing: 0.02,
                max_position_size: 0.25,
                max_drawdown: 0.15,
                stop_loss: 0.05,
            },
            optimization: OptimizationConfig {
                default_iterations: 100,
                default_strategy: "random-search".to_string(),
                target_metric: "sharpe_ratio".to_string(),
                grid_levels_range: [3, 20],
                grid_spacing_range: [0.005, 0.05],
            },
            backtesting: BacktestingConfig {
                default_lookback_days: 30,
                transaction_fee: 0.0026,
                slippage: 0.001,
            },
            monitoring: MonitoringConfig {
                check_interval_seconds: 60,
                alert_on_error: true,
                alert_on_large_drawdown: true,
                log_level: "info".to_string(),
                log_to_file: true,
                log_directory: "logs".to_string(),
            },
            database: DatabaseConfig {
                db_path: "data/test.db".to_string(),
                backup_interval_hours: 24,
            },
        };

        let validator = PreFlightValidator::new(config);

        let mut strategy = Strategy::new(
            "XRPGBP".to_string(),
            "Test".to_string(),
            10,
            0.02,
            0.65,
            0.45,
            500.0,
        );

        let result = validator.validate_strategy(&strategy);
        assert!(result.passed);

        // Test invalid grid levels
        strategy.grid_levels = 0;
        let result = validator.validate_strategy(&strategy);
        assert!(!result.passed);

        // Test invalid price range
        strategy.grid_levels = 10;
        strategy.upper_price = 0.3;
        let result = validator.validate_strategy(&strategy);
        assert!(!result.passed);
    }
}
