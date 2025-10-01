//! Strategy database operations

use rusqlite::{params, Result as SqlResult, Row};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use rusqlite::Connection;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    pub id: Option<i64>,
    pub pair: String,
    pub name: String,
    pub grid_levels: i32,
    pub grid_spacing: f64,
    pub upper_price: f64,
    pub lower_price: f64,
    pub capital: f64,
    pub stop_loss_pct: Option<f64>,
    pub take_profit_pct: Option<f64>,
    pub max_position_size: Option<f64>,
    pub rebalance_threshold: Option<f64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub is_active: bool,
}

impl Strategy {
    /// Create a new strategy instance
    pub fn new(
        pair: String,
        name: String,
        grid_levels: i32,
        grid_spacing: f64,
        upper_price: f64,
        lower_price: f64,
        capital: f64,
    ) -> Self {
        Strategy {
            id: None,
            pair,
            name,
            grid_levels,
            grid_spacing,
            upper_price,
            lower_price,
            capital,
            stop_loss_pct: None,
            take_profit_pct: None,
            max_position_size: None,
            rebalance_threshold: None,
            created_at: None,
            updated_at: None,
            is_active: true,
        }
    }

    /// Parse a row from the database
    fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(Strategy {
            id: Some(row.get(0)?),
            pair: row.get(1)?,
            name: row.get(2)?,
            grid_levels: row.get(3)?,
            grid_spacing: row.get(4)?,
            upper_price: row.get(5)?,
            lower_price: row.get(6)?,
            capital: row.get(7)?,
            stop_loss_pct: row.get(8)?,
            take_profit_pct: row.get(9)?,
            max_position_size: row.get(10)?,
            rebalance_threshold: row.get(11)?,
            created_at: Some(row.get(12)?),
            updated_at: Some(row.get(13)?),
            is_active: row.get::<_, i32>(14)? == 1,
        })
    }

    /// Insert strategy into database
    pub fn insert(&self, conn: Arc<Mutex<Connection>>) -> SqlResult<i64> {
        let conn = conn.lock().unwrap();
        conn.execute(
            "INSERT INTO strategies (
                pair, name, grid_levels, grid_spacing, upper_price, lower_price,
                capital, stop_loss_pct, take_profit_pct, max_position_size,
                rebalance_threshold, is_active
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                self.pair,
                self.name,
                self.grid_levels,
                self.grid_spacing,
                self.upper_price,
                self.lower_price,
                self.capital,
                self.stop_loss_pct,
                self.take_profit_pct,
                self.max_position_size,
                self.rebalance_threshold,
                if self.is_active { 1 } else { 0 },
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Update strategy in database
    pub fn update(&self, conn: Arc<Mutex<Connection>>) -> SqlResult<usize> {
        let conn = conn.lock().unwrap();
        conn.execute(
            "UPDATE strategies SET
                name = ?1, grid_levels = ?2, grid_spacing = ?3, upper_price = ?4,
                lower_price = ?5, capital = ?6, stop_loss_pct = ?7,
                take_profit_pct = ?8, max_position_size = ?9,
                rebalance_threshold = ?10, is_active = ?11,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?12",
            params![
                self.name,
                self.grid_levels,
                self.grid_spacing,
                self.upper_price,
                self.lower_price,
                self.capital,
                self.stop_loss_pct,
                self.take_profit_pct,
                self.max_position_size,
                self.rebalance_threshold,
                if self.is_active { 1 } else { 0 },
                self.id,
            ],
        )
    }

    /// Find strategy by ID
    pub fn find_by_id(conn: Arc<Mutex<Connection>>, id: i64) -> SqlResult<Option<Self>> {
        let conn = conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, pair, name, grid_levels, grid_spacing, upper_price,
                    lower_price, capital, stop_loss_pct, take_profit_pct,
                    max_position_size, rebalance_threshold, created_at,
                    updated_at, is_active
             FROM strategies WHERE id = ?1"
        )?;

        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(Self::from_row(row)?)),
            None => Ok(None),
        }
    }

    /// Find strategy by pair
    pub fn find_by_pair(conn: Arc<Mutex<Connection>>, pair: &str) -> SqlResult<Option<Self>> {
        let conn = conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, pair, name, grid_levels, grid_spacing, upper_price,
                    lower_price, capital, stop_loss_pct, take_profit_pct,
                    max_position_size, rebalance_threshold, created_at,
                    updated_at, is_active
             FROM strategies WHERE pair = ?1"
        )?;

        let mut rows = stmt.query(params![pair])?;
        match rows.next()? {
            Some(row) => Ok(Some(Self::from_row(row)?)),
            None => Ok(None),
        }
    }

    /// List all strategies
    pub fn list_all(conn: Arc<Mutex<Connection>>) -> SqlResult<Vec<Self>> {
        let conn = conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, pair, name, grid_levels, grid_spacing, upper_price,
                    lower_price, capital, stop_loss_pct, take_profit_pct,
                    max_position_size, rebalance_threshold, created_at,
                    updated_at, is_active
             FROM strategies ORDER BY pair"
        )?;

        let rows = stmt.query_map([], |row| Self::from_row(row))?;
        rows.collect()
    }

    /// List active strategies only
    pub fn list_active(conn: Arc<Mutex<Connection>>) -> SqlResult<Vec<Self>> {
        let conn = conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, pair, name, grid_levels, grid_spacing, upper_price,
                    lower_price, capital, stop_loss_pct, take_profit_pct,
                    max_position_size, rebalance_threshold, created_at,
                    updated_at, is_active
             FROM strategies WHERE is_active = 1 ORDER BY pair"
        )?;

        let rows = stmt.query_map([], |row| Self::from_row(row))?;
        rows.collect()
    }

    /// Delete strategy by ID
    pub fn delete(conn: Arc<Mutex<Connection>>, id: i64) -> SqlResult<usize> {
        let conn = conn.lock().unwrap();
        conn.execute("DELETE FROM strategies WHERE id = ?1", params![id])
    }

    /// Deactivate strategy (soft delete)
    pub fn deactivate(conn: Arc<Mutex<Connection>>, id: i64) -> SqlResult<usize> {
        let conn = conn.lock().unwrap();
        conn.execute(
            "UPDATE strategies SET is_active = 0, updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            params![id]
        )
    }

    /// Count total strategies
    pub fn count(conn: Arc<Mutex<Connection>>) -> SqlResult<i64> {
        let conn = conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM strategies", [], |row| row.get(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_strategy_crud() {
        let db = Database::new_in_memory().unwrap();
        db.run_migrations().unwrap();
        let conn = db.get_connection();

        // Create
        let mut strategy = Strategy::new(
            "XRPGBP".to_string(),
            "XRP Grid Strategy".to_string(),
            20,
            0.01,
            0.65,
            0.45,
            1000.0,
        );
        let id = strategy.insert(Arc::clone(&conn)).unwrap();
        strategy.id = Some(id);

        // Read
        let loaded = Strategy::find_by_id(Arc::clone(&conn), id).unwrap().unwrap();
        assert_eq!(loaded.pair, "XRPGBP");
        assert_eq!(loaded.grid_levels, 20);

        // Update
        let mut updated = loaded.clone();
        updated.grid_levels = 25;
        updated.update(Arc::clone(&conn)).unwrap();

        let reloaded = Strategy::find_by_id(Arc::clone(&conn), id).unwrap().unwrap();
        assert_eq!(reloaded.grid_levels, 25);

        // List
        let all = Strategy::list_all(Arc::clone(&conn)).unwrap();
        assert_eq!(all.len(), 1);

        // Delete
        Strategy::delete(Arc::clone(&conn), id).unwrap();
        let deleted = Strategy::find_by_id(Arc::clone(&conn), id).unwrap();
        assert!(deleted.is_none());
    }
}
