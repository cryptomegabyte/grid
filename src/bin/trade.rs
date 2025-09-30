// Professional Live Grid Trading Engine
// Reads optimized strategy configurations and executes live trades

use clap::{Parser, Subcommand};
use grid_trading_bot::{GridTrader, MarketState};
use grid_trading_bot::websocket_client::WebSocketClient;
use serde::{Deserialize, Serialize};
use std::fs;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error, debug};

#[derive(Parser)]
#[command(name = "trade")]
#[command(about = "Professional Live Grid Trading Engine")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start live trading with all profitable strategies
    Start {
        /// Dry run mode (no real trades)
        #[arg(short, long)]
        dry_run: bool,
        
        /// Maximum total capital allocation across all pairs
        #[arg(short, long, default_value = "10000")]
        max_capital: f64,
    },
    /// Trade a specific pair with its optimized strategy
    Pair {
        /// Trading pair
        pair: String,
        
        /// Dry run mode
        #[arg(short, long)]
        dry_run: bool,
        
        /// Capital allocation for this pair
        #[arg(short, long, default_value = "5000")]
        capital: f64,
    },
    /// List available strategies
    List,
    /// Monitor running trades
    Monitor,
}

#[derive(Serialize, Deserialize, Clone)]
struct OptimizedStrategy {
    trading_pair: String,
    timeframe: String,
    grid_levels: usize,
    grid_spacing: f64,
    initial_capital: f64,
    use_markov_predictions: bool,
    markov_lookback_periods: usize,
    
    expected_annual_return: f64,
    sharpe_ratio: f64,
    max_drawdown: f64,
    win_rate: f64,
    total_trades_tested: usize,
    
    max_position_size_pct: f64,
    max_daily_loss_pct: f64,
    
    generated_at: chrono::DateTime<chrono::Utc>,
    backtest_period_days: i64,
    data_points_analyzed: usize,
    confidence_score: f64,
}

#[derive(Serialize, Deserialize)]
struct TradingSession {
    session_id: String,
    started_at: chrono::DateTime<chrono::Utc>,
    strategies: HashMap<String, OptimizedStrategy>,
    dry_run: bool,
    total_capital_allocated: f64,
    active_trades: HashMap<String, Vec<LiveTrade>>,
}

#[derive(Serialize, Deserialize, Clone)]
struct LiveTrade {
    trade_id: String,
    pair: String,
    side: String, // "buy" or "sell"  
    price: f64,
    quantity: f64,
    timestamp: chrono::DateTime<chrono::Utc>,
    grid_level: f64,
    status: String, // "pending", "filled", "cancelled"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Start { dry_run, max_capital } => {
            info!("🚀 Starting live trading engine");
            start_live_trading(dry_run, max_capital).await?;
        }
        Commands::Pair { pair, dry_run, capital } => {
            info!("🎯 Trading specific pair: {}", pair);
            trade_single_pair(&pair, dry_run, capital).await?;
        }
        Commands::List => {
            list_strategies().await?;
        }
        Commands::Monitor => {
            monitor_trades().await?;
        }
    }
    
    Ok(())
}

async fn start_live_trading(dry_run: bool, max_capital: f64) -> Result<(), Box<dyn std::error::Error>> {
    info!("📊 Loading optimized strategies...");
    
    let strategies = load_all_strategies().await?;
    
    if strategies.is_empty() {
        error!("❌ No strategies found! Run 'cargo run --bin backtest auto' first");
        return Ok(());
    }
    
    info!("✅ Loaded {} profitable strategies", strategies.len());
    
    // Calculate capital allocation per strategy
    let capital_per_strategy = max_capital / strategies.len() as f64;
    
    // Create trading session
    let session = TradingSession {
        session_id: uuid::Uuid::new_v4().to_string(),
        started_at: chrono::Utc::now(),
        strategies: strategies.clone(),
        dry_run,
        total_capital_allocated: max_capital,
        active_trades: HashMap::new(),
    };
    
    // Save session for monitoring
    save_trading_session(&session)?;
    
    if dry_run {
        info!("🧪 DRY RUN MODE - No real trades will be executed");
    } else {
        warn!("💰 LIVE TRADING MODE - Real money will be used!");
        warn!("⚠️  Press Ctrl+C within 10 seconds to cancel...");
        sleep(Duration::from_secs(10)).await;
    }
    
    info!("🔄 Starting trading loops for {} pairs", strategies.len());
    
    // Start trading loops for each strategy
    let mut handles = Vec::new();
    
    for (pair, strategy) in strategies {
        let strategy_clone = strategy.clone();
        let dry_run_clone = dry_run;
        let capital = capital_per_strategy;
        
        let handle = tokio::spawn(async move {
            if let Err(e) = run_strategy_loop(&pair, strategy_clone, dry_run_clone, capital).await {
                error!("❌ Strategy loop failed for {}: {}", pair, e);
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all strategies to complete (they shouldn't unless there's an error)
    for handle in handles {
        handle.await?;
    }
    
    Ok(())
}

async fn trade_single_pair(pair: &str, dry_run: bool, capital: f64) -> Result<(), Box<dyn std::error::Error>> {
    let strategy = load_strategy(pair).await?;
    
    info!("📈 Strategy loaded for {}", pair);
    info!("   Expected Annual Return: {:.2}%", strategy.expected_annual_return);
    info!("   Sharpe Ratio: {:.2}", strategy.sharpe_ratio);
    info!("   Confidence Score: {:.1}%", strategy.confidence_score);
    
    if dry_run {
        info!("🧪 DRY RUN MODE");
    } else {
        warn!("💰 LIVE TRADING MODE");
    }
    
    run_strategy_loop(pair, strategy, dry_run, capital).await?;
    
    Ok(())
}

async fn run_strategy_loop(
    pair: &str, 
    strategy: OptimizedStrategy, 
    dry_run: bool, 
    capital: f64
) -> Result<(), Box<dyn std::error::Error>> {
    info!("🎯 Starting trading loop for {} with £{:.2}", pair, capital);
    
    // Initialize WebSocket connection
    let mut ws_client = WebSocketClient::new(&format!("wss://ws.kraken.com/"));
    ws_client.connect().await?;
    
    // Subscribe to price updates for this pair
    ws_client.subscribe_to_ticker(pair).await?;
    
    // Initialize grid trader with optimized parameters
    let mut trader = GridTrader::new(
        strategy.grid_levels,
        strategy.grid_spacing,
        capital,
    );
    
    info!("✅ {} trading loop initialized", pair);
    info!("   Grid Levels: {}", strategy.grid_levels);
    info!("   Grid Spacing: {:.2}%", strategy.grid_spacing * 100.0);
    info!("   Capital: £{:.2}", capital);
    
    // Main trading loop
    loop {
        match ws_client.receive_message().await {
            Ok(price_update) => {
                debug!("📊 {} price update: £{:.6}", pair, price_update.price);
                
                // Update market state (use your existing logic)
                let market_state = detect_market_state(&price_update);
                
                // Update trader state
                trader.update_price(price_update.price, market_state);
                
                // Check for trading signals
                if let Some(signal) = trader.check_signals() {
                    info!("🚨 {} signal: {:?} at £{:.6}", pair, signal.signal_type, signal.price);
                    
                    if dry_run {
                        info!("🧪 DRY RUN: Would execute {} trade", signal.signal_type);
                        log_simulated_trade(pair, &signal).await?;
                    } else {
                        info!("💰 LIVE: Executing {} trade", signal.signal_type);
                        execute_live_trade(pair, &signal, &mut ws_client).await?;
                    }
                }
                
                // Risk management checks
                if should_stop_trading(&trader, &strategy) {
                    warn!("🛑 Risk limits reached for {}. Stopping trading.", pair);
                    break;
                }
                
            }
            Err(e) => {
                error!("📡 WebSocket error for {}: {}", pair, e);
                
                // Attempt to reconnect
                warn!("🔄 Attempting to reconnect...");
                sleep(Duration::from_secs(5)).await;
                
                match ws_client.reconnect().await {
                    Ok(_) => info!("✅ Reconnected successfully"),
                    Err(e) => {
                        error!("❌ Reconnection failed: {}", e);
                        break;
                    }
                }
            }
        }
        
        // Small delay to prevent excessive CPU usage
        sleep(Duration::from_millis(100)).await;
    }
    
    info!("🏁 Trading loop ended for {}", pair);
    Ok(())
}

async fn load_all_strategies() -> Result<HashMap<String, OptimizedStrategy>, Box<dyn std::error::Error>> {
    let mut strategies = HashMap::new();
    
    if !std::path::Path::new("strategies").exists() {
        return Ok(strategies);
    }
    
    for entry in fs::read_dir("strategies")? {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("json") 
           && path.file_name().and_then(|s| s.to_str()).map_or(false, |s| s.contains("_optimized")) {
            
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(strategy) = serde_json::from_str::<OptimizedStrategy>(&content) {
                    // Only load strategies with good confidence scores
                    if strategy.confidence_score >= 60.0 && strategy.sharpe_ratio > 0.5 {
                        strategies.insert(strategy.trading_pair.clone(), strategy);
                    }
                }
            }
        }
    }
    
    Ok(strategies)
}

async fn load_strategy(pair: &str) -> Result<OptimizedStrategy, Box<dyn std::error::Error>> {
    let filename = format!("strategies/{}_optimized.json", pair.to_lowercase());
    let content = fs::read_to_string(filename)?;
    let strategy: OptimizedStrategy = serde_json::from_str(&content)?;
    Ok(strategy)
}

async fn list_strategies() -> Result<(), Box<dyn std::error::Error>> {
    let strategies = load_all_strategies().await?;
    
    if strategies.is_empty() {
        println!("❌ No strategies found!");
        println!("Run 'cargo run --bin backtest auto' to generate strategies first.");
        return Ok(());
    }
    
    println!("📋 Available Trading Strategies:");
    println!("=================================");
    
    for (pair, strategy) in strategies {
        println!("🎯 {}", pair);
        println!("   Expected Return: {:.2}%/year", strategy.expected_annual_return);
        println!("   Sharpe Ratio: {:.2}", strategy.sharpe_ratio);
        println!("   Confidence: {:.1}%", strategy.confidence_score);
        println!("   Generated: {}", strategy.generated_at.format("%Y-%m-%d %H:%M"));
        println!();
    }
    
    println!("To start live trading: cargo run --bin trade start");
    println!("To start dry run: cargo run --bin trade start --dry-run");
    
    Ok(())
}

async fn monitor_trades() -> Result<(), Box<dyn std::error::Error>> {
    // Load current trading session
    if !std::path::Path::new("session.json").exists() {
        println!("❌ No active trading session found");
        return Ok(());
    }
    
    let content = fs::read_to_string("session.json")?;
    let session: TradingSession = serde_json::from_str(&content)?;
    
    println!("📊 Trading Session Monitor");
    println!("==========================");
    println!("Session ID: {}", session.session_id);
    println!("Started: {}", session.started_at.format("%Y-%m-%d %H:%M:%S"));
    println!("Mode: {}", if session.dry_run { "DRY RUN" } else { "LIVE" });
    println!("Total Capital: £{:.2}", session.total_capital_allocated);
    println!("Active Strategies: {}", session.strategies.len());
    
    // TODO: Add real-time trade monitoring
    println!("\n🔄 Real-time monitoring will be implemented in next phase");
    
    Ok(())
}

// Placeholder functions - implement with your existing logic
fn detect_market_state(price_update: &PriceUpdate) -> MarketState {
    // Use your existing market state detection logic
    MarketState::Ranging
}

async fn log_simulated_trade(pair: &str, signal: &GridSignal) -> Result<(), Box<dyn std::error::Error>> {
    info!("📝 Simulated trade logged for {}: {:?}", pair, signal);
    Ok(())
}

async fn execute_live_trade(
    pair: &str, 
    signal: &GridSignal, 
    ws_client: &mut WebSocketClient
) -> Result<(), Box<dyn std::error::Error>> {
    // Implement actual trade execution via Kraken API
    info!("🔄 Executing live trade for {}: {:?}", pair, signal);
    Ok(())
}

fn should_stop_trading(trader: &GridTrader, strategy: &OptimizedStrategy) -> bool {
    // Implement risk management logic
    false
}

fn save_trading_session(session: &TradingSession) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(session)?;
    fs::write("session.json", json)?;
    Ok(())
}

// Placeholder types - these should match your actual types
struct PriceUpdate {
    price: f64,
}

struct GridSignal {
    signal_type: String,
    price: f64,
}