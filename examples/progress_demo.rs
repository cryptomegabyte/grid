//! Example program demonstrating progress bars in grid trading bot
//! 
//! Run with: cargo run --example progress_demo

use grid_trading_bot::{OptimizationProgress, BacktestProgress, Spinner};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    println!("🎨 Grid Trading Bot - Progress Bar Demo\n");
    
    // Demo 1: Spinner for quick operations
    println!("1️⃣  Spinner Demo (quick operations):");
    let spinner = Spinner::new("Loading configuration...");
    sleep(Duration::from_millis(500)).await;
    spinner.update("Validating API keys...");
    sleep(Duration::from_millis(500)).await;
    spinner.update("Connecting to database...");
    sleep(Duration::from_millis(500)).await;
    spinner.finish("System ready!");
    
    println!("\n");
    
    // Demo 2: Optimization Progress Bar
    println!("2️⃣  Optimization Progress Demo:");
    let opt_progress = OptimizationProgress::new(50);
    
    for i in 1..=50 {
        let score = 0.5 + (i as f64 * 0.01);
        let params = format!(
            "levels={}, spacing={:.3}",
            5 + (i % 10),
            0.01 + (i as f64 * 0.001)
        );
        opt_progress.update(i, score, &params);
        sleep(Duration::from_millis(50)).await;
    }
    
    opt_progress.finish(1.0);
    
    println!("\n");
    
    // Demo 3: Backtest Progress Bar
    println!("3️⃣  Backtest Progress Demo:");
    let bt_progress = BacktestProgress::new(7);
    
    let steps = vec![
        "Fetching historical data...",
        "Detecting market states...",
        "Computing grid levels...",
        "Detecting trading signals...",
        "Calculating trading costs...",
        "Simulating portfolio...",
        "Analyzing performance...",
    ];
    
    for step in steps {
        bt_progress.set_step(step);
        sleep(Duration::from_millis(300)).await;
    }
    
    bt_progress.finish(127, 45.8);
    
    println!("\n✨ Demo complete!");
}
