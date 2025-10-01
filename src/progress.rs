//! Progress bar utilities for long-running operations
//! 
//! Provides visual feedback during optimization, backtesting, and other
//! time-consuming operations using the indicatif crate.

use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use std::time::Duration;

/// Progress bar for optimization iterations
pub struct OptimizationProgress {
    pub progress: ProgressBar,
    pub total_iterations: usize,
}

impl OptimizationProgress {
    /// Create a new optimization progress bar
    pub fn new(total_iterations: usize) -> Self {
        let progress = ProgressBar::new(total_iterations as u64);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})\n{msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        
        Self {
            progress,
            total_iterations,
        }
    }

    /// Update progress with current iteration and best score
    pub fn update(&self, iteration: usize, best_score: f64, current_params: &str) {
        self.progress.set_position(iteration as u64);
        self.progress.set_message(format!(
            "🎯 Best Score: {:.4} | Testing: {}",
            best_score, current_params
        ));
    }

    /// Mark optimization as complete
    pub fn finish(&self, best_score: f64) {
        self.progress.finish_with_message(format!(
            "✅ Optimization complete! Best score: {:.4}",
            best_score
        ));
    }

    /// Mark optimization as failed
    pub fn finish_with_error(&self, error: &str) {
        self.progress.finish_with_message(format!("❌ Failed: {}", error));
    }
}

/// Progress bar for backtesting operations
pub struct BacktestProgress {
    pub progress: ProgressBar,
}

impl BacktestProgress {
    /// Create a new backtest progress bar
    pub fn new(total_steps: usize) -> Self {
        let progress = ProgressBar::new(total_steps as u64);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len}\n{msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        
        Self { progress }
    }

    /// Update with current step
    pub fn set_step(&self, step: &str) {
        self.progress.inc(1);
        self.progress.set_message(format!("📊 {}", step));
    }

    /// Mark backtest as complete
    pub fn finish(&self, total_trades: usize, return_pct: f64) {
        self.progress.finish_with_message(format!(
            "✅ Backtest complete! {} trades, {:.2}% return",
            total_trades, return_pct
        ));
    }
}

/// Spinner for quick operations
pub struct Spinner {
    pub spinner: ProgressBar,
}

impl Spinner {
    /// Create a new spinner
    pub fn new(message: &str) -> Self {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
        );
        spinner.enable_steady_tick(Duration::from_millis(100));
        spinner.set_message(message.to_string());
        
        Self { spinner }
    }

    /// Update spinner message
    pub fn update(&self, message: &str) {
        self.spinner.set_message(message.to_string());
    }

    /// Finish spinner with success
    pub fn finish(&self, message: &str) {
        self.spinner.finish_with_message(format!("✅ {}", message));
    }

    /// Finish spinner with error
    pub fn finish_with_error(&self, message: &str) {
        self.spinner.finish_with_message(format!("❌ {}", message));
    }
}

/// Multi-progress for parallel operations
pub struct MultiOptimization {
    #[allow(dead_code)]
    multi: MultiProgress,
    bars: Vec<ProgressBar>,
}

impl MultiOptimization {
    /// Create a new multi-progress for multiple pairs
    pub fn new(pairs: &[String]) -> Self {
        let multi = MultiProgress::new();
        let mut bars = Vec::new();
        
        for pair in pairs {
            let pb = multi.add(ProgressBar::new(100));
            pb.set_style(
                ProgressStyle::default_bar()
                    .template(&format!("{}: {{bar:20.cyan/blue}} {{pos}}% - {{msg}}", pair))
                    .unwrap()
                    .progress_chars("#>-")
            );
            bars.push(pb);
        }
        
        Self { multi, bars }
    }

    /// Update progress for a specific pair
    pub fn update_pair(&self, index: usize, progress: u64, message: &str) {
        if let Some(pb) = self.bars.get(index) {
            pb.set_position(progress);
            pb.set_message(message.to_string());
        }
    }

    /// Finish all progress bars
    pub fn finish_all(&self) {
        for pb in &self.bars {
            pb.finish_with_message("✅ Done");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimization_progress() {
        let progress = OptimizationProgress::new(100);
        progress.update(10, 0.75, "levels=10, spacing=0.02");
        progress.finish(0.85);
    }

    #[test]
    fn test_spinner() {
        let spinner = Spinner::new("Loading...");
        std::thread::sleep(Duration::from_millis(100));
        spinner.finish("Loaded");
    }
}
