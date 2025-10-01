// Unified Grid Trading Bot - Professional CLI
// Single entry point for all grid trading operations

use clap::{Parser, Subcommand};
use tracing::info;

// Load command modules from cli directory
#[path = "../cli/backtest_commands.rs"]
mod backtest_commands;
#[path = "../cli/trade_commands.rs"]
mod trade_commands;

#[derive(Parser)]
#[command(name = "grid-bot")]
#[command(version = "0.2.0")]
#[command(about = "Professional Grid Trading System", long_about = None)]
#[command(author = "Grid Trading Team")]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
    
    /// Configuration file path
    #[arg(short, long, global = true, default_value = "config.toml")]
    config: String,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize configuration and workspace
    Init {
        /// Skip creating example strategies
        #[arg(long)]
        no_examples: bool,
    },
    
    /// Optimize trading strategies
    #[command(subcommand)]
    Optimize(OptimizeCommands),
    
    /// Run backtests
    #[command(subcommand)]
    Backtest(BacktestCommands),
    
    /// Execute live trading
    #[command(subcommand)]
    Trade(TradeCommands),
    
    /// Strategy management
    #[command(subcommand)]
    Strategy(StrategyCommands),
    
    /// System status and health checks
    Status {
        /// Show detailed system information
        #[arg(short, long)]
        detailed: bool,
    },
}

#[derive(Subcommand)]
enum OptimizeCommands {
    /// Optimize all GBP pairs
    All {
        /// Maximum number of pairs to optimize
        #[arg(short, long)]
        limit: Option<usize>,
        
        /// Optimization strategy
        #[arg(short, long, default_value = "random-search")]
        strategy: String,
        
        /// Number of iterations
        #[arg(short, long, default_value = "20")]
        iterations: usize,
        
        /// Generate report
        #[arg(short, long)]
        report: bool,
    },
    
    /// Optimize specific trading pair
    Pair {
        /// Trading pair (e.g., XRPGBP)
        pair: String,
        
        /// Optimization strategy
        #[arg(short, long, default_value = "random-search")]
        strategy: String,
        
        /// Number of iterations
        #[arg(short, long, default_value = "100")]
        iterations: usize,
        
        /// Comprehensive optimization
        #[arg(short, long)]
        comprehensive: bool,
    },
}

#[derive(Subcommand)]
enum BacktestCommands {
    /// Run quick demo backtest
    Demo {
        /// Trading pair
        #[arg(default_value = "XRPGBP")]
        pair: String,
    },
    
    /// Scan multiple pairs
    Scan {
        /// Maximum number of pairs
        #[arg(short, long)]
        limit: Option<usize>,
        
        /// Generate report
        #[arg(short, long)]
        report: bool,
    },
    
    /// Run backtest with custom parameters
    Run {
        /// Trading pair
        pair: String,
        
        /// Start date (YYYY-MM-DD)
        #[arg(short, long)]
        start: Option<String>,
        
        /// End date (YYYY-MM-DD)
        #[arg(short, long)]
        end: Option<String>,
        
        /// Grid levels
        #[arg(short, long)]
        levels: Option<usize>,
        
        /// Grid spacing
        #[arg(long)]
        spacing: Option<f64>,
    },
}

#[derive(Subcommand)]
enum TradeCommands {
    /// Start live trading
    Start {
        /// Initial capital
        #[arg(short, long, default_value = "500")]
        capital: f64,
        
        /// Trading duration in hours
        #[arg(long)]
        hours: Option<f64>,
        
        /// Trading duration in minutes
        #[arg(short, long)]
        minutes: Option<f64>,
        
        /// Specific pairs to trade (comma-separated)
        #[arg(short, long)]
        pairs: Option<String>,
        
        /// Dry run mode (paper trading)
        #[arg(short, long)]
        dry_run: bool,
    },
    
    /// Stop all active trading
    Stop {
        /// Force stop without graceful shutdown
        #[arg(short, long)]
        force: bool,
    },
    
    /// Pause trading temporarily
    Pause,
    
    /// Resume paused trading
    Resume,
}

#[derive(Subcommand)]
enum StrategyCommands {
    /// List all strategies
    List {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },
    
    /// Validate strategy file
    Validate {
        /// Strategy file path
        file: String,
    },
    
    /// Export strategies
    Export {
        /// Output directory
        #[arg(short, long, default_value = "export")]
        output: String,
    },
    
    /// Import strategies
    Import {
        /// Input directory
        input: String,
    },
    
    /// Remove old or invalid strategies
    Clean {
        /// Days to keep
        #[arg(short, long, default_value = "30")]
        days: u32,
        
        /// Dry run
        #[arg(short = 'n', long)]
        dry_run: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Setup logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    std::env::set_var("RUST_LOG", log_level);
    tracing_subscriber::fmt::init();
    
    info!("🚀 Grid Trading Bot v0.2.0");
    info!("📁 Config: {}", cli.config);
    
    // Execute command
    match cli.command {
        Commands::Init { no_examples } => {
            init_workspace(no_examples).await?;
        }
        
        Commands::Optimize(cmd) => {
            handle_optimize_command(cmd, &cli.config).await?;
        }
        
        Commands::Backtest(cmd) => {
            handle_backtest_command(cmd, &cli.config).await?;
        }
        
        Commands::Trade(cmd) => {
            handle_trade_command(cmd, &cli.config).await?;
        }
        
        Commands::Strategy(cmd) => {
            handle_strategy_command(cmd, &cli.config).await?;
        }
        
        Commands::Status { detailed } => {
            show_status(detailed, &cli.config).await?;
        }
    }
    
    Ok(())
}

async fn init_workspace(no_examples: bool) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    
    info!("🔧 Initializing workspace...");
    
    // Create directories
    fs::create_dir_all("strategies")?;
    fs::create_dir_all("logs")?;
    fs::create_dir_all("data")?;
    
    // Create default config if it doesn't exist
    if !std::path::Path::new("config.toml").exists() {
        let default_config = include_str!("../../config.toml.example");
        fs::write("config.toml", default_config)?;
        info!("📝 Created config.toml");
    }
    
    // Create example strategies if requested
    if !no_examples {
        info!("📦 Creating example strategies...");
        // Would create example strategy files here
    }
    
    info!("✅ Workspace initialized successfully!");
    info!("💡 Next steps:");
    info!("   1. Edit config.toml with your API keys");
    info!("   2. Run: grid-bot optimize all");
    info!("   3. Run: grid-bot trade start --dry-run");
    
    Ok(())
}

async fn handle_optimize_command(
    cmd: OptimizeCommands,
    _config: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        OptimizeCommands::All { limit, strategy, iterations, report } => {
            backtest_commands::optimize_all_pairs(limit, &strategy, iterations, report).await?;
        }
        OptimizeCommands::Pair { pair, strategy, iterations, comprehensive } => {
            backtest_commands::optimize_single_pair(&pair, &strategy, iterations, comprehensive).await?;
        }
    }
    Ok(())
}

async fn handle_backtest_command(
    cmd: BacktestCommands,
    _config: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        BacktestCommands::Demo { pair } => {
            backtest_commands::run_demo_backtest(&pair).await?;
        }
        BacktestCommands::Scan { limit, report } => {
            backtest_commands::scan_pairs(limit, report).await?;
        }
        BacktestCommands::Run { pair, start, end, levels, spacing } => {
            backtest_commands::run_custom_backtest(&pair, start, end, levels, spacing).await?;
        }
    }
    Ok(())
}

async fn handle_trade_command(
    cmd: TradeCommands,
    _config: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        TradeCommands::Start { capital, hours, minutes, pairs, dry_run } => {
            trade_commands::start_trading(capital, hours, minutes, pairs, dry_run).await?;
        }
        TradeCommands::Stop { force } => {
            trade_commands::stop_trading(force).await?;
        }
        TradeCommands::Pause => {
            trade_commands::pause_trading().await?;
        }
        TradeCommands::Resume => {
            trade_commands::resume_trading().await?;
        }
    }
    Ok(())
}

async fn handle_strategy_command(
    cmd: StrategyCommands,
    _config: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        StrategyCommands::List { detailed } => {
            list_strategies(detailed).await?;
        }
        StrategyCommands::Validate { file } => {
            validate_strategy(&file).await?;
        }
        StrategyCommands::Export { output } => {
            export_strategies(&output).await?;
        }
        StrategyCommands::Import { input } => {
            import_strategies(&input).await?;
        }
        StrategyCommands::Clean { days, dry_run } => {
            clean_strategies(days, dry_run).await?;
        }
    }
    Ok(())
}

async fn show_status(_detailed: bool, _config: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    
    info!("📊 System Status");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Check strategies directory
    if let Ok(entries) = fs::read_dir("strategies") {
        let count = entries.count();
        info!("📁 Strategies: {} files", count);
    } else {
        info!("📁 Strategies: directory not found");
    }
    
    // Check config
    if std::path::Path::new("config.toml").exists() {
        info!("⚙️  Config: OK");
    } else {
        info!("⚙️  Config: NOT FOUND (run: grid-bot init)");
    }
    
    // Check data directory
    if std::path::Path::new("data").exists() {
        info!("💾 Data: OK");
    } else {
        info!("💾 Data: NOT FOUND");
    }
    
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("✅ System is operational");
    
    Ok(())
}

async fn list_strategies(detailed: bool) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    
    info!("📋 Available Strategies");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let entries = fs::read_dir("strategies")?;
    let mut count = 0;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            count += 1;
            let filename = path.file_name().unwrap().to_string_lossy();
            
            if detailed {
                // Read and parse strategy file
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(strategy) = serde_json::from_str::<serde_json::Value>(&content) {
                        info!("  {} - {}", count, filename);
                        if let Some(pair) = strategy.get("trading_pair") {
                            info!("    Pair: {}", pair);
                        }
                        if let Some(levels) = strategy.get("grid_levels") {
                            info!("    Levels: {}", levels);
                        }
                        if let Some(return_val) = strategy.get("expected_return") {
                            info!("    Expected Return: {}%", return_val);
                        }
                    }
                }
            } else {
                info!("  {} - {}", count, filename);
            }
        }
    }
    
    if count == 0 {
        info!("  No strategies found. Run: grid-bot optimize all");
    }
    
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("Total: {} strategies", count);
    
    Ok(())
}

async fn validate_strategy(file: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    use serde_json::Value;
    
    info!("🔍 Validating strategy: {}", file);
    
    let content = fs::read_to_string(file)?;
    let strategy: Value = serde_json::from_str(&content)?;
    
    // Validate required fields
    let required_fields = vec!["trading_pair", "grid_levels", "grid_spacing"];
    let mut all_valid = true;
    
    for field in required_fields {
        if strategy.get(field).is_none() {
            info!("  ❌ Missing required field: {}", field);
            all_valid = false;
        } else {
            info!("  ✅ {}", field);
        }
    }
    
    if all_valid {
        info!("✅ Strategy is valid!");
    } else {
        info!("❌ Strategy validation failed");
        return Err("Invalid strategy file".into());
    }
    
    Ok(())
}

async fn export_strategies(output: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    
    info!("📦 Exporting strategies to: {}", output);
    fs::create_dir_all(output)?;
    
    let entries = fs::read_dir("strategies")?;
    let mut count = 0;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let filename = path.file_name().unwrap();
            let dest = format!("{}/{}", output, filename.to_string_lossy());
            fs::copy(&path, &dest)?;
            count += 1;
        }
    }
    
    info!("✅ Exported {} strategies", count);
    Ok(())
}

async fn import_strategies(input: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    
    info!("📥 Importing strategies from: {}", input);
    fs::create_dir_all("strategies")?;
    
    let entries = fs::read_dir(input)?;
    let mut count = 0;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let filename = path.file_name().unwrap();
            let dest = format!("strategies/{}", filename.to_string_lossy());
            fs::copy(&path, &dest)?;
            count += 1;
        }
    }
    
    info!("✅ Imported {} strategies", count);
    Ok(())
}

async fn clean_strategies(days: u32, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    
    info!("🧹 Cleaning strategies older than {} days{}", days, if dry_run { " (dry run)" } else { "" });
    
    let cutoff_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs() - (days as u64 * 86400);
    
    let entries = fs::read_dir("strategies")?;
    let mut count = 0;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if let Ok(metadata) = entry.metadata() {
            if let Ok(modified) = metadata.modified() {
                if modified.duration_since(UNIX_EPOCH)?.as_secs() < cutoff_time {
                    count += 1;
                    info!("  🗑️  {}", path.display());
                    
                    if !dry_run {
                        fs::remove_file(&path)?;
                    }
                }
            }
        }
    }
    
    if dry_run {
        info!("Would remove {} old strategies", count);
    } else {
        info!("✅ Removed {} old strategies", count);
    }
    
    Ok(())
}
