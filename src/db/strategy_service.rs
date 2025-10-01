//! Strategy service for managing strategies with database and JSON compatibility

use super::{Database, Strategy};
use std::fs;
use std::path::Path;
use serde_json::Value;
use rusqlite::Result as SqlResult;

/// Service for managing strategies with both database and legacy JSON support
pub struct StrategyService {
    db: Database,
    strategies_dir: String,
}

impl StrategyService {
    /// Create a new strategy service
    pub fn new(db: Database, strategies_dir: String) -> Self {
        StrategyService { db, strategies_dir }
    }

    /// Initialize database and optionally migrate JSON strategies
    pub fn init(&self, migrate_json: bool) -> SqlResult<()> {
        self.db.run_migrations()?;
        
        if migrate_json {
            self.migrate_json_strategies().ok(); // Don't fail if no JSON files
        }
        
        Ok(())
    }

    /// Migrate JSON strategy files to database
    pub fn migrate_json_strategies(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let mut migrated = 0;
        
        if !Path::new(&self.strategies_dir).exists() {
            return Ok(0);
        }
        
        let entries = fs::read_dir(&self.strategies_dir)?;
        let conn = self.db.get_connection();
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(json) = serde_json::from_str::<Value>(&content) {
                        // Check if strategy already exists
                        let pair = json.get("trading_pair")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        
                        // Skip if already in database
                        if let Ok(Some(_)) = Strategy::find_by_pair(conn.clone(), pair) {
                            continue;
                        }
                        
                        // Parse JSON into Strategy
                        if let Ok(strategy) = self.json_to_strategy(&json) {
                            strategy.insert(conn.clone())?;
                            migrated += 1;
                        }
                    }
                }
            }
        }
        
        Ok(migrated)
    }

    /// Convert JSON strategy file to Strategy struct
    fn json_to_strategy(&self, json: &Value) -> Result<Strategy, Box<dyn std::error::Error>> {
        let pair = json.get("trading_pair")
            .and_then(|v| v.as_str())
            .ok_or("Missing trading_pair")?
            .to_string();
        
        let name = pair.clone(); // Use pair as name if not specified
        
        let grid_levels = json.get("grid_levels")
            .and_then(|v| v.as_i64())
            .unwrap_or(10) as i32;
        
        let grid_spacing = json.get("grid_spacing")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.02);
        
        let upper_price = json.get("upper_price")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        
        let lower_price = json.get("lower_price")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);
        
        let capital = json.get("capital")
            .and_then(|v| v.as_f64())
            .unwrap_or(500.0);
        
        let mut strategy = Strategy::new(
            pair,
            name,
            grid_levels,
            grid_spacing,
            upper_price,
            lower_price,
            capital,
        );
        
        // Optional fields
        strategy.stop_loss_pct = json.get("stop_loss")
            .and_then(|v| v.as_f64());
        
        strategy.take_profit_pct = json.get("take_profit")
            .and_then(|v| v.as_f64());
        
        strategy.max_position_size = json.get("max_position_size")
            .and_then(|v| v.as_f64());
        
        Ok(strategy)
    }

    /// List all strategies from database
    pub fn list_all(&self) -> SqlResult<Vec<Strategy>> {
        let conn = self.db.get_connection();
        Strategy::list_all(conn)
    }

    /// List active strategies only
    pub fn list_active(&self) -> SqlResult<Vec<Strategy>> {
        let conn = self.db.get_connection();
        Strategy::list_active(conn)
    }

    /// Find strategy by pair
    pub fn find_by_pair(&self, pair: &str) -> SqlResult<Option<Strategy>> {
        let conn = self.db.get_connection();
        Strategy::find_by_pair(conn, pair)
    }

    /// Add or update strategy
    pub fn save_strategy(&self, strategy: &Strategy) -> SqlResult<i64> {
        let conn = self.db.get_connection();
        
        if let Some(id) = strategy.id {
            strategy.update(conn)?;
            Ok(id)
        } else {
            strategy.insert(conn)
        }
    }

    /// Delete strategy
    pub fn delete_strategy(&self, id: i64) -> SqlResult<usize> {
        let conn = self.db.get_connection();
        Strategy::delete(conn, id)
    }

    /// Deactivate strategy (soft delete)
    pub fn deactivate_strategy(&self, id: i64) -> SqlResult<usize> {
        let conn = self.db.get_connection();
        Strategy::deactivate(conn, id)
    }

    /// Count total strategies
    pub fn count(&self) -> SqlResult<i64> {
        let conn = self.db.get_connection();
        Strategy::count(conn)
    }

    /// Export strategy to JSON file (for backwards compatibility)
    pub fn export_to_json(&self, strategy: &Strategy, output_dir: &str) -> Result<String, Box<dyn std::error::Error>> {
        fs::create_dir_all(output_dir)?;
        
        let filename = format!("{}/{}_strategy.json", output_dir, strategy.pair.to_lowercase());
        
        let json = serde_json::json!({
            "trading_pair": strategy.pair,
            "name": strategy.name,
            "grid_levels": strategy.grid_levels,
            "grid_spacing": strategy.grid_spacing,
            "upper_price": strategy.upper_price,
            "lower_price": strategy.lower_price,
            "capital": strategy.capital,
            "stop_loss": strategy.stop_loss_pct,
            "take_profit": strategy.take_profit_pct,
            "max_position_size": strategy.max_position_size,
            "rebalance_threshold": strategy.rebalance_threshold,
            "is_active": strategy.is_active,
        });
        
        let content = serde_json::to_string_pretty(&json)?;
        fs::write(&filename, content)?;
        
        Ok(filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_service() {
        let db = Database::new_in_memory().unwrap();
        let service = StrategyService::new(db, "strategies".to_string());
        
        service.init(false).unwrap();
        
        let strategy = Strategy::new(
            "XRPGBP".to_string(),
            "XRP Test".to_string(),
            20, 0.01, 0.65, 0.45, 1000.0
        );
        
        let id = service.save_strategy(&strategy).unwrap();
        assert!(id > 0);
        
        let loaded = service.find_by_pair("XRPGBP").unwrap().unwrap();
        assert_eq!(loaded.name, "XRP Test");
        
        let count = service.count().unwrap();
        assert_eq!(count, 1);
    }
}
