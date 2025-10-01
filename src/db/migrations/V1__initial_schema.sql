-- Strategies table: stores trading strategy configurations
CREATE TABLE IF NOT EXISTS strategies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pair TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    grid_levels INTEGER NOT NULL,
    grid_spacing REAL NOT NULL,
    upper_price REAL NOT NULL,
    lower_price REAL NOT NULL,
    capital REAL NOT NULL,
    stop_loss_pct REAL,
    take_profit_pct REAL,
    max_position_size REAL,
    rebalance_threshold REAL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    is_active INTEGER NOT NULL DEFAULT 1
);

-- Trades table: stores individual trade executions
CREATE TABLE IF NOT EXISTS trades (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    strategy_id INTEGER NOT NULL,
    trade_type TEXT NOT NULL, -- 'BUY' or 'SELL'
    price REAL NOT NULL,
    quantity REAL NOT NULL,
    cost REAL NOT NULL,
    fee REAL NOT NULL,
    grid_level INTEGER,
    timestamp TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    order_id TEXT,
    status TEXT NOT NULL DEFAULT 'COMPLETED', -- 'PENDING', 'COMPLETED', 'FAILED'
    FOREIGN KEY (strategy_id) REFERENCES strategies(id)
);

-- Execution history: tracks strategy performance over time
CREATE TABLE IF NOT EXISTS execution_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    strategy_id INTEGER NOT NULL,
    session_start TEXT NOT NULL,
    session_end TEXT,
    total_trades INTEGER NOT NULL DEFAULT 0,
    profitable_trades INTEGER NOT NULL DEFAULT 0,
    total_profit REAL NOT NULL DEFAULT 0.0,
    total_fees REAL NOT NULL DEFAULT 0.0,
    max_drawdown REAL,
    sharpe_ratio REAL,
    win_rate REAL,
    avg_profit_per_trade REAL,
    status TEXT NOT NULL DEFAULT 'RUNNING', -- 'RUNNING', 'STOPPED', 'ERROR'
    error_message TEXT,
    FOREIGN KEY (strategy_id) REFERENCES strategies(id)
);

-- Backtest results: stores optimization and backtest outcomes
CREATE TABLE IF NOT EXISTS backtest_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    strategy_id INTEGER NOT NULL,
    start_date TEXT NOT NULL,
    end_date TEXT NOT NULL,
    initial_capital REAL NOT NULL,
    final_capital REAL NOT NULL,
    total_return REAL NOT NULL,
    sharpe_ratio REAL,
    max_drawdown REAL,
    win_rate REAL,
    total_trades INTEGER NOT NULL,
    avg_trade_duration TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (strategy_id) REFERENCES strategies(id)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_trades_strategy ON trades(strategy_id);
CREATE INDEX IF NOT EXISTS idx_trades_timestamp ON trades(timestamp);
CREATE INDEX IF NOT EXISTS idx_execution_strategy ON execution_history(strategy_id);
CREATE INDEX IF NOT EXISTS idx_backtest_strategy ON backtest_results(strategy_id);
CREATE INDEX IF NOT EXISTS idx_strategies_pair ON strategies(pair);
CREATE INDEX IF NOT EXISTS idx_strategies_active ON strategies(is_active);
