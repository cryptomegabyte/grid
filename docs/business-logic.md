# Business Logic

## 🧠 Autonomous Optimization Strategy

### Intelligent Parameter Discovery

The autonomous optimization system transforms traditional manual parameter tuning into an intelligent, data-driven discovery process:

```rust
// Multi-dimensional parameter optimization
pub struct OptimizationConfig {
    pub grid_levels: ParameterRange<usize>,      // 5-20 levels
    pub grid_spacing: ParameterRange<f64>,       // 0.001-0.100 (0.1%-10%)
    pub timeframes: Vec<String>,                 // ["15m", "30m", "1h", "2h", "4h", "1d"]
    pub risk_management: RiskManagementConfig,   // Dynamic risk parameters
}
```

### Advanced Grid Strategies

The system automatically evaluates multiple sophisticated grid configurations:

**1. Fibonacci Grid Strategy:**
```rust
// Golden ratio spacing for natural support/resistance
spacing_ratios = [1.0, 1.618, 2.618, 4.236, 6.854, ...]
levels = base_price * (1.0 ± spacing * ratio)
```

**2. Volatility-Adjusted Grid:**
```rust
// Dynamic spacing based on market volatility
volatility = calculate_volatility(price_history, period=20)
adaptive_spacing = base_spacing * (1.0 + volatility_factor)
```

**3. Trend-Following Grid:**
```rust
// Asymmetric grids that adapt to market direction
trend_strength = calculate_trend_strength(price_history)
if trend_strength > 0.6 {
    buy_levels *= 0.8;  // Fewer buy levels in uptrend
    sell_levels *= 1.2; // More sell levels in uptrend
}
```

### Multi-Objective Optimization

The system balances multiple objectives simultaneously:

```rust
composite_score = 0.4 * normalize_return(total_return) +
                 0.3 * normalize_sharpe(sharpe_ratio) +
                 0.2 * normalize_drawdown(max_drawdown) +
                 0.1 * normalize_trades(trade_frequency)
```

## 🎯 Traditional Grid Trading Strategy

### Core Concept

Grid trading is a **quantitative trading strategy** that places buy and sell orders at regular intervals around a base price, creating a "grid" of orders. The strategy profits from market volatility by capturing small price movements.

```
Price Level    Action        Logic
───────────────────────────────────────
£2.16  ↑      SELL         Take profit
£2.15  ↑      SELL         Take profit  
£2.14  ↑      SELL         Take profit
£2.13  ←─     CURRENT      Reference point
£2.12  ↓      BUY          Buy the dip
£2.11  ↓      BUY          Buy the dip
£2.10  ↓      BUY          Buy the dip
```

### Mathematical Foundation

**Grid Level Calculation:**
```rust
// For a base price P and spacing S:
buy_levels = [P - S*1, P - S*2, P - S*3, ...]
sell_levels = [P + S*1, P + S*2, P + S*3, ...]
```

**Signal Generation:**
- **BUY Signal:** When price crosses below a buy level
- **SELL Signal:** When price crosses above a sell level
- **No Signal:** When price moves within existing grid bounds

## 🔬 Market State Analysis

### Three-State Market Model

Our system classifies market conditions into three distinct states:

```rust
pub enum MarketState {
    TrendingUp,    // Strong upward momentum
    TrendingDown,  // Strong downward momentum  
    Ranging,       // Sideways/consolidating movement
}
```

### State Detection Algorithm

**Moving Average Analysis:**
```rust
// Simplified logic
if short_ma > long_ma * (1.0 + trend_threshold) {
    MarketState::TrendingUp
} else if short_ma < long_ma * (1.0 - trend_threshold) {
    MarketState::TrendingDown
} else {
    MarketState::Ranging
}
```

**Why This Matters:**
- **Ranging markets:** Grid trading performs best
- **Trending markets:** Risk of losses on one side
- **Volatile markets:** More opportunities for profit

## ⚡ Signal Generation Logic

### Primary Signal Detection

**Price Level Crossing:**
```rust
pub fn generate_signal(&mut self, current_price: f64) -> GridSignal {
    // Check for buy signals (price drops to buy level)
    for &buy_level in &self.buy_levels {
        if current_price <= buy_level && self.last_price > buy_level {
            return GridSignal::Buy(buy_level);
        }
    }
    
    // Check for sell signals (price rises to sell level)  
    for &sell_level in &self.sell_levels {
        if current_price >= sell_level && self.last_price < sell_level {
            return GridSignal::Sell(sell_level);
        }
    }
    
    GridSignal::None
}
```

### Anti-Noise Filtering

**Minimum Price Movement:**
```rust
const MIN_PRICE_CHANGE: f64 = 0.001; // 0.1% minimum change

// Ignore micro-movements that could be noise
if (current_price - last_price).abs() < MIN_PRICE_CHANGE {
    return GridSignal::None;
}
```

**Duplicate Signal Prevention:**
```rust
// Prevent repeated signals at the same level
if signal_matches_recent_signal(&new_signal, &self.recent_signals) {
    return GridSignal::None;
}
```

## 📊 Vectorized Backtesting Logic

### High-Performance Data Processing

**NDArray Operations:**
```rust
use ndarray::Array1;

// Vectorized price analysis
let prices: Array1<f64> = Array1::from(historical_data);
let returns = prices.slice(s![1..]).to_owned() / prices.slice(s![..prices.len()-1]) - 1.0;
let volatility = returns.std(0.0);
```

**Signal Generation at Scale:**
```rust
// Process entire price series at once
for (i, &price) in prices.iter().enumerate() {
    if let Some(signal) = grid_trader.process_price(price, timestamps[i]) {
        signals.push(signal);
    }
}
```

### Portfolio Simulation

**Position Tracking:**
```rust
struct Portfolio {
    cash_balance: f64,
    position_size: f64,
    total_fees: f64,
    trade_history: Vec<Trade>,
}

impl Portfolio {
    fn execute_trade(&mut self, signal: GridSignal, price: f64) {
        match signal {
            GridSignal::Buy(level) => {
                let trade_size = self.calculate_position_size(level);
                let fee = trade_size * TRADING_FEE_RATE;
                self.cash_balance -= trade_size + fee;
                self.position_size += trade_size / price;
                self.total_fees += fee;
            },
            GridSignal::Sell(level) => {
                // Similar logic for sell orders
            },
            GridSignal::None => {} // No action
        }
    }
}
```

## 🎯 Markov Chain Market Analysis

### Market Regime Detection

**State Transition Matrix:**
```
Current\Next   Up    Down  Range
─────────────────────────────────
Trending Up   0.7   0.1   0.2
Trending Down 0.1   0.7   0.2  
Ranging       0.3   0.3   0.4
```

**Confidence Calculation:**
```rust
pub fn calculate_confidence(&self, current_state: MarketState) -> f64 {
    let transitions = &self.transition_matrix[current_state as usize];
    let max_probability = transitions.iter().max_by(|a, b| a.partial_cmp(b).unwrap());
    max_probability.unwrap_or(&0.0) * 100.0
}
```

### Practical Application

**Risk Assessment:**
- **High confidence (>60%):** Execute trades with full position size
- **Medium confidence (40-60%):** Reduce position size by 50%
- **Low confidence (<40%):** Consider avoiding trades

## 💰 Risk Management Logic

### Position Sizing

**Kelly Criterion Implementation:**
```rust
fn calculate_optimal_position_size(
    win_rate: f64, 
    avg_win: f64, 
    avg_loss: f64,
    account_balance: f64
) -> f64 {
    let kelly_fraction = (win_rate * avg_win - (1.0 - win_rate) * avg_loss) / avg_win;
    account_balance * kelly_fraction.max(0.0).min(0.25) // Cap at 25%
}
```

### Stop-Loss Logic

**Maximum Drawdown Protection:**
```rust
const MAX_DRAWDOWN: f64 = 0.05; // 5% maximum loss

if current_loss_percentage > MAX_DRAWDOWN {
    // Close all positions and halt trading
    emergency_stop_trading();
}
```

### Trading Cost Modeling

**Realistic Fee Structure:**
```rust
const MAKER_FEE: f64 = 0.0016;  // 0.16% for maker orders
const TAKER_FEE: f64 = 0.0026;  // 0.26% for taker orders
const SLIPPAGE: f64 = 0.001;    // 0.1% average slippage

fn calculate_total_cost(trade_amount: f64, is_maker: bool) -> f64 {
    let fee_rate = if is_maker { MAKER_FEE } else { TAKER_FEE };
    trade_amount * (fee_rate + SLIPPAGE)
}
```

## 🔄 Real-Time Decision Making

### WebSocket Event Processing

**Price Update Handler:**
```rust
async fn handle_price_update(&mut self, price_data: TickerData) {
    // 1. Update market state
    self.market_analyzer.update_state(&price_data);
    
    // 2. Generate trading signal
    let signal = self.grid_trader.process_price(price_data.price);
    
    // 3. Risk assessment
    let confidence = self.markov_analyzer.get_confidence();
    
    // 4. Execute trade (if conditions met)
    if confidence > MIN_CONFIDENCE_THRESHOLD {
        self.execute_signal(signal, price_data.price).await;
    }
}
```

### Performance Optimization

**Event Batching:**
```rust
// Process multiple price updates in batches
let batch_size = 10;
let mut price_batch = Vec::with_capacity(batch_size);

while let Some(price) = price_stream.next().await {
    price_batch.push(price);
    
    if price_batch.len() >= batch_size {
        process_price_batch(&mut price_batch).await;
        price_batch.clear();
    }
}
```

This business logic forms the foundation of a robust, mathematically-sound grid trading system capable of operating in live market conditions.