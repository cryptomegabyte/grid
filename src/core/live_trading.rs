// Live Trading Engine with Realistic Simulation
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use tokio::time::{sleep, Duration, Instant};
use tracing::{info, warn, error, debug};
use uuid::Uuid;
use rand::{thread_rng, Rng};
use crate::clients::kraken_ws::{KrakenWebSocketClient, MarketData, OHLCData};
use crate::simulation::SimulationAdapter;
use crate::core::grid_trader::GridTrader;
use crate::core::types::GridSignal;
use crate::config::{TradingConfig, MarketConfig};
use futures_util::StreamExt;
use tokio_tungstenite::tungstenite::protocol::Message;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedStrategy {
    pub trading_pair: String,
    pub grid_levels: u32,
    pub grid_spacing: f64,
    pub expected_return: f64,
    pub total_trades: usize,
    pub win_rate: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub total_fees: f64,
    pub markov_confidence: f64,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct LiveStrategy {
    pub pair: String,
    pub config: OptimizedStrategy,
    pub grid_levels: Vec<f64>,
    
    // CRITICAL: Use position-safe GridTrader instead of manual tracking
    pub grid_trader: GridTrader,
    
    // Legacy fields kept for compatibility
    pub current_position: f64,  // Deprecated: use grid_trader.inventory_quantity
    pub available_capital: f64,  // Deprecated: use grid_trader.cash_balance
    
    pub active_orders: Vec<SimulatedOrder>,
    pub market_data: Option<MarketData>,
    pub recent_ohlc: Vec<OHLCData>,
    pub support_resistance: SupportResistanceLevels,
    pub volatility_metrics: VolatilityMetrics,
}

#[derive(Debug, Clone)]
pub struct SupportResistanceLevels {
    pub support_levels: Vec<f64>,
    pub resistance_levels: Vec<f64>,
    pub last_calculated: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct VolatilityMetrics {
    pub atr: f64,          // Average True Range
    pub std_dev: f64,      // Standard deviation of returns
    pub bollinger_upper: f64,
    pub bollinger_lower: f64,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum GridMode {
    Static,
    VolatilityAdaptive,
    SupportResistance,
    Fibonacci,
    TrendFollowing,
}

#[derive(Debug, Clone)]
pub struct SimulatedOrder {
    pub id: String,
    pub pair: String,
    pub side: String, // "buy" or "sell"
    pub price: f64,
    pub quantity: f64,
    pub timestamp: DateTime<Utc>,
    pub status: OrderStatus,
}

#[derive(Debug, Clone)]
pub enum OrderStatus {
    Pending,
    Filled,
    PartiallyFilled(f64), // filled quantity
    Cancelled,
    Rejected(String),
}

pub struct LiveTradingEngine {
    strategies: HashMap<String, LiveStrategy>,
    portfolio: PortfolioState,
    total_capital: f64,
    trade_history: Vec<SimulatedTrade>,
    #[allow(dead_code)]
    kraken_client: reqwest::Client,
    current_prices: HashMap<String, PriceData>,
    trade_log_file: String,
    portfolio_log_file: String,
    last_portfolio_update: Instant,
    ws_client: Option<KrakenWebSocketClient>,
    use_real_data: bool,
    grid_mode: GridMode,
    // New: Simulation engine for realistic order execution
    simulation_engine: Option<SimulationAdapter>,
    use_simulation_engine: bool,
}

#[derive(Debug, Clone)]
pub struct PriceData {
    pub bid: f64,
    pub ask: f64,
    pub last: f64,
    pub volume: f64,
    pub timestamp: DateTime<Utc>,
    pub volatility: f64,
    pub high_24h: f64,
    pub low_24h: f64,
}

#[derive(Debug, Clone)]
pub struct PortfolioState {
    pub cash_balance: f64,
    pub positions: HashMap<String, f64>, // pair -> quantity
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub total_fees_paid: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimulatedTrade {
    pub id: String,
    pub pair: String,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
    pub fee: f64,
    pub timestamp: DateTime<Utc>,
    pub execution_delay_ms: u64,
    pub slippage: f64,
}

impl LiveTradingEngine {
    /// Convert internal pair names to Kraken WebSocket format (verified from API)
    fn convert_to_kraken_pair(pair: &str) -> String {
        // Using exact WebSocket names from Kraken API response
        match pair {
            "AAVEGBP" => "AAVE/GBP".to_string(),
            "ADAGBP" => "ADA/GBP".to_string(),
            "ALGOGBP" => "ALGO/GBP".to_string(),
            "ATOMGBP" => "ATOM/GBP".to_string(),
            "BCHGBP" => "BCH/GBP".to_string(),
            "DOTGBP" => "DOT/GBP".to_string(),
            "ETHGBP" => "ETH/GBP".to_string(),
            "EURGBP" => "EUR/GBP".to_string(),
            "FILGBP" => "FIL/GBP".to_string(),
            "GRTGBP" => "GRT/GBP".to_string(),
            "KSMGBP" => "KSM/GBP".to_string(),
            "LINKGBP" => "LINK/GBP".to_string(),
            "LTCGBP" => "LTC/GBP".to_string(),
            "MINAGBP" => "MINA/GBP".to_string(),
            "PEPEGBP" => "PEPE/GBP".to_string(),
            "POPCATGBP" => "POPCAT/GBP".to_string(),
            "SANDGBP" => "SAND/GBP".to_string(),
            "SOLGBP" => "SOL/GBP".to_string(),
            "SUIGBP" => "SUI/GBP".to_string(),
            "USDCGBP" => "USDC/GBP".to_string(),
            "USDTGBP" => "USDT/GBP".to_string(),
            "XRPGBP" => "XRP/GBP".to_string(),
            _ => {
                warn!("⚠️  Unknown pair {}, using as-is", pair);
                pair.to_string()
            }
        }
    }

    /// Check if pair is supported for WebSocket subscriptions (all our GBP pairs are supported)
    fn is_websocket_supported(pair: &str) -> bool {
        matches!(pair, 
            "AAVEGBP" | "ADAGBP" | "ALGOGBP" | "ATOMGBP" | "BCHGBP" | "DOTGBP" | 
            "ETHGBP" | "EURGBP" | "FILGBP" | "GRTGBP" | "KSMGBP" | "LINKGBP" | 
            "LTCGBP" | "MINAGBP" | "PEPEGBP" | "POPCATGBP" | "SANDGBP" | "SOLGBP" | 
            "SUIGBP" | "USDCGBP" | "USDTGBP" | "XRPGBP"
        )
    }

    pub fn new(initial_capital: f64) -> Self {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        
        Self {
            strategies: HashMap::new(),
            portfolio: PortfolioState {
                cash_balance: initial_capital,
                positions: HashMap::new(),
                unrealized_pnl: 0.0,
                realized_pnl: 0.0,
                total_fees_paid: 0.0,
            },
            total_capital: initial_capital,
            trade_history: Vec::new(),
            kraken_client: reqwest::Client::new(),
            current_prices: HashMap::new(),
            trade_log_file: format!("logs/trades/trade_log_{}.csv", timestamp),
            portfolio_log_file: format!("logs/portfolio/portfolio_log_{}.csv", timestamp),
            last_portfolio_update: Instant::now(),
            ws_client: None,
            use_real_data: true,
            grid_mode: GridMode::VolatilityAdaptive,
            simulation_engine: Some(SimulationAdapter::new()),
            use_simulation_engine: true,
        }
    }

    pub fn with_real_data(mut self, enable: bool) -> Self {
        self.use_real_data = enable;
        self
    }

    pub fn with_grid_mode(mut self, mode: GridMode) -> Self {
        self.grid_mode = mode;
        self
    }

    /// Enable/disable simulation engine for order execution
    pub fn with_simulation_engine(mut self, enable: bool) -> Self {
        self.use_simulation_engine = enable;
        if enable && self.simulation_engine.is_none() {
            self.simulation_engine = Some(SimulationAdapter::new());
        }
        self
    }
    
    /// CRITICAL: Check portfolio-level risk limits before allowing any trade
    fn check_portfolio_risk(&self) -> Result<(), String> {
        let total_value = self.calculate_total_portfolio_value();
        let drawdown = (self.total_capital - total_value) / self.total_capital;
        
        // Check maximum drawdown limit (15%)
        if drawdown > 0.15 {
            return Err(format!("Portfolio drawdown {:.1}% exceeds 15% limit - TRADING HALTED", drawdown * 100.0));
        }
        
        // Check total exposure across all strategies
        let mut total_inventory_value = 0.0;
        for (pair, strategy) in &self.strategies {
            if let Some(price_data) = self.current_prices.get(pair) {
                let inventory_value = strategy.grid_trader.inventory_quantity() * price_data.last;
                total_inventory_value += inventory_value;
            }
        }
        
        let exposure_pct = total_inventory_value / total_value;
        if exposure_pct > 0.60 {
            return Err(format!("Total portfolio exposure {:.1}% exceeds 60% limit", exposure_pct * 100.0));
        }
        
        // Check daily loss limit
        let daily_pnl = self.portfolio.realized_pnl + self.portfolio.unrealized_pnl;
        let daily_loss_pct = daily_pnl / self.total_capital;
        if daily_loss_pct < -0.05 {
            return Err(format!("Daily loss {:.1}% exceeds 5% limit - TRADING HALTED", daily_loss_pct.abs() * 100.0));
        }
        
        Ok(())
    }
    
    fn calculate_total_portfolio_value(&self) -> f64 {
        let mut total = self.portfolio.cash_balance;
        for (pair, strategy) in &self.strategies {
            if let Some(price_data) = self.current_prices.get(pair) {
                total += strategy.grid_trader.get_portfolio_value(price_data.last);
            }
        }
        total
    }

    /// Initialize WebSocket connection for real market data
    pub async fn connect_market_data(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let ws_client = KrakenWebSocketClient::connect("wss://ws.kraken.com").await?;
        self.ws_client = Some(ws_client);
        
        info!("✅ Connected to Kraken WebSocket for real market data");
        Ok(())
    }

    /// Subscribe to market data for all trading pairs
    pub async fn subscribe_market_data(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ws_client) = &mut self.ws_client {
            let mut subscription_count = 0;
            for pair in self.strategies.keys() {
                if Self::is_websocket_supported(pair) {
                    let kraken_pair = Self::convert_to_kraken_pair(pair);
                    
                    // Subscribe to ticker data (most important)
                    if let Err(e) = ws_client.subscribe_to_ticker(&kraken_pair).await {
                        warn!("Failed to subscribe to ticker for {}: {}", pair, e);
                        continue;
                    }
                    
                    // Subscribe to OHLC data for technical analysis
                    if let Err(e) = ws_client.subscribe_to_ohlc(&kraken_pair, 1).await {
                        warn!("Failed to subscribe to OHLC for {}: {}", pair, e);
                    }
                    
                    subscription_count += 1;
                    info!("✅ Subscribed to market data for {}", pair);
                    
                    // Rate limiting to avoid overwhelming the server
                    tokio::time::sleep(Duration::from_millis(300)).await;
                } else {
                    info!("⚠️  {} not supported for WebSocket, will use REST API", pair);
                }
            }
            
            if subscription_count > 0 {
                info!("🎯 Successfully subscribed to {} pairs for real-time data", subscription_count);
            } else {
                warn!("⚠️  No WebSocket subscriptions active, using REST API only");
            }
        }
        Ok(())
    }

    /// Load all optimized strategies from the optimized_strategies folder
    pub fn load_optimized_strategies<P: AsRef<Path>>(&mut self, strategies_dir: P) -> Result<usize, Box<dyn std::error::Error>> {
        let dir_path = strategies_dir.as_ref();
        info!("🔍 Loading optimized strategies from: {}", dir_path.display());

        if !dir_path.exists() {
            warn!("Strategies directory does not exist: {}", dir_path.display());
            return Ok(0);
        }

        let mut loaded_count = 0;
        let entries = fs::read_dir(dir_path)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.load_single_strategy(&path) {
                    Ok(strategy) => {
                        info!("✅ Loaded strategy for {}", strategy.pair);
                        self.strategies.insert(strategy.pair.clone(), strategy);
                        loaded_count += 1;
                    }
                    Err(e) => {
                        error!("❌ Failed to load strategy from {}: {}", path.display(), e);
                    }
                }
            }
        }

        info!("🎯 Successfully loaded {} optimized strategies", loaded_count);
        Ok(loaded_count)
    }

    /// Load a single strategy file and convert to LiveStrategy
    fn load_single_strategy<P: AsRef<Path>>(&self, file_path: P) -> Result<LiveStrategy, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(file_path)?;
        let optimized: OptimizedStrategy = serde_json::from_str(&content)?;
        
        // Calculate grid levels based on current market price (will be updated with real data)
        let estimated_price = 1.0; // Placeholder - will be replaced with live price
        let grid_levels = self.calculate_static_grid_levels(estimated_price, optimized.grid_spacing, optimized.grid_levels);
        
        // Allocate capital per strategy (equal allocation for now)
        let capital_per_strategy = self.total_capital / 20.0; // Assuming max 20 strategies
        
        // CRITICAL: Initialize position-safe GridTrader
        let trading_config = TradingConfig {
            kraken_ws_url: "wss://ws.kraken.com".to_string(),
            trading_pair: optimized.trading_pair.clone(),
            grid_levels: optimized.grid_levels as usize,
            grid_spacing: optimized.grid_spacing,
            min_price_change: 0.001,
        };
        
        let market_config = MarketConfig::default();
        let grid_trader = GridTrader::with_capital(trading_config, market_config, capital_per_strategy);
        
        Ok(LiveStrategy {
            pair: optimized.trading_pair.clone(),
            config: optimized,
            grid_levels,
            grid_trader,  // Position-safe trader with capital tracking
            current_position: 0.0,  // Deprecated but kept for compatibility
            available_capital: capital_per_strategy,  // Deprecated but kept for compatibility
            active_orders: Vec::new(),
            market_data: None,
            recent_ohlc: Vec::new(),
            support_resistance: SupportResistanceLevels {
                support_levels: Vec::new(),
                resistance_levels: Vec::new(),
                last_calculated: Utc::now(),
            },
            volatility_metrics: VolatilityMetrics {
                atr: 0.0,
                std_dev: 0.0,
                bollinger_upper: 0.0,
                bollinger_lower: 0.0,
                last_updated: Utc::now(),
            },
        })
    }

    /// Calculate smart grid levels based on market conditions
    fn calculate_smart_grid_levels(&self, strategy: &LiveStrategy, current_price: f64) -> Vec<f64> {
        match self.grid_mode {
            GridMode::Static => self.calculate_static_grid_levels(current_price, strategy.config.grid_spacing, strategy.config.grid_levels),
            GridMode::VolatilityAdaptive => self.calculate_volatility_adaptive_grid(strategy, current_price),
            GridMode::SupportResistance => self.calculate_support_resistance_grid(strategy, current_price),
            GridMode::Fibonacci => self.calculate_fibonacci_grid(current_price, strategy.config.grid_levels),
            GridMode::TrendFollowing => self.calculate_trend_following_grid(strategy, current_price),
        }
    }

    /// Calculate static grid levels (original method)
    fn calculate_static_grid_levels(&self, current_price: f64, spacing: f64, levels: u32) -> Vec<f64> {
        let mut grid_levels = Vec::new();
        let half_levels = levels / 2;
        
        for i in 1..=half_levels {
            grid_levels.push(current_price * (1.0 - spacing * i as f64));
            grid_levels.push(current_price * (1.0 + spacing * i as f64));
        }
        
        grid_levels.sort_by(|a, b| a.partial_cmp(b).unwrap());
        grid_levels
    }

    /// Calculate volatility-adaptive grid using ATR
    fn calculate_volatility_adaptive_grid(&self, strategy: &LiveStrategy, current_price: f64) -> Vec<f64> {
        let mut grid_levels = Vec::new();
        let half_levels = strategy.config.grid_levels / 2;
        
        // Use ATR for dynamic spacing if available, otherwise use market volatility
        let volatility = if strategy.volatility_metrics.atr > 0.0 {
            strategy.volatility_metrics.atr / current_price // Convert to percentage
        } else if let Some(market_data) = &strategy.market_data {
            market_data.volatility.max(0.01) // Minimum 1% volatility
        } else {
            0.02 // Default 2% volatility
        };
        
        // Adaptive spacing based on volatility
        let base_spacing = volatility * 0.5; // Half of daily volatility per grid level
        
        for i in 1..=half_levels {
            let dynamic_spacing = base_spacing * (i as f64).sqrt(); // Exponential-like spacing
            grid_levels.push(current_price * (1.0 - dynamic_spacing));
            grid_levels.push(current_price * (1.0 + dynamic_spacing));
        }
        
        grid_levels.sort_by(|a, b| a.partial_cmp(b).unwrap());
        grid_levels
    }

    /// Calculate grid based on support and resistance levels
    fn calculate_support_resistance_grid(&self, strategy: &LiveStrategy, current_price: f64) -> Vec<f64> {
        let mut grid_levels = Vec::new();
        
        // If we have calculated support/resistance levels, use them
        if !strategy.support_resistance.support_levels.is_empty() && 
           !strategy.support_resistance.resistance_levels.is_empty() {
            
            // Place buy orders near support levels
            for &support in &strategy.support_resistance.support_levels {
                if support < current_price {
                    grid_levels.push(support * 1.001); // Slightly above support
                }
            }
            
            // Place sell orders near resistance levels
            for &resistance in &strategy.support_resistance.resistance_levels {
                if resistance > current_price {
                    grid_levels.push(resistance * 0.999); // Slightly below resistance
                }
            }
        }
        
        // If no S/R levels available, fall back to volatility-adaptive
        if grid_levels.is_empty() {
            return self.calculate_volatility_adaptive_grid(strategy, current_price);
        }
        
        grid_levels.sort_by(|a, b| a.partial_cmp(b).unwrap());
        grid_levels
    }

    /// Calculate Fibonacci-based grid levels
    fn calculate_fibonacci_grid(&self, current_price: f64, levels: u32) -> Vec<f64> {
        let mut grid_levels = Vec::new();
        let fibonacci_ratios = [0.236, 0.382, 0.5, 0.618, 0.786, 1.0, 1.272, 1.618, 2.618];
        
        let max_deviation = 0.1; // 10% maximum deviation
        
        for &ratio in &fibonacci_ratios {
            if grid_levels.len() >= levels as usize {
                break;
            }
            
            let deviation = max_deviation * ratio;
            if ratio <= 1.0 {
                grid_levels.push(current_price * (1.0 - deviation));
            }
            if grid_levels.len() < levels as usize {
                grid_levels.push(current_price * (1.0 + deviation));
            }
        }
        
        grid_levels.sort_by(|a, b| a.partial_cmp(b).unwrap());
        grid_levels.truncate(levels as usize);
        grid_levels
    }

    /// Calculate trend-following grid that adapts to price direction
    fn calculate_trend_following_grid(&self, strategy: &LiveStrategy, current_price: f64) -> Vec<f64> {
        let mut grid_levels = Vec::new();
        let half_levels = strategy.config.grid_levels / 2;
        
        // Determine trend direction from recent OHLC data
        let trend_direction = if strategy.recent_ohlc.len() >= 5 {
            let recent_closes: Vec<f64> = strategy.recent_ohlc
                .iter()
                .rev()
                .take(5)
                .map(|ohlc| ohlc.close)
                .collect();
            
            let sma_recent = recent_closes.iter().sum::<f64>() / recent_closes.len() as f64;
            
            if current_price > sma_recent * 1.01 {
                1.0 // Uptrend
            } else if current_price < sma_recent * 0.99 {
                -1.0 // Downtrend
            } else {
                0.0 // Sideways
            }
        } else {
            0.0 // No trend data available
        };
        
        let base_spacing = strategy.config.grid_spacing;
        
        for i in 1..=half_levels {
            let spacing_multiplier = if trend_direction > 0.0 {
                // In uptrend, tighter buy levels, wider sell levels
                if i <= half_levels / 2 {
                    base_spacing * 0.7 // Tighter buy levels
                } else {
                    base_spacing * 1.3 // Wider sell levels
                }
            } else if trend_direction < 0.0 {
                // In downtrend, wider buy levels, tighter sell levels
                if i <= half_levels / 2 {
                    base_spacing * 1.3 // Wider buy levels
                } else {
                    base_spacing * 0.7 // Tighter sell levels
                }
            } else {
                base_spacing // Normal spacing for sideways market
            };
            
            grid_levels.push(current_price * (1.0 - spacing_multiplier * i as f64));
            grid_levels.push(current_price * (1.0 + spacing_multiplier * i as f64));
        }
        
        grid_levels.sort_by(|a, b| a.partial_cmp(b).unwrap());
        grid_levels
    }

    /// Update strategy with new market data and recalculate grids
    pub fn update_strategy_market_data(&mut self, pair: &str, market_data: MarketData) {
        if let Some(strategy) = self.strategies.get_mut(pair) {
            // Update current price data
            let price_data = PriceData {
                bid: market_data.bid,
                ask: market_data.ask,
                last: market_data.price,
                volume: market_data.volume_24h,
                timestamp: DateTime::from_timestamp(market_data.timestamp as i64, 0).unwrap_or_else(Utc::now),
                volatility: market_data.volatility,
                high_24h: market_data.high_24h,
                low_24h: market_data.low_24h,
            };
            
            self.current_prices.insert(pair.to_string(), price_data);
            strategy.market_data = Some(market_data.clone());
            
            // We'll recalculate grid levels separately to avoid borrowing conflicts
            
            debug!("📈 Updated {} market data: price={:.4}, volatility={:.3}%", 
                   pair, market_data.price, market_data.volatility * 100.0);
        }
        
        // Grid levels will be recalculated periodically in the main loop
    }

    /// Recalculate grid levels for all strategies with current market data
    pub fn recalculate_all_grids(&mut self) {
        let pairs: Vec<String> = self.strategies.keys().cloned().collect();
        for pair in pairs {
            let (price, strategy_clone) = if let Some(strategy) = self.strategies.get(&pair) {
                if let Some(market_data) = &strategy.market_data {
                    (market_data.price, strategy.clone())
                } else {
                    continue;
                }
            } else {
                continue;
            };
            
            // Calculate new grid levels with cloned strategy to avoid borrowing conflicts
            let new_grid_levels = self.calculate_smart_grid_levels(&strategy_clone, price);
            
            // Update the actual strategy
            if let Some(strategy) = self.strategies.get_mut(&pair) {
                strategy.grid_levels = new_grid_levels;
            }
        }
    }

    /// Update strategy with new OHLC data and calculate technical indicators
    pub fn update_strategy_ohlc(&mut self, pair: &str, ohlc: OHLCData) {
        // First, update the OHLC data
        let should_calculate_indicators = if let Some(strategy) = self.strategies.get_mut(pair) {
            strategy.recent_ohlc.push(ohlc);
            if strategy.recent_ohlc.len() > 50 {
                strategy.recent_ohlc.remove(0);
            }
            strategy.recent_ohlc.len() >= 20
        } else {
            false
        };
        
        // Then calculate indicators if we have enough data
        if should_calculate_indicators {
            if let Some(strategy) = self.strategies.get_mut(pair) {
                Self::calculate_volatility_metrics_static(strategy);
                Self::calculate_support_resistance_static(strategy);
            }
        }
    }

    /// Calculate volatility metrics (ATR, Bollinger Bands, etc.)
    fn calculate_volatility_metrics_static(strategy: &mut LiveStrategy) {
        let ohlc_data = &strategy.recent_ohlc;
        if ohlc_data.len() < 14 {
            return;
        }
        
        // Calculate ATR (Average True Range)
        let mut true_ranges = Vec::new();
        for i in 1..ohlc_data.len() {
            let high_low = ohlc_data[i].high - ohlc_data[i].low;
            let high_close = (ohlc_data[i].high - ohlc_data[i-1].close).abs();
            let low_close = (ohlc_data[i].low - ohlc_data[i-1].close).abs();
            
            let true_range = high_low.max(high_close).max(low_close);
            true_ranges.push(true_range);
        }
        
        let atr = if true_ranges.len() >= 14 {
            true_ranges.iter().rev().take(14).sum::<f64>() / 14.0
        } else {
            0.0
        };
        
        // Calculate standard deviation of returns
        let closes: Vec<f64> = ohlc_data.iter().map(|d| d.close).collect();
        let returns: Vec<f64> = closes.windows(2)
            .map(|pair| (pair[1] / pair[0] - 1.0))
            .collect();
        
        let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance = returns.iter()
            .map(|r| (r - mean_return).powi(2))
            .sum::<f64>() / returns.len() as f64;
        let std_dev = variance.sqrt();
        
        // Calculate Bollinger Bands (20-period, 2 standard deviations)
        let sma_20 = if closes.len() >= 20 {
            closes.iter().rev().take(20).sum::<f64>() / 20.0
        } else {
            closes.last().copied().unwrap_or(0.0)
        };
        
        let bollinger_upper = sma_20 + (2.0 * std_dev * sma_20);
        let bollinger_lower = sma_20 - (2.0 * std_dev * sma_20);
        
        strategy.volatility_metrics = VolatilityMetrics {
            atr,
            std_dev,
            bollinger_upper,
            bollinger_lower,
            last_updated: Utc::now(),
        };
        
        debug!("📊 Updated volatility metrics for {}: ATR={:.4}, StdDev={:.3}%", 
               strategy.pair, atr, std_dev * 100.0);
    }

    /// Calculate support and resistance levels using pivot points and local extremes
    fn calculate_support_resistance_static(strategy: &mut LiveStrategy) {
        let ohlc_data = &strategy.recent_ohlc;
        if ohlc_data.len() < 10 {
            return;
        }
        
        let mut support_levels = Vec::new();
        let mut resistance_levels = Vec::new();
        
        // Find local minima (support) and maxima (resistance)
        for i in 2..ohlc_data.len()-2 {
            let current = &ohlc_data[i];
            let prev2 = &ohlc_data[i-2];
            let prev1 = &ohlc_data[i-1];
            let next1 = &ohlc_data[i+1];
            let next2 = &ohlc_data[i+2];
            
            // Check for local minimum (support)
            if current.low < prev2.low && current.low < prev1.low && 
               current.low < next1.low && current.low < next2.low {
                support_levels.push(current.low);
            }
            
            // Check for local maximum (resistance)
            if current.high > prev2.high && current.high > prev1.high && 
               current.high > next1.high && current.high > next2.high {
                resistance_levels.push(current.high);
            }
        }
        
        // Remove levels that are too close to each other (within 0.5%)
        support_levels.sort_by(|a, b| a.partial_cmp(b).unwrap());
        resistance_levels.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let mut filtered_support = Vec::new();
        let mut filtered_resistance = Vec::new();
        
        for &level in &support_levels {
            if filtered_support.is_empty() {
                filtered_support.push(level);
            } else if let Some(last_level) = filtered_support.last() {
                if (level - last_level).abs() / level > 0.005 {
                    filtered_support.push(level);
                }
            }
        }
        
        for &level in &resistance_levels {
            if filtered_resistance.is_empty() {
                filtered_resistance.push(level);
            } else if let Some(last_level) = filtered_resistance.last() {
                if (level - last_level).abs() / level > 0.005 {
                    filtered_resistance.push(level);
                }
            }
        }
        
        // Keep only the most recent and relevant levels (last 5 of each)
        filtered_support.truncate(5);
        filtered_resistance.truncate(5);
        
        strategy.support_resistance = SupportResistanceLevels {
            support_levels: filtered_support.clone(),
            resistance_levels: filtered_resistance.clone(),
            last_calculated: Utc::now(),
        };
        
        debug!("🎯 Updated S/R levels for {}: {} support, {} resistance", 
               strategy.pair, filtered_support.len(), filtered_resistance.len());
    }

    /// Process real-time WebSocket messages for market data (non-blocking)
    pub async fn process_websocket_messages(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Collect messages first to avoid borrowing conflicts
        let mut market_updates = Vec::new();
        let mut ohlc_updates = Vec::new();
        
        if let Some(ws_client) = &mut self.ws_client {
            // Process only one message per call to avoid blocking
            if let Some(message) = ws_client.ws_receiver.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        if let Ok(data) = serde_json::from_str::<Value>(&text) {
                            // Update simulation engine with order book data
                            if let Some(sim_engine) = &mut self.simulation_engine {
                                // Try to parse as order book update
                                if let Some(_order_book) = crate::clients::kraken_ws::parse_kraken_orderbook(&data) {
                                    if let Some(pair_name) = data.get(3).and_then(|p| p.as_str()) {
                                        sim_engine.update_from_kraken_ws(pair_name, &data);
                                        debug!("📖 Updated simulation order book for {}", pair_name);
                                    }
                                }
                            }
                            
                            // Handle different message types
                            if let Some(market_data) = crate::clients::kraken_ws::parse_kraken_ticker(&data) {
                                market_updates.push(market_data);
                            } else if let Some(ohlc_data) = crate::clients::kraken_ws::parse_kraken_ohlc(&data) {
                                // Extract pair from the message
                                if let Some(pair) = data.get(3).and_then(|p| p.as_str()) {
                                    ohlc_updates.push((pair.to_string(), ohlc_data));
                                }
                            } else {
                                // Handle other events
                                crate::clients::kraken_ws::handle_kraken_event(&data);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        warn!("WebSocket connection closed");
                        return Err("WebSocket connection closed".into());
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        return Err(e.into());
                    }
                    _ => {}
                }
            }
        }
        
        // Apply updates after releasing WebSocket borrow
        for market_data in market_updates {
            let pair = market_data.pair.clone();
            self.update_strategy_market_data(&pair, market_data);
        }
        for (pair, ohlc_data) in ohlc_updates {
            self.update_strategy_ohlc(&pair, ohlc_data);
        }
        
        Ok(())
    }

    /// Start the live trading simulation (indefinite)
    pub async fn start_simulation(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🚀 Starting live trading simulation with {} strategies", self.strategies.len());
        self.run_trading_loop(None).await
    }

    /// Start the live trading simulation with duration limit
    pub async fn start_simulation_with_duration(&mut self, duration: Duration) -> Result<(), Box<dyn std::error::Error>> {
        info!("🚀 Starting live trading simulation with {} strategies for {:.1} hours", 
              self.strategies.len(), duration.as_secs_f64() / 3600.0);
        self.run_trading_loop(Some(duration)).await
    }

    /// Internal trading loop with optional duration
    async fn run_trading_loop(&mut self, duration: Option<Duration>) -> Result<(), Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        
        // Connect to real market data
        self.connect_market_data().await?;
        self.subscribe_market_data().await?;
        info!("🔗 Connected to real market data feeds");
        
        // Main trading loop
        loop {
            // Check if we should stop due to duration limit
            if let Some(duration) = duration {
                if start_time.elapsed() >= duration {
                    info!("⏰ Trading session completed after {:.1} hours", duration.as_secs_f64() / 3600.0);
                    break;
                }
            }

            // 1. Process real-time WebSocket messages
            tokio::time::timeout(Duration::from_millis(50), self.process_websocket_messages()).await.ok();
            
            // 2. Fallback price updates for any missing data
            self.update_live_prices().await?;
            
            // 3. Periodically recalculate smart grids (every 10 seconds)
            if start_time.elapsed().as_secs() % 10 == 0 {
                self.recalculate_all_grids();
            }
            
            // 4. Check for grid triggers
            self.check_grid_triggers().await;
            
            // 3. Process pending orders
            self.process_pending_orders().await;
            
            // 4. Update portfolio state
            self.update_portfolio_state();
            
            // 5. Log performance metrics
            self.log_performance_update();
            
            // 6. Sleep before next iteration
            sleep(Duration::from_millis(100)).await; // 10 updates per second
        }
        
        Ok(())
    }

    async fn update_live_prices(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for pair in self.strategies.keys() {
            match self.fetch_live_price(pair).await {
                Ok(price_data) => {
                    self.current_prices.insert(pair.clone(), price_data);
                }
                Err(e) => {
                    warn!("⚠️  Failed to fetch price for {}: {}", pair, e);
                }
            }
        }
        Ok(())
    }

    async fn fetch_live_price(&self, pair: &str) -> Result<PriceData, Box<dyn std::error::Error>> {
        // If we have real market data from WebSocket, use it
        if let Some(price_data) = self.current_prices.get(pair) {
            return Ok(price_data.clone());
        }
        
        // If no WebSocket data available, fetch from REST API as fallback
        let url = format!("https://api.kraken.com/0/public/Ticker?pair={}", pair);
        let response = self.kraken_client.get(&url).send().await?;
        let data: serde_json::Value = response.json().await?;
        
        if let Some(ticker_data) = data.get("result").and_then(|r| r.as_object()) {
            if let Some((_, ticker)) = ticker_data.iter().next() {
                let ask_str = ticker.get("a").and_then(|a| a.get(0)).and_then(|p| p.as_str())
                    .ok_or("Missing ask price")?;
                let bid_str = ticker.get("b").and_then(|b| b.get(0)).and_then(|p| p.as_str())
                    .ok_or("Missing bid price")?;
                let last_str = ticker.get("c").and_then(|c| c.get(0)).and_then(|p| p.as_str())
                    .ok_or("Missing last price")?;
                let volume_str = ticker.get("v").and_then(|v| v.get(1)).and_then(|p| p.as_str())
                    .unwrap_or("0.0");
                let high_str = ticker.get("h").and_then(|h| h.get(1)).and_then(|p| p.as_str())
                    .unwrap_or(last_str);
                let low_str = ticker.get("l").and_then(|l| l.get(1)).and_then(|p| p.as_str())
                    .unwrap_or(last_str);
                
                let ask = ask_str.parse::<f64>()?;
                let bid = bid_str.parse::<f64>()?;
                let last = last_str.parse::<f64>()?;
                let volume = volume_str.parse::<f64>().unwrap_or(0.0);
                let high_24h = high_str.parse::<f64>().unwrap_or(last);
                let low_24h = low_str.parse::<f64>().unwrap_or(last);
                
                let volatility = if high_24h > low_24h {
                    (high_24h - low_24h) / last
                } else {
                    0.01
                };
                
                return Ok(PriceData {
                    bid,
                    ask,
                    last,
                    volume,
                    timestamp: Utc::now(),
                    volatility,
                    high_24h,
                    low_24h,
                });
            }
        }
        
        Err(format!("Failed to fetch price data for {}", pair).into())
    }

    async fn check_grid_triggers(&mut self) {
        let mut orders_to_place = Vec::new();
        
        // First pass: collect orders to place (avoid borrowing conflicts)
        {
            let strategies = &self.strategies;
            let current_prices = &self.current_prices;
            
            for (pair, strategy) in strategies {
                if let Some(price_data) = current_prices.get(pair) {
                    let mut updated_strategy = strategy.clone();
                    self.update_grid_levels(&mut updated_strategy, price_data.last);
                    
                    // Log grid levels periodically for debugging (every 60 seconds)
                    static mut DEBUG_COUNTER: u32 = 0;
                    unsafe {
                        DEBUG_COUNTER += 1;
                        if DEBUG_COUNTER % 600 == 1 {
                            debug!("🎯 {}: Price £{:.6} | Grid levels: {} | Position: {}", 
                                pair, price_data.last, updated_strategy.grid_levels.len(), strategy.current_position);
                        }
                    }
                    
                    // Check for buy/sell triggers - more permissive logic
                    for &level in &updated_strategy.grid_levels {
                        // Buy when price is below grid level (support)
                        if level < price_data.last {
                            let distance_pct = (price_data.last - level) / level;
                            // Trigger buy if price is within 1% above the level
                            if distance_pct <= 0.01 && !self.has_pending_order_at_level(pair, level, "buy") {
                                let order_size = strategy.available_capital * 0.05; // 5% of available capital
                                if order_size >= 1.0 {
                                    orders_to_place.push((pair.clone(), "buy".to_string(), level, order_size));
                                }
                            }
                        } 
                        // Sell when price is above grid level (resistance) AND we have position
                        else if level > price_data.last && strategy.current_position > 0.0 {
                            let distance_pct = (level - price_data.last) / price_data.last;
                            // Trigger sell if price is within 1% below the level
                            if distance_pct <= 0.01 && !self.has_pending_order_at_level(pair, level, "sell") {
                                let order_size = (strategy.current_position * 0.2).max(1.0); // 20% of position, min 1 unit
                                orders_to_place.push((pair.clone(), "sell".to_string(), level, order_size));
                            }
                        }
                    }
                }
            }
        }
        
        // Second pass: place orders
        for (pair, side, price, quantity) in orders_to_place {
            self.place_simulated_order(&pair, &side, price, quantity).await;
        }
    }

    fn update_grid_levels(&self, strategy: &mut LiveStrategy, current_price: f64) {
        // CRITICAL: Update GridTrader with new price and get signals
        let signal = strategy.grid_trader.update_with_price(current_price);
        
        // Legacy grid level recalculation (kept for visualization)
        let spacing = strategy.config.grid_spacing;
        let levels = strategy.config.grid_levels;
        strategy.grid_levels = self.calculate_static_grid_levels(current_price, spacing, levels);
        
        // Log position summary
        if strategy.grid_trader.should_log_price(current_price, 0.001) {
            debug!("📊 {}: {}", strategy.pair, strategy.grid_trader.get_position_summary(current_price));
            strategy.grid_trader.update_logged_price(current_price);
        }
        
        // Handle trading signals from position-safe GridTrader
        match signal {
            GridSignal::Buy(_) | GridSignal::Sell(_) => {
                debug!("🎯 GridTrader generated signal: {:?}", signal);
            }
            GridSignal::None => {}
        }
    }

    fn has_pending_order_at_level(&self, pair: &str, level: f64, side: &str) -> bool {
        self.strategies.get(pair)
            .map(|s| s.active_orders.iter()
                .any(|order| order.side == side && 
                    (order.price - level).abs() < level * 0.001 && // Within 0.1%
                    matches!(order.status, OrderStatus::Pending)
                ))
            .unwrap_or(false)
    }

    async fn place_simulated_order(&mut self, pair: &str, side: &str, price: f64, quantity: f64) {
        // CRITICAL: Check portfolio-level risk limits first
        if let Err(risk_error) = self.check_portfolio_risk() {
            warn!("🚨 RISK LIMIT VIOLATION: {}", risk_error);
            warn!("⛔ Order blocked: {} {} {} @ £{:.6}", side.to_uppercase(), quantity, pair, price);
            return;
        }
        
        // Apply realistic constraints
        if quantity < 1.0 { // Minimum order size
            debug!("⚠️  Order too small: {:.2} units for {} (min: 1.0)", quantity, pair);
            return;
        }

        let order_id = Uuid::new_v4().to_string();
        let order = SimulatedOrder {
            id: order_id.clone(),
            pair: pair.to_string(),
            side: side.to_string(),
            price,
            quantity,
            timestamp: Utc::now(),
            status: OrderStatus::Pending,
        };

        // Add to strategy's active orders
        if let Some(strategy) = self.strategies.get_mut(pair) {
            strategy.active_orders.push(order.clone());
            info!("📝 Placed {} order: {:.2} {} @ £{:.6} (ID: {})", 
                  side.to_uppercase(), quantity, pair, price, &order_id[..8]);
        } else {
            warn!("⚠️  Strategy not found for {}", pair);
        }
    }

    async fn process_pending_orders(&mut self) {
        let mut orders_to_process = Vec::new();
        
        // Collect orders that might execute
        for (pair, strategy) in &self.strategies {
            if let Some(price_data) = self.current_prices.get(pair) {
                for order in &strategy.active_orders {
                    if matches!(order.status, OrderStatus::Pending)
                        && self.should_execute_order(order, price_data) {
                            orders_to_process.push((pair.clone(), order.clone()));
                        }
                }
            }
        }

        // Process executions
        for (pair, order) in orders_to_process {
            self.execute_simulated_order(&pair, &order).await;
        }
    }

    fn should_execute_order(&self, order: &SimulatedOrder, price_data: &PriceData) -> bool {
        // Simulate realistic execution conditions
        let mut rng = thread_rng();
        
        // Check if price crosses order level
        let crosses_level = match order.side.as_str() {
            "buy" => price_data.ask <= order.price, // Can buy at ask price <= order price
            "sell" => price_data.bid >= order.price, // Can sell at bid price >= order price
            _ => false,
        };

        if !crosses_level {
            return false;
        }

        // Simulate liquidity and execution probability (90% fill rate)
        let fill_probability = 0.9;
        let _execution_delay = Duration::from_millis(rng.gen_range(50..200)); // 50-200ms delay
        
        // For simulation, we'll execute immediately but log the delay
        rng.gen::<f64>() < fill_probability
    }

    async fn execute_simulated_order(&mut self, pair: &str, order: &SimulatedOrder) {
        // Use simulation engine if enabled and available
        if self.use_simulation_engine {
            if let Some(sim_engine) = &mut self.simulation_engine {
                if sim_engine.is_ready(pair) {
                    // Execute using simulation engine
                    match sim_engine.execute_live_order(order) {
                        Ok(exec_result) => {
                            use crate::simulation::execution_simulator::ExecutionStatus;
                            
                            if exec_result.status == ExecutionStatus::Success 
                                || exec_result.status == ExecutionStatus::PartialFill {
                                // Create trade record
                                let trade = SimulatedTrade {
                                    id: order.id.clone(),
                                    pair: pair.to_string(),
                                    side: order.side.clone(),
                                    price: exec_result.average_price,
                                    quantity: exec_result.total_filled,
                                    fee: exec_result.total_fees,
                                    timestamp: exec_result.timestamp,
                                    execution_delay_ms: exec_result.execution_time_ms,
                                    slippage: exec_result.total_slippage,
                                };

                                // Update portfolio
                                self.update_portfolio_from_trade(&trade);
                                
                                // CRITICAL: Update GridTrader position tracking
                                if let Some(strategy) = self.strategies.get_mut(pair) {
                                    let signal = match trade.side.as_str() {
                                        "buy" => GridSignal::Buy(order.price),
                                        "sell" => GridSignal::Sell(order.price),
                                        _ => GridSignal::None,
                                    };
                                    
                                    // Use GridTrader's position-safe execution
                                    strategy.grid_trader.execute_trade(&signal, trade.price);
                                    
                                    // Legacy tracking (deprecated but kept for compatibility)
                                    let trade_value = trade.price * trade.quantity;
                                    match trade.side.as_str() {
                                        "buy" => {
                                            strategy.current_position += trade.quantity;
                                            strategy.available_capital -= trade_value + trade.fee;
                                        }
                                        "sell" => {
                                            strategy.current_position -= trade.quantity;
                                            strategy.available_capital += trade_value - trade.fee;
                                        }
                                        _ => {}
                                    }

                                    // Update order status
                                    for active_order in &mut strategy.active_orders {
                                        if active_order.id == order.id {
                                            active_order.status = OrderStatus::Filled;
                                            break;
                                        }
                                    }
                                }

                                // Log trade
                                self.log_trade(&trade);
                                self.trade_history.push(trade.clone());

                                info!("✅ EXECUTED (SIM ENGINE): {} {} {} @ £{:.6} | Fee: £{:.2} | Slippage: £{:.2} | Latency: {}ms", 
                                      trade.side.to_uppercase(), trade.quantity, trade.pair, 
                                      trade.price, trade.fee, trade.slippage, trade.execution_delay_ms);
                                return;
                            } else {
                                warn!("⚠️ Order execution failed in simulation engine: {:?}", exec_result.status);
                            }
                        }
                        Err(e) => {
                            warn!("⚠️ Simulation engine error: {}", e);
                        }
                    }
                }
            }
        }

        // Fallback to old simulation method if engine not available
        let mut rng = thread_rng();
        
        // Calculate realistic execution price with slippage
        let price_data = self.current_prices.get(pair).unwrap();
        let slippage_bps = rng.gen_range(1.0..5.0); // 1-5 basis points slippage
        let slippage_factor = slippage_bps / 10000.0;
        
        let execution_price = match order.side.as_str() {
            "buy" => price_data.ask * (1.0 + slippage_factor), // Buy at higher price
            "sell" => price_data.bid * (1.0 - slippage_factor), // Sell at lower price
            _ => order.price,
        };

        // Calculate fees (Kraken taker fee ~0.26%)
        let fee_rate = 0.0026;
        let trade_value = execution_price * order.quantity;
        let fees = trade_value * fee_rate;
        let slippage_cost = (execution_price - order.price).abs() * order.quantity;

        // Create executed trade
        let trade = SimulatedTrade {
            id: order.id.clone(),
            pair: pair.to_string(),
            side: order.side.clone(),
            price: execution_price,
            quantity: order.quantity,
            fee: fees,
            timestamp: Utc::now(),
            execution_delay_ms: rng.gen_range(50..200),
            slippage: slippage_cost,
        };

        // Update portfolio
        self.update_portfolio_from_trade(&trade);
        
        // Update strategy position
        if let Some(strategy) = self.strategies.get_mut(pair) {
            match trade.side.as_str() {
                "buy" => {
                    strategy.current_position += trade.quantity;
                    strategy.available_capital -= trade_value + fees;
                }
                "sell" => {
                    strategy.current_position -= trade.quantity;
                    strategy.available_capital += trade_value - fees;
                }
                _ => {}
            }

            // Update order status
            for active_order in &mut strategy.active_orders {
                if active_order.id == order.id {
                    active_order.status = OrderStatus::Filled;
                    break;
                }
            }
        }

        // Log trade
        self.log_trade(&trade);
        self.trade_history.push(trade.clone());

        info!("✅ EXECUTED: {} {} {} @ £{:.6} | Fee: £{:.2} | Slippage: £{:.2}", 
              trade.side.to_uppercase(), trade.quantity, trade.pair, 
              trade.price, trade.fee, trade.slippage);
    }

    fn update_portfolio_from_trade(&mut self, trade: &SimulatedTrade) {
        let trade_value = trade.price * trade.quantity;
        
        match trade.side.as_str() {
            "buy" => {
                self.portfolio.cash_balance -= trade_value + trade.fee;
                *self.portfolio.positions.entry(trade.pair.clone()).or_insert(0.0) += trade.quantity;
            }
            "sell" => {
                self.portfolio.cash_balance += trade_value - trade.fee;
                *self.portfolio.positions.entry(trade.pair.clone()).or_insert(0.0) -= trade.quantity;
            }
            _ => {}
        }
        
        self.portfolio.total_fees_paid += trade.fee;
    }

    fn update_portfolio_state(&mut self) {
        // Calculate unrealized P&L
        let mut total_unrealized_pnl = 0.0;
        
        for (pair, position) in &self.portfolio.positions {
            if let Some(price_data) = self.current_prices.get(pair) {
                let position_value = *position * price_data.last;
                // This is a simplified calculation - in reality we'd track cost basis
                total_unrealized_pnl += position_value;
            }
        }
        
        self.portfolio.unrealized_pnl = total_unrealized_pnl;
        
        // Log portfolio state every 30 seconds
        if self.last_portfolio_update.elapsed() > Duration::from_secs(30) {
            self.log_portfolio_state();
            self.last_portfolio_update = Instant::now();
        }
    }

    fn log_trade(&self, trade: &SimulatedTrade) {
        // Log to CSV file
        let log_entry = format!(
            "{},{},{},{},{:.6},{:.4},{:.2},{:.2},{}\n",
            trade.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            trade.pair,
            trade.side,
            trade.quantity,
            trade.price,
            trade.fee,
            trade.slippage,
            trade.execution_delay_ms,
            trade.id
        );
        
        // Ensure logs directory exists
        if let Some(parent) = std::path::Path::new(&self.trade_log_file).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.trade_log_file) 
        {
            // Write header if file is new
            if file.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
                let _ = writeln!(file, "timestamp,pair,side,quantity,price,fee,slippage,delay_ms,order_id");
            }
            let _ = write!(file, "{}", log_entry);
        }
    }

    fn log_portfolio_state(&self) {
        let summary = self.get_portfolio_summary();
        
        let log_entry = format!(
            "{},{:.2},{:.2},{:.2},{:.2},{:.2},{},{}\n",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            summary.total_value,
            summary.cash_balance,
            summary.unrealized_pnl,
            summary.realized_pnl,
            summary.total_return,
            summary.total_trades,
            summary.active_strategies
        );
        
        // Ensure logs directory exists
        if let Some(parent) = std::path::Path::new(&self.portfolio_log_file).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.portfolio_log_file) 
        {
            // Write header if file is new
            if file.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
                let _ = writeln!(file, "timestamp,total_value,cash_balance,unrealized_pnl,realized_pnl,total_return_pct,total_trades,active_strategies");
            }
            let _ = write!(file, "{}", log_entry);
        }
    }

    fn log_performance_update(&self) {
        // Log summary every 600 iterations (60 seconds at 10 Hz)
        static mut COUNTER: u32 = 0;
        unsafe {
            COUNTER += 1;
            if COUNTER % 600 == 0 {
                let summary = self.get_portfolio_summary();
                info!("💰 Portfolio: £{:.2} ({:+.2}%) | Trades: {} | Active Orders: {}", 
                    summary.total_value,
                    summary.total_return,
                    summary.total_trades,
                    summary.active_orders);
                
                // Log simulation engine stats if enabled
                if self.use_simulation_engine {
                    if let Some(sim_engine) = &self.simulation_engine {
                        info!("🎮 {}", sim_engine.get_stats());
                    }
                }
                
                // Log top performing pairs
                if !self.trade_history.is_empty() {
                    let mut pair_trades: HashMap<String, Vec<&SimulatedTrade>> = HashMap::new();
                    for trade in &self.trade_history {
                        pair_trades.entry(trade.pair.clone()).or_default().push(trade);
                    }
                    
                    info!("📊 Active pairs: {}", pair_trades.len());
                }
            }
        }
    }

    fn count_active_orders(&self) -> usize {
        self.strategies.values()
            .map(|s| s.active_orders.iter()
                .filter(|o| matches!(o.status, OrderStatus::Pending))
                .count())
            .sum()
    }

    /// Get current portfolio summary
    pub fn get_portfolio_summary(&self) -> PortfolioSummary {
        let total_value = self.portfolio.cash_balance + self.portfolio.unrealized_pnl;
        let total_return = (total_value - self.total_capital) / self.total_capital * 100.0;
        
        PortfolioSummary {
            total_value,
            cash_balance: self.portfolio.cash_balance,
            unrealized_pnl: self.portfolio.unrealized_pnl,
            realized_pnl: self.portfolio.realized_pnl,
            total_return,
            active_strategies: self.strategies.len(),
            total_trades: self.trade_history.len(),
            total_fees: self.portfolio.total_fees_paid,
            active_orders: self.count_active_orders(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PortfolioSummary {
    pub total_value: f64,
    pub cash_balance: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub total_return: f64,
    pub active_strategies: usize,
    pub total_trades: usize,
    pub total_fees: f64,
    pub active_orders: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::File;
    use std::io::Write;

    #[tokio::test]
    async fn test_load_optimized_strategies() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_strategy.json");
        
        let strategy = OptimizedStrategy {
            trading_pair: "TESTGBP".to_string(),
            grid_levels: 10,
            grid_spacing: 0.02,
            expected_return: 0.15,
            total_trades: 5,
            win_rate: 0.6,
            sharpe_ratio: 1.2,
            max_drawdown: 0.05,
            total_fees: 10.0,
            markov_confidence: 0.75,
            generated_at: Utc::now(),
        };
        
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "{}", serde_json::to_string_pretty(&strategy).unwrap()).unwrap();
        
        let mut engine = LiveTradingEngine::new(10000.0);
        let loaded_count = engine.load_optimized_strategies(dir.path()).unwrap();
        
        assert_eq!(loaded_count, 1);
        assert!(engine.strategies.contains_key("TESTGBP"));
    }
}