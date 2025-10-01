//! Execution history database operations

use rusqlite::{params, Result as SqlResult, Row};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use rusqlite::Connection;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionHistory {
    pub id: Option<i64>,
    pub strategy_id: i64,
    pub session_start: String,
    pub session_end: Option<String>,
    pub total_trades: i32,
    pub profitable_trades: i32,
    pub total_profit: f64,
    pub total_fees: f64,
    pub max_drawdown: Option<f64>,
    pub sharpe_ratio: Option<f64>,
    pub win_rate: Option<f64>,
    pub avg_profit_per_trade: Option<f64>,
    pub status: ExecutionStatus,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    Running,
    Stopped,
    Error,
}

impl ExecutionStatus {
    fn to_string(&self) -> &str {
        match self {
            ExecutionStatus::Running => "RUNNING",
            ExecutionStatus::Stopped => "STOPPED",
            ExecutionStatus::Error => "ERROR",
        }
    }

    fn from_string(s: &str) -> Self {
        match s {
            "STOPPED" => ExecutionStatus::Stopped,
            "ERROR" => ExecutionStatus::Error,
            _ => ExecutionStatus::Running,
        }
    }
}

impl ExecutionHistory {
    /// Create a new execution history record
    pub fn new(strategy_id: i64, session_start: String) -> Self {
        ExecutionHistory {
            id: None,
            strategy_id,
            session_start,
            session_end: None,
            total_trades: 0,
            profitable_trades: 0,
            total_profit: 0.0,
            total_fees: 0.0,
            max_drawdown: None,
            sharpe_ratio: None,
            win_rate: None,
            avg_profit_per_trade: None,
            status: ExecutionStatus::Running,
            error_message: None,
        }
    }

    /// Parse a row from the database
    fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(ExecutionHistory {
            id: Some(row.get(0)?),
            strategy_id: row.get(1)?,
            session_start: row.get(2)?,
            session_end: row.get(3)?,
            total_trades: row.get(4)?,
            profitable_trades: row.get(5)?,
            total_profit: row.get(6)?,
            total_fees: row.get(7)?,
            max_drawdown: row.get(8)?,
            sharpe_ratio: row.get(9)?,
            win_rate: row.get(10)?,
            avg_profit_per_trade: row.get(11)?,
            status: ExecutionStatus::from_string(&row.get::<_, String>(12)?),
            error_message: row.get(13)?,
        })
    }

    /// Insert execution history into database
    pub fn insert(&self, conn: Arc<Mutex<Connection>>) -> SqlResult<i64> {
        let conn = conn.lock().unwrap();
        conn.execute(
            "INSERT INTO execution_history (
                strategy_id, session_start, session_end, total_trades,
                profitable_trades, total_profit, total_fees, max_drawdown,
                sharpe_ratio, win_rate, avg_profit_per_trade, status, error_message
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                self.strategy_id,
                self.session_start,
                self.session_end,
                self.total_trades,
                self.profitable_trades,
                self.total_profit,
                self.total_fees,
                self.max_drawdown,
                self.sharpe_ratio,
                self.win_rate,
                self.avg_profit_per_trade,
                self.status.to_string(),
                self.error_message,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Update execution history
    pub fn update(&self, conn: Arc<Mutex<Connection>>) -> SqlResult<usize> {
        let conn = conn.lock().unwrap();
        conn.execute(
            "UPDATE execution_history SET
                session_end = ?1, total_trades = ?2, profitable_trades = ?3,
                total_profit = ?4, total_fees = ?5, max_drawdown = ?6,
                sharpe_ratio = ?7, win_rate = ?8, avg_profit_per_trade = ?9,
                status = ?10, error_message = ?11
            WHERE id = ?12",
            params![
                self.session_end,
                self.total_trades,
                self.profitable_trades,
                self.total_profit,
                self.total_fees,
                self.max_drawdown,
                self.sharpe_ratio,
                self.win_rate,
                self.avg_profit_per_trade,
                self.status.to_string(),
                self.error_message,
                self.id,
            ],
        )
    }

    /// Find execution by ID
    pub fn find_by_id(conn: Arc<Mutex<Connection>>, id: i64) -> SqlResult<Option<Self>> {
        let conn = conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, strategy_id, session_start, session_end, total_trades,
                    profitable_trades, total_profit, total_fees, max_drawdown,
                    sharpe_ratio, win_rate, avg_profit_per_trade, status, error_message
             FROM execution_history WHERE id = ?1"
        )?;

        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => Ok(Some(Self::from_row(row)?)),
            None => Ok(None),
        }
    }

    /// List executions for a strategy
    pub fn list_by_strategy(
        conn: Arc<Mutex<Connection>>,
        strategy_id: i64
    ) -> SqlResult<Vec<Self>> {
        let conn = conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, strategy_id, session_start, session_end, total_trades,
                    profitable_trades, total_profit, total_fees, max_drawdown,
                    sharpe_ratio, win_rate, avg_profit_per_trade, status, error_message
             FROM execution_history WHERE strategy_id = ?1 ORDER BY session_start DESC"
        )?;

        let rows = stmt.query_map(params![strategy_id], |row| Self::from_row(row))?;
        rows.collect()
    }

    /// Get latest execution for a strategy
    pub fn get_latest(
        conn: Arc<Mutex<Connection>>,
        strategy_id: i64
    ) -> SqlResult<Option<Self>> {
        let conn = conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, strategy_id, session_start, session_end, total_trades,
                    profitable_trades, total_profit, total_fees, max_drawdown,
                    sharpe_ratio, win_rate, avg_profit_per_trade, status, error_message
             FROM execution_history 
             WHERE strategy_id = ?1 
             ORDER BY session_start DESC 
             LIMIT 1"
        )?;

        let mut rows = stmt.query(params![strategy_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(Self::from_row(row)?)),
            None => Ok(None),
        }
    }

    /// Mark execution as stopped
    pub fn mark_stopped(
        conn: Arc<Mutex<Connection>>,
        id: i64,
        session_end: String
    ) -> SqlResult<usize> {
        let conn = conn.lock().unwrap();
        conn.execute(
            "UPDATE execution_history 
             SET status = 'STOPPED', session_end = ?1 
             WHERE id = ?2",
            params![session_end, id],
        )
    }

    /// Mark execution as error
    pub fn mark_error(
        conn: Arc<Mutex<Connection>>,
        id: i64,
        error_message: String,
        session_end: String
    ) -> SqlResult<usize> {
        let conn = conn.lock().unwrap();
        conn.execute(
            "UPDATE execution_history 
             SET status = 'ERROR', error_message = ?1, session_end = ?2 
             WHERE id = ?3",
            params![error_message, session_end, id],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Database, Strategy};

    #[test]
    fn test_execution_history() {
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

        // Create execution
        let mut execution = ExecutionHistory::new(
            strategy_id,
            "2024-01-01T00:00:00Z".to_string()
        );
        let exec_id = execution.insert(Arc::clone(&conn)).unwrap();
        execution.id = Some(exec_id);

        // Read
        let loaded = ExecutionHistory::find_by_id(Arc::clone(&conn), exec_id)
            .unwrap()
            .unwrap();
        assert_eq!(loaded.status, ExecutionStatus::Running);

        // Update
        execution.total_trades = 10;
        execution.profitable_trades = 7;
        execution.total_profit = 150.0;
        execution.update(Arc::clone(&conn)).unwrap();

        let updated = ExecutionHistory::find_by_id(Arc::clone(&conn), exec_id)
            .unwrap()
            .unwrap();
        assert_eq!(updated.total_trades, 10);
        assert_eq!(updated.profitable_trades, 7);

        // Mark stopped
        ExecutionHistory::mark_stopped(
            Arc::clone(&conn),
            exec_id,
            "2024-01-01T12:00:00Z".to_string()
        ).unwrap();

        let stopped = ExecutionHistory::find_by_id(Arc::clone(&conn), exec_id)
            .unwrap()
            .unwrap();
        assert_eq!(stopped.status, ExecutionStatus::Stopped);
    }
}
