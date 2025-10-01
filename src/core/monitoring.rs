// Real-time monitoring and safety features for live trading

use crate::core::{TradingError, HealthMonitor, PositionManager};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use tokio::time::interval;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct TradingMonitor {
    health_monitor: Arc<HealthMonitor>,
    performance_tracker: Arc<Mutex<PerformanceTracker>>,
    safety_limits: SafetyLimits,
    alert_system: AlertSystem,
    is_trading_enabled: Arc<Mutex<bool>>,
}

#[derive(Debug, Clone)]
pub struct SafetyLimits {
    pub max_daily_loss_pct: f64,
    pub max_drawdown_pct: f64,
    pub max_position_count: usize,
    pub max_trades_per_hour: u32,
    pub max_consecutive_losses: u32,
    pub min_account_balance: f64,
    pub volatility_shutdown_threshold: f64,
}

#[derive(Debug, Clone)]
pub struct PerformanceTracker {
    daily_pnl: f64,
    peak_portfolio_value: f64,
    current_drawdown: f64,
    trades_today: VecDeque<TradeRecord>,
    consecutive_losses: u32,
    last_trade_time: Option<Instant>,
    performance_metrics: RealTimeMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub pnl: f64,
    pub trade_type: String,
    pub quantity: f64,
    pub price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealTimeMetrics {
    pub uptime_hours: f64,
    pub total_trades: u64,
    pub win_rate: f64,
    pub average_trade_pnl: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub current_positions: u32,
    pub account_balance: f64,
}

#[derive(Debug, Clone)]
pub struct AlertSystem {
    pub email_alerts: bool,
    pub webhook_url: Option<String>,
    pub alert_history: Arc<Mutex<VecDeque<Alert>>>,
    pub max_alert_history: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub timestamp: DateTime<Utc>,
    pub level: AlertLevel,
    pub message: String,
    pub context: AlertContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
    Emergency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertContext {
    pub portfolio_value: f64,
    pub daily_pnl: f64,
    pub current_drawdown: f64,
    pub active_positions: u32,
    pub system_health: String,
}

impl TradingMonitor {
    pub fn new(safety_limits: SafetyLimits) -> Self {
        Self {
            health_monitor: Arc::new(HealthMonitor::new()),
            performance_tracker: Arc::new(Mutex::new(PerformanceTracker::new())),
            safety_limits,
            alert_system: AlertSystem::new(),
            is_trading_enabled: Arc::new(Mutex::new(true)),
        }
    }

    /// Main monitoring loop - should be run as a background task
    pub async fn start_monitoring(&self, position_manager: Arc<Mutex<PositionManager>>) {
        let mut interval = interval(Duration::from_secs(10)); // Monitor every 10 seconds
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.perform_monitoring_cycle(&position_manager).await {
                self.send_alert(
                    AlertLevel::Critical,
                    format!("Monitoring cycle failed: {}", e),
                    self.get_current_context(&position_manager).await,
                ).await;
            }
        }
    }

    async fn perform_monitoring_cycle(
        &self,
        position_manager: &Arc<Mutex<PositionManager>>,
    ) -> Result<(), TradingError> {
        // 1. Health check
        let health_status = self.health_monitor.perform_health_check().await?;
        
        // 2. Performance tracking
        self.update_performance_metrics(position_manager).await;
        
        // 3. Safety limit checks
        self.check_safety_limits(position_manager).await?;
        
        // 4. System health alerts
        if health_status != crate::core::error_handling::HealthStatus::Healthy {
            self.send_alert(
                AlertLevel::Warning,
                format!("System health degraded: {:?}", health_status),
                self.get_current_context(position_manager).await,
            ).await;
        }

        Ok(())
    }

    async fn update_performance_metrics(&self, position_manager: &Arc<Mutex<PositionManager>>) {
        let portfolio_summary = {
            let pm = position_manager.lock().unwrap();
            pm.get_portfolio_summary()
        };

        let mut tracker = self.performance_tracker.lock().unwrap();
        
        // Update peak value and drawdown
        if portfolio_summary.total_value > tracker.peak_portfolio_value {
            tracker.peak_portfolio_value = portfolio_summary.total_value;
        }
        
        tracker.current_drawdown = (tracker.peak_portfolio_value - portfolio_summary.total_value) 
            / tracker.peak_portfolio_value;
        
        // Update daily P&L
        tracker.daily_pnl = portfolio_summary.total_pnl;
        
        // Clean old trades (keep only today's)
        let now = Utc::now();
        tracker.trades_today.retain(|trade| {
            (now - trade.timestamp).num_hours() < 24
        });
        
        // Update metrics
        tracker.performance_metrics = RealTimeMetrics {
            uptime_hours: 0.0, // Would be calculated from start time
            total_trades: tracker.trades_today.len() as u64,
            win_rate: self.calculate_win_rate(&tracker.trades_today),
            average_trade_pnl: self.calculate_average_pnl(&tracker.trades_today),
            sharpe_ratio: 0.0, // Would need more sophisticated calculation
            max_drawdown: tracker.current_drawdown,
            current_positions: portfolio_summary.position_count as u32,
            account_balance: portfolio_summary.total_value,
        };
    }

    async fn check_safety_limits(
        &self,
        position_manager: &Arc<Mutex<PositionManager>>,
    ) -> Result<(), TradingError> {
        let portfolio_summary = {
            let pm = position_manager.lock().unwrap();
            pm.get_portfolio_summary()
        };

        let tracker = self.performance_tracker.lock().unwrap();
        
        // Check daily loss limit
        let daily_loss_pct = (-tracker.daily_pnl / portfolio_summary.total_value).max(0.0);
        if daily_loss_pct > self.safety_limits.max_daily_loss_pct {
            self.emergency_shutdown("Daily loss limit exceeded").await;
            return Err(TradingError::RiskViolation(
                format!("Daily loss {:.2}% exceeds limit {:.2}%", 
                        daily_loss_pct * 100.0, 
                        self.safety_limits.max_daily_loss_pct * 100.0)
            ));
        }

        // Check drawdown limit
        if tracker.current_drawdown > self.safety_limits.max_drawdown_pct {
            self.emergency_shutdown("Maximum drawdown exceeded").await;
            return Err(TradingError::RiskViolation(
                format!("Drawdown {:.2}% exceeds limit {:.2}%", 
                        tracker.current_drawdown * 100.0, 
                        self.safety_limits.max_drawdown_pct * 100.0)
            ));
        }

        // Check position count
        if portfolio_summary.position_count > self.safety_limits.max_position_count {
            self.send_alert(
                AlertLevel::Warning,
                format!("Position count {} exceeds recommended limit {}", 
                        portfolio_summary.position_count, 
                        self.safety_limits.max_position_count),
                self.get_current_context(position_manager).await,
            ).await;
        }

        // Check minimum account balance
        if portfolio_summary.total_value < self.safety_limits.min_account_balance {
            self.emergency_shutdown("Account balance below minimum").await;
            return Err(TradingError::RiskViolation(
                format!("Account balance £{:.2} below minimum £{:.2}", 
                        portfolio_summary.total_value, 
                        self.safety_limits.min_account_balance)
            ));
        }

        // Check consecutive losses
        if tracker.consecutive_losses > self.safety_limits.max_consecutive_losses {
            self.pause_trading("Too many consecutive losses").await;
        }

        // Check trades per hour
        let recent_trades = tracker.trades_today.iter()
            .filter(|trade| (Utc::now() - trade.timestamp).num_minutes() < 60)
            .count() as u32;
        
        if recent_trades > self.safety_limits.max_trades_per_hour {
            self.pause_trading("Trading frequency too high").await;
        }

        Ok(())
    }

    async fn emergency_shutdown(&self, reason: &str) {
        {
            let mut trading_enabled = self.is_trading_enabled.lock().unwrap();
            *trading_enabled = false;
        }

        self.send_alert(
            AlertLevel::Emergency,
            format!("EMERGENCY SHUTDOWN: {}", reason),
            AlertContext {
                portfolio_value: 0.0,
                daily_pnl: 0.0,
                current_drawdown: 0.0,
                active_positions: 0,
                system_health: "SHUTDOWN".to_string(),
            },
        ).await;

        println!("🚨 EMERGENCY SHUTDOWN: {}", reason);
    }

    async fn pause_trading(&self, reason: &str) {
        {
            let mut trading_enabled = self.is_trading_enabled.lock().unwrap();
            *trading_enabled = false;
        }

        self.send_alert(
            AlertLevel::Critical,
            format!("Trading paused: {}", reason),
            AlertContext {
                portfolio_value: 0.0,
                daily_pnl: 0.0,
                current_drawdown: 0.0,
                active_positions: 0,
                system_health: "PAUSED".to_string(),
            },
        ).await;

        println!("⏸️  Trading paused: {}", reason);
        
        // Auto-resume after 1 hour (could be configurable)
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(3600)).await;
            // Would need access to self to re-enable
            println!("🔄 Auto-resuming trading after pause");
        });
    }

    pub async fn record_trade(&self, trade: TradeRecord) {
        let mut tracker = self.performance_tracker.lock().unwrap();
        
        // Update consecutive losses counter
        if trade.pnl < 0.0 {
            tracker.consecutive_losses += 1;
        } else {
            tracker.consecutive_losses = 0;
        }
        
        tracker.trades_today.push_back(trade.clone());
        tracker.last_trade_time = Some(Instant::now());

        // Check if this trade triggers any alerts
        if trade.pnl < -100.0 { // Large loss threshold
            self.send_alert(
                AlertLevel::Warning,
                format!("Large loss on trade: £{:.2}", trade.pnl),
                AlertContext {
                    portfolio_value: 0.0,
                    daily_pnl: tracker.daily_pnl,
                    current_drawdown: tracker.current_drawdown,
                    active_positions: 0,
                    system_health: "ACTIVE".to_string(),
                },
            ).await;
        }
    }

    pub fn is_trading_allowed(&self) -> bool {
        *self.is_trading_enabled.lock().unwrap()
    }

    pub async fn manual_shutdown(&self, reason: &str) {
        self.emergency_shutdown(reason).await;
    }

    pub fn enable_trading(&self) {
        let mut trading_enabled = self.is_trading_enabled.lock().unwrap();
        *trading_enabled = true;
        println!("✅ Trading manually enabled");
    }

    pub fn get_performance_summary(&self) -> RealTimeMetrics {
        let tracker = self.performance_tracker.lock().unwrap();
        tracker.performance_metrics.clone()
    }

    async fn send_alert(&self, level: AlertLevel, message: String, context: AlertContext) {
        let alert = Alert {
            timestamp: Utc::now(),
            level: level.clone(),
            message: message.clone(),
            context,
        };

        // Store alert
        {
            let mut history = self.alert_system.alert_history.lock().unwrap();
            history.push_back(alert.clone());
            if history.len() > self.alert_system.max_alert_history {
                history.pop_front();
            }
        }

        // Send alert (implementation would depend on configured channels)
        match level {
            AlertLevel::Emergency | AlertLevel::Critical => {
                println!("🚨 CRITICAL ALERT: {}", message);
                // Would send email/webhook here
            }
            AlertLevel::Warning => {
                println!("⚠️  WARNING: {}", message);
            }
            AlertLevel::Info => {
                println!("ℹ️  INFO: {}", message);
            }
        }
    }

    async fn get_current_context(&self, position_manager: &Arc<Mutex<PositionManager>>) -> AlertContext {
        let portfolio_summary = {
            let pm = position_manager.lock().unwrap();
            pm.get_portfolio_summary()
        };

        let tracker = self.performance_tracker.lock().unwrap();

        AlertContext {
            portfolio_value: portfolio_summary.total_value,
            daily_pnl: tracker.daily_pnl,
            current_drawdown: tracker.current_drawdown,
            active_positions: portfolio_summary.position_count as u32,
            system_health: if self.is_trading_allowed() { "ACTIVE" } else { "PAUSED" }.to_string(),
        }
    }

    fn calculate_win_rate(&self, trades: &VecDeque<TradeRecord>) -> f64 {
        if trades.is_empty() {
            return 0.0;
        }

        let winning_trades = trades.iter().filter(|t| t.pnl > 0.0).count();
        winning_trades as f64 / trades.len() as f64
    }

    fn calculate_average_pnl(&self, trades: &VecDeque<TradeRecord>) -> f64 {
        if trades.is_empty() {
            return 0.0;
        }

        let total_pnl: f64 = trades.iter().map(|t| t.pnl).sum();
        total_pnl / trades.len() as f64
    }
}

impl PerformanceTracker {
    fn new() -> Self {
        Self {
            daily_pnl: 0.0,
            peak_portfolio_value: 0.0,
            current_drawdown: 0.0,
            trades_today: VecDeque::new(),
            consecutive_losses: 0,
            last_trade_time: None,
            performance_metrics: RealTimeMetrics {
                uptime_hours: 0.0,
                total_trades: 0,
                win_rate: 0.0,
                average_trade_pnl: 0.0,
                sharpe_ratio: 0.0,
                max_drawdown: 0.0,
                current_positions: 0,
                account_balance: 0.0,
            },
        }
    }
}

impl AlertSystem {
    fn new() -> Self {
        Self {
            email_alerts: false,
            webhook_url: None,
            alert_history: Arc::new(Mutex::new(VecDeque::new())),
            max_alert_history: 1000,
        }
    }
}

impl Default for SafetyLimits {
    fn default() -> Self {
        Self {
            max_daily_loss_pct: 0.05,        // 5% daily loss
            max_drawdown_pct: 0.15,          // 15% max drawdown
            max_position_count: 10,          // Max 10 concurrent positions
            max_trades_per_hour: 100,        // Max 100 trades per hour
            max_consecutive_losses: 5,       // Pause after 5 consecutive losses
            min_account_balance: 1000.0,     // £1000 minimum balance
            volatility_shutdown_threshold: 0.1, // 10% volatility threshold
        }
    }
}