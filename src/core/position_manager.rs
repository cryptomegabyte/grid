// Advanced position management and risk control system

use crate::core::types::GridSignal;
use crate::core::error_handling::TradingError;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub struct PositionManager {
    positions: HashMap<String, Position>,
    risk_limits: RiskLimits,
    portfolio_value: f64,
    available_capital: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub quantity: f64,
    pub average_price: f64,
    pub market_value: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct RiskLimits {
    pub max_position_value: f64,           // Maximum value per position
    pub max_total_exposure: f64,           // Maximum total portfolio exposure
    pub max_daily_loss: f64,               // Maximum daily loss limit
    pub max_drawdown: f64,                 // Maximum drawdown from peak
    pub position_sizing_method: PositionSizingMethod,
    pub risk_per_trade: f64,               // Risk per trade as % of capital
}

#[derive(Debug, Clone)]
pub enum PositionSizingMethod {
    FixedAmount(f64),                      // Fixed dollar amount
    FixedPercentage(f64),                  // Fixed percentage of capital
    KellyOptimal,                          // Kelly criterion optimal sizing
    RiskParity,                            // Risk parity approach
    VolatilityAdjusted { target_vol: f64 }, // Volatility-adjusted sizing
}

impl PositionManager {
    pub fn new(initial_capital: f64, risk_limits: RiskLimits) -> Self {
        Self {
            positions: HashMap::new(),
            risk_limits,
            portfolio_value: initial_capital,
            available_capital: initial_capital,
        }
    }

    /// Check if a trade is allowed based on risk limits
    pub fn can_execute_trade(
        &self,
        symbol: &str,
        _trade_type: &GridSignal,
        price: f64,
        market_volatility: f64,
    ) -> Result<f64, TradingError> {
        // Calculate proposed position size
        let position_size = self.calculate_position_size(symbol, price, market_volatility)?;
        
        // Check individual position limits
        if position_size * price > self.risk_limits.max_position_value {
            return Err(TradingError::RiskViolation(
                format!("Position size ${:.2} exceeds max position limit ${:.2}",
                        position_size * price, self.risk_limits.max_position_value)
            ));
        }

        // Check total exposure limits
        let current_exposure = self.calculate_total_exposure();
        let new_exposure = current_exposure + (position_size * price);
        
        if new_exposure > self.risk_limits.max_total_exposure {
            return Err(TradingError::RiskViolation(
                format!("Total exposure ${:.2} would exceed limit ${:.2}",
                        new_exposure, self.risk_limits.max_total_exposure)
            ));
        }

        // Check available capital
        let required_capital = position_size * price;
        if required_capital > self.available_capital {
            return Err(TradingError::RiskViolation(
                format!("Insufficient capital: required ${:.2}, available ${:.2}",
                        required_capital, self.available_capital)
            ));
        }

        // Check daily loss limit
        let daily_pnl = self.calculate_daily_pnl();
        if daily_pnl < -self.risk_limits.max_daily_loss {
            return Err(TradingError::RiskViolation(
                format!("Daily loss limit exceeded: ${:.2}", daily_pnl.abs())
            ));
        }

        // Check maximum drawdown
        let current_drawdown = self.calculate_current_drawdown();
        if current_drawdown > self.risk_limits.max_drawdown {
            return Err(TradingError::RiskViolation(
                format!("Maximum drawdown exceeded: {:.2}%", current_drawdown * 100.0)
            ));
        }

        Ok(position_size)
    }

    /// Calculate optimal position size based on configured method
    fn calculate_position_size(
        &self,
        symbol: &str,
        price: f64,
        volatility: f64,
    ) -> Result<f64, TradingError> {
        match &self.risk_limits.position_sizing_method {
            PositionSizingMethod::FixedAmount(amount) => {
                Ok(amount / price)
            }
            PositionSizingMethod::FixedPercentage(pct) => {
                Ok((self.available_capital * pct) / price)
            }
            PositionSizingMethod::KellyOptimal => {
                self.calculate_kelly_position_size(symbol, price)
            }
            PositionSizingMethod::RiskParity => {
                self.calculate_risk_parity_size(price, volatility)
            }
            PositionSizingMethod::VolatilityAdjusted { target_vol } => {
                self.calculate_vol_adjusted_size(price, volatility, *target_vol)
            }
        }
    }

    fn calculate_kelly_position_size(&self, _symbol: &str, price: f64) -> Result<f64, TradingError> {
        // Simplified Kelly criterion - in practice, you'd need historical win/loss data
        let estimated_win_rate = 0.55; // 55% estimated win rate
        let estimated_avg_win = 0.02;  // 2% average win
        let estimated_avg_loss = 0.015; // 1.5% average loss
        
        let kelly_fraction: f64 = (estimated_win_rate * estimated_avg_win - 
                            (1.0 - estimated_win_rate) * estimated_avg_loss) / estimated_avg_win;
        
        let kelly_amount = self.available_capital * kelly_fraction.max(0.0).min(0.25); // Cap at 25%
        Ok(kelly_amount / price)
    }

    fn calculate_risk_parity_size(&self, price: f64, volatility: f64) -> Result<f64, TradingError> {
        let target_risk = self.available_capital * self.risk_limits.risk_per_trade;
        let position_value = target_risk / volatility.max(0.01); // Avoid division by zero
        Ok(position_value / price)
    }

    fn calculate_vol_adjusted_size(
        &self,
        price: f64,
        volatility: f64,
        target_vol: f64,
    ) -> Result<f64, TradingError> {
        let vol_adjustment = target_vol / volatility.max(0.01);
        let base_size = self.available_capital * 0.1; // 10% base allocation
        let adjusted_size = base_size * vol_adjustment;
        Ok(adjusted_size / price)
    }

    /// Execute a trade and update positions
    pub fn execute_trade(
        &mut self,
        symbol: &str,
        signal: &GridSignal,
        execution_price: f64,
        quantity: f64,
        fees: f64,
    ) -> Result<TradeExecution, TradingError> {
        // Create position if it doesn't exist
        if !self.positions.contains_key(symbol) {
            self.positions.insert(symbol.to_string(), Position {
                symbol: symbol.to_string(),
                quantity: 0.0,
                average_price: 0.0,
                market_value: 0.0,
                unrealized_pnl: 0.0,
                realized_pnl: 0.0,
                created_at: Utc::now(),
                last_updated: Utc::now(),
                stop_loss: None,
                take_profit: None,
            });
        }

        let trade_execution = match signal {
            GridSignal::Buy(_) => {
                let position = self.positions.get_mut(symbol).unwrap();
                Self::execute_buy_internal_mut(position, quantity, execution_price, fees)?
            }
            GridSignal::Sell(_) => {
                let position = self.positions.get_mut(symbol).unwrap();
                Self::execute_sell_internal_mut(position, quantity, execution_price, fees)?
            }
            GridSignal::None => {
                return Err(TradingError::PositionError("No signal to execute".to_string()));
            }
        };

        // Update available capital
        self.available_capital -= trade_execution.total_cost;
        
        // Update position timestamps
        if let Some(position) = self.positions.get_mut(symbol) {
            position.last_updated = Utc::now();
        }

        Ok(trade_execution)
    }

    fn execute_buy_internal_mut(
        position: &mut Position,
        quantity: f64,
        price: f64,
        fees: f64,
    ) -> Result<TradeExecution, TradingError> {
        let total_cost = quantity * price + fees;

        // Update position with new shares
        let old_quantity = position.quantity;
        let old_value = old_quantity * position.average_price;
        let new_value = quantity * price;
        
        position.quantity += quantity;
        if position.quantity > 0.0 {
            position.average_price = (old_value + new_value) / position.quantity;
        }

        Ok(TradeExecution {
            symbol: position.symbol.clone(),
            trade_type: TradeType::Buy,
            quantity,
            price,
            fees,
            total_cost,
            timestamp: Utc::now(),
        })
    }

    fn execute_sell_internal_mut(
        position: &mut Position,
        quantity: f64,
        price: f64,
        fees: f64,
    ) -> Result<TradeExecution, TradingError> {
        if position.quantity < quantity {
            return Err(TradingError::PositionError(
                format!("Insufficient position: have {}, trying to sell {}", 
                        position.quantity, quantity)
            ));
        }

        let total_proceeds = quantity * price - fees;
        
        // Calculate realized P&L
        let cost_basis = quantity * position.average_price;
        let realized_pnl = total_proceeds - cost_basis;
        position.realized_pnl += realized_pnl;

        // Update position
        position.quantity -= quantity;

        Ok(TradeExecution {
            symbol: position.symbol.clone(),
            trade_type: TradeType::Sell,
            quantity,
            price,
            fees,
            total_cost: -total_proceeds, // Negative because we're receiving money
            timestamp: Utc::now(),
        })
    }

    /// Update all positions with current market prices
    pub fn update_positions(&mut self, market_prices: &HashMap<String, f64>) {
        for (symbol, position) in &mut self.positions {
            if let Some(&current_price) = market_prices.get(symbol) {
                position.market_value = position.quantity * current_price;
                position.unrealized_pnl = position.market_value - (position.quantity * position.average_price);
            }
        }
        
        // Update portfolio value
        self.portfolio_value = self.available_capital + self.calculate_total_position_value();
    }

    /// Get current portfolio summary
    pub fn get_portfolio_summary(&self) -> PortfolioSummary {
        let total_unrealized_pnl: f64 = self.positions.values()
            .map(|p| p.unrealized_pnl)
            .sum();
        
        let total_realized_pnl: f64 = self.positions.values()
            .map(|p| p.realized_pnl)
            .sum();

        PortfolioSummary {
            total_value: self.portfolio_value,
            available_capital: self.available_capital,
            total_exposure: self.calculate_total_exposure(),
            unrealized_pnl: total_unrealized_pnl,
            realized_pnl: total_realized_pnl,
            total_pnl: total_unrealized_pnl + total_realized_pnl,
            position_count: self.positions.len(),
            positions: self.positions.clone(),
        }
    }

    // Helper methods
    fn calculate_total_exposure(&self) -> f64 {
        self.positions.values()
            .map(|p| p.market_value.abs())
            .sum()
    }

    fn calculate_total_position_value(&self) -> f64 {
        self.positions.values()
            .map(|p| p.market_value)
            .sum()
    }

    fn calculate_daily_pnl(&self) -> f64 {
        // Simplified - in practice, you'd track daily P&L separately
        self.positions.values()
            .map(|p| p.unrealized_pnl + p.realized_pnl)
            .sum()
    }

    fn calculate_current_drawdown(&self) -> f64 {
        // Simplified drawdown calculation
        // In practice, you'd track high-water marks
        let current_pnl_pct = (self.portfolio_value / self.available_capital) - 1.0;
        if current_pnl_pct < 0.0 {
            current_pnl_pct.abs()
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct TradeExecution {
    pub symbol: String,
    pub trade_type: TradeType,
    pub quantity: f64,
    pub price: f64,
    pub fees: f64,
    pub total_cost: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum TradeType {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub struct PortfolioSummary {
    pub total_value: f64,
    pub available_capital: f64,
    pub total_exposure: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub total_pnl: f64,
    pub position_count: usize,
    pub positions: HashMap<String, Position>,
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self {
            max_position_value: 1000.0,        // $1000 max per position
            max_total_exposure: 10000.0,       // $10k total exposure
            max_daily_loss: 500.0,             // $500 daily loss limit
            max_drawdown: 0.2,                 // 20% max drawdown
            position_sizing_method: PositionSizingMethod::FixedPercentage(0.1), // 10%
            risk_per_trade: 0.02,              // 2% risk per trade
        }
    }
}