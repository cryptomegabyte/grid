//! Database module for SQLite-based strategy and trade management

use rusqlite::{Connection, Result as SqlResult};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub mod strategy;
pub mod trade;
pub mod execution;
pub mod strategy_service;

pub use strategy::Strategy;
pub use trade::Trade;
pub use execution::ExecutionHistory;
pub use strategy_service::StrategyService;

/// Database manager with connection pooling
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Create a new database connection
    pub fn new<P: AsRef<Path>>(path: P) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        
        // Enable foreign keys
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        
        Ok(Database {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create an in-memory database (for testing)
    pub fn new_in_memory() -> SqlResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        
        Ok(Database {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Run migrations to set up or update the schema
    pub fn run_migrations(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        
        // Read and execute the migration SQL
        let migration_sql = include_str!("migrations/V1__initial_schema.sql");
        conn.execute_batch(migration_sql)?;
        
        Ok(())
    }

    /// Get a reference to the connection (for custom queries)
    pub fn get_connection(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }

    /// Begin a transaction
    pub fn begin_transaction(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("BEGIN TRANSACTION", [])?;
        Ok(())
    }

    /// Commit a transaction
    pub fn commit_transaction(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("COMMIT", [])?;
        Ok(())
    }

    /// Rollback a transaction
    pub fn rollback_transaction(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("ROLLBACK", [])?;
        Ok(())
    }

    /// Check database health
    pub fn health_check(&self) -> SqlResult<bool> {
        let conn = self.conn.lock().unwrap();
        let result: i32 = conn.query_row("SELECT 1", [], |row| row.get(0))?;
        Ok(result == 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() {
        let db = Database::new_in_memory().unwrap();
        assert!(db.health_check().unwrap());
    }

    #[test]
    fn test_migrations() {
        let db = Database::new_in_memory().unwrap();
        db.run_migrations().unwrap();
        
        // Verify tables were created
        let conn = db.conn.lock().unwrap();
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
            [],
            |row| row.get(0)
        ).unwrap();
        
        assert!(count >= 4); // strategies, trades, execution_history, backtest_results
    }
}
