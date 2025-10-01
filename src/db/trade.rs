//! Trade database operations

use rusqlite::{params, Result as SqlResult, Row};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use rusqlite::Connection;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: Option<i64>,
    pub strategy_id: i64,
    pub trade_type: TradeType,
    pub price: f64,
    pub quantity: f64,
    pub cost: f64,
    pub fee: f64,
    pub grid_level: Option<i32>,
    pub timestamp: Option<String>,
    pub order_id: Option<String>,
    pub status: TradeStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TradeType {
    Buy,
    Sell,
}

impl TradeType {
    fn to_string(&self) -> &str {
        match self {
            TradeType::Buy => "BUY",
            TradeType::Sell => "SELL",
        }
    }

    fn from_string(s: &str) -> Self {
        match s {
            "SELL" => TradeType::Sell,
            _ => TradeType::Buy,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TradeStatus {
    Pending,
    Completed,
    Failed,
}

impl TradeStatus {
    fn to_string(&self) -> &str {
        match self {
            TradeStatus::Pending => "PENDING",
            TradeStatus::Completed => "COMPLETED",
            TradeStatus::Failed => "FAILED",
        }
    }

    fn from_string(s: &str) -> Self {
        match s {
            "PENDING" => TradeStatus::Pending,
            "FAILED" => TradeStatus::Failed,
            _ => TradeStatus::Completed,
        }
    }
}

impl Trade {
    /// Create a new trade instance
    pub fn new(
        strategy_id: i64,
        trade_type: TradeType,
        price: f64,
        quantity: f64,
        cost: f64,
        fee: f64,
    ) -> Self {
        Trade {
            id: None,
            strategy_id,
            trade_type,
            price,
            quantity,
            cost,
            fee,
            grid_level: None,
            timestamp: None,
            order_id: None,
            status: TradeStatus::Completed,
        }
    }

    /// Parse a row from the database
    fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(Trade {
            id: Some(row.get(0)?),
            strategy_id: row.get(1)?,
            trade_type: TradeType::from_string(&row.get::<_, String>(2)?),
            price: row.get(3)?,
            quantity: row.get(4)?,
            cost: row.get(5)?,
            fee: row.get(6)?,
            grid_level: row.get(7)?,
            timestamp: Some(row.get(8)?),
            order_id: row.get(9)?,
            status: TradeStatus::from_string(&row.get::<_, String>(10)?),
        })
    }

    /// Insert trade into database
    pub fn insert(&self, conn: Arc<Mutex<Connection>>) -> SqlResult<i64> {
        let conn = conn.lock().unwrap();
        conn.execute(
            "INSERT INTO trades (
                strategy_id, trade_type, price, quantity, cost, fee,
                grid_level, order_id, status
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                self.strategy_id,
                self.trade_type.to_string(),
                self.price,
                self.quantity,
                self.cost,
                self.fee,
                self.grid_level,
                self.order_id,
                self.status.to_string(),
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Update trade status
    pub fn update_status(
        conn: Arc<Mutex<Connection>>,
        id: i64,
        status: TradeStatus,
    ) -> SqlResult<usize> {
        let conn = conn.lock().unwrap();
        conn.execute(
            "UPDATE trades SET status = ?1 WHERE id = ?2",
            params![status.to_string(), id],
        )
    }

    /// Find trade by ID
    pub fn find_by_id(conn: Arc<Mutex<Connection>>, id: i64) -> SqlResult<Option<Self>> {
        let conn = conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, strategy_id, trade_type, price, quantity, cost, fee,
                    grid_level, timestamp, order_id, status
             FROM trades WHERE id = ?1"
        )?;

        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(Self::from_row(row)?)),
            None => Ok(None),
        }
    }

    /// List trades for a strategy
    pub fn list_by_strategy(conn: Arc<Mutex<Connection>>, strategy_id: i64) -> SqlResult<Vec<Self>> {
        let conn = conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, strategy_id, trade_type, price, quantity, cost, fee,
                    grid_level, timestamp, order_id, status
             FROM trades WHERE strategy_id = ?1 ORDER BY timestamp DESC"
        )?;

        let rows = stmt.query_map(params![strategy_id], |row| Self::from_row(row))?;
        rows.collect()
    }

    /// Get trade statistics for a strategy
    pub fn get_stats(conn: Arc<Mutex<Connection>>, strategy_id: i64) -> SqlResult<TradeStats> {
        let conn = conn.lock().unwrap();
        
        let mut stmt = conn.prepare(
            "SELECT 
                COUNT(*) as total_trades,
                SUM(CASE WHEN trade_type = 'BUY' THEN 1 ELSE 0 END) as buy_count,
                SUM(CASE WHEN trade_type = 'SELL' THEN 1 ELSE 0 END) as sell_count,
                SUM(cost) as total_cost,
                SUM(fee) as total_fees,
                AVG(price) as avg_price,
                MIN(price) as min_price,
                MAX(price) as max_price
             FROM trades 
             WHERE strategy_id = ?1 AND status = 'COMPLETED'"
        )?;

        let stats = stmt.query_row(params![strategy_id], |row| {
            Ok(TradeStats {
                total_trades: row.get(0)?,
                buy_count: row.get(1)?,
                sell_count: row.get(2)?,
                total_cost: row.get(3).unwrap_or(0.0),
                total_fees: row.get(4).unwrap_or(0.0),
                avg_price: row.get(5).unwrap_or(0.0),
                min_price: row.get(6).unwrap_or(0.0),
                max_price: row.get(7).unwrap_or(0.0),
            })
        })?;

        Ok(stats)
    }

    /// Delete trades for a strategy
    pub fn delete_by_strategy(conn: Arc<Mutex<Connection>>, strategy_id: i64) -> SqlResult<usize> {
        let conn = conn.lock().unwrap();
        conn.execute("DELETE FROM trades WHERE strategy_id = ?1", params![strategy_id])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeStats {
    pub total_trades: i64,
    pub buy_count: i64,
    pub sell_count: i64,
    pub total_cost: f64,
    pub total_fees: f64,
    pub avg_price: f64,
    pub min_price: f64,
    pub max_price: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Database, Strategy};

    #[test]
    fn test_trade_crud() {
        let db = Database::new_in_memory().unwrap();
        db.run_migrations().unwrap();
        let conn = db.get_connection();

        // Create a strategy first
        let strategy = Strategy::new(
            "XRPGBP".to_string(),
            "Test".to_string(),
            20, 0.01, 0.65, 0.45, 1000.0
        );
        let strategy_id = strategy.insert(Arc::clone(&conn)).unwrap();

        // Create trade
        let trade = Trade::new(strategy_id, TradeType::Buy, 0.50, 100.0, 50.0, 0.10);
        let trade_id = trade.insert(Arc::clone(&conn)).unwrap();

        // Read
        let loaded = Trade::find_by_id(Arc::clone(&conn), trade_id).unwrap().unwrap();
        assert_eq!(loaded.trade_type, TradeType::Buy);
        assert_eq!(loaded.price, 0.50);

        // List by strategy
        let trades = Trade::list_by_strategy(Arc::clone(&conn), strategy_id).unwrap();
        assert_eq!(trades.len(), 1);

        // Stats
        let stats = Trade::get_stats(Arc::clone(&conn), strategy_id).unwrap();
        assert_eq!(stats.total_trades, 1);
        assert_eq!(stats.buy_count, 1);
    }
}
