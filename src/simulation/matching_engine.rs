// Order Matching Engine
// Simulates realistic order matching using the local order book

use crate::simulation::order_book::LocalOrderBook;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Order to be matched against the order book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedOrder {
    pub id: String,
    pub pair: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: Option<f64>, // None for market orders
    pub quantity: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
    PostOnly, // Limit order that only adds liquidity
}

/// Result of order matching
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub order_id: String,
    pub fills: Vec<FillInfo>,
    pub status: OrderStatus,
    pub total_filled: f64,
    pub average_price: f64,
    pub remaining_quantity: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillInfo {
    pub price: f64,
    pub quantity: f64,
    pub timestamp: DateTime<Utc>,
    pub is_maker: bool, // True if order provided liquidity
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    FullyFilled,
    PartiallyFilled,
    Rejected,
    PostedToBook, // For post-only orders
}

/// Order matching engine with realistic market microstructure
pub struct OrderMatchingEngine {
    /// Queue of pending orders (for time priority)
    #[allow(dead_code)]
    order_queue: VecDeque<SimulatedOrder>,
    /// Configuration
    config: MatchingConfig,
}

#[derive(Debug, Clone)]
pub struct MatchingConfig {
    /// Minimum order size
    pub min_order_size: f64,
    /// Maximum order size
    pub max_order_size: f64,
    /// Allow partial fills
    pub allow_partial_fills: bool,
    /// Simulate maker/taker dynamics
    pub simulate_maker_taker: bool,
    /// Probability of being front-run on aggressive orders (0.0 - 1.0)
    pub front_run_probability: f64,
}

impl Default for MatchingConfig {
    fn default() -> Self {
        Self {
            min_order_size: 0.0001,
            max_order_size: 1000.0,
            allow_partial_fills: true,
            simulate_maker_taker: true,
            front_run_probability: 0.1, // 10% chance of being front-run
        }
    }
}

impl OrderMatchingEngine {
    pub fn new(config: MatchingConfig) -> Self {
        Self {
            order_queue: VecDeque::new(),
            config,
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(MatchingConfig::default())
    }

    /// Match an order against the order book
    pub fn match_order(
        &mut self,
        order: SimulatedOrder,
        order_book: &LocalOrderBook,
    ) -> MatchResult {
        // Validate order
        if let Err(_reason) = self.validate_order(&order, order_book) {
            return MatchResult {
                order_id: order.id.clone(),
                fills: Vec::new(),
                status: OrderStatus::Rejected,
                total_filled: 0.0,
                average_price: 0.0,
                remaining_quantity: order.quantity,
                timestamp: Utc::now(),
            };
        }

        // Match based on order type
        match order.order_type {
            OrderType::Market => self.match_market_order(order, order_book),
            OrderType::Limit => self.match_limit_order(order, order_book),
            OrderType::PostOnly => self.match_post_only_order(order, order_book),
        }
    }

    /// Match a market order (takes liquidity)
    fn match_market_order(
        &self,
        order: SimulatedOrder,
        order_book: &LocalOrderBook,
    ) -> MatchResult {
        let mut fills = Vec::new();
        let mut remaining = order.quantity;

        // Determine which side of the book to match against
        let levels = match order.side {
            OrderSide::Buy => {
                // Buy order matches against asks (selling liquidity)
                order_book.asks.iter().collect::<Vec<_>>()
            }
            OrderSide::Sell => {
                // Sell order matches against bids (buying liquidity)
                order_book.bids.iter().rev().collect::<Vec<_>>()
            }
        };

        // Walk the order book
        for (_, level) in levels {
            if remaining <= 0.0 {
                break;
            }

            let fill_quantity = remaining.min(level.volume);
            
            fills.push(FillInfo {
                price: level.price,
                quantity: fill_quantity,
                timestamp: Utc::now(),
                is_maker: false, // Market orders are always takers
            });

            remaining -= fill_quantity;
        }

        let total_filled = order.quantity - remaining;
        let average_price = if total_filled > 0.0 {
            fills.iter().map(|f| f.price * f.quantity).sum::<f64>() / total_filled
        } else {
            0.0
        };

        let status = if remaining == 0.0 {
            OrderStatus::FullyFilled
        } else if total_filled > 0.0 {
            OrderStatus::PartiallyFilled
        } else {
            OrderStatus::Rejected
        };

        MatchResult {
            order_id: order.id,
            fills,
            status,
            total_filled,
            average_price,
            remaining_quantity: remaining,
            timestamp: Utc::now(),
        }
    }

    /// Match a limit order (may add or take liquidity)
    fn match_limit_order(
        &self,
        order: SimulatedOrder,
        order_book: &LocalOrderBook,
    ) -> MatchResult {
        let limit_price = order.price.unwrap_or(0.0);
        let mut fills = Vec::new();
        let mut remaining = order.quantity;

        // Determine if order can match immediately
        let can_match = match order.side {
            OrderSide::Buy => {
                // Buy limit can match if limit price >= best ask
                order_book.best_ask()
                    .map(|ask| limit_price >= ask.price)
                    .unwrap_or(false)
            }
            OrderSide::Sell => {
                // Sell limit can match if limit price <= best bid
                order_book.best_bid()
                    .map(|bid| limit_price <= bid.price)
                    .unwrap_or(false)
            }
        };

        if !can_match {
            // Order would be posted to book
            return MatchResult {
                order_id: order.id,
                fills: Vec::new(),
                status: OrderStatus::PostedToBook,
                total_filled: 0.0,
                average_price: limit_price,
                remaining_quantity: order.quantity,
                timestamp: Utc::now(),
            };
        }

        // Match against order book
        let levels = match order.side {
            OrderSide::Buy => order_book.asks.iter().collect::<Vec<_>>(),
            OrderSide::Sell => order_book.bids.iter().rev().collect::<Vec<_>>(),
        };

        for (_, level) in levels {
            if remaining <= 0.0 {
                break;
            }

            // Check if price still matches
            let price_matches = match order.side {
                OrderSide::Buy => limit_price >= level.price,
                OrderSide::Sell => limit_price <= level.price,
            };

            if !price_matches {
                break;
            }

            let fill_quantity = remaining.min(level.volume);
            
            fills.push(FillInfo {
                price: level.price,
                quantity: fill_quantity,
                timestamp: Utc::now(),
                is_maker: false, // Taking liquidity
            });

            remaining -= fill_quantity;
        }

        let total_filled = order.quantity - remaining;
        let average_price = if total_filled > 0.0 {
            fills.iter().map(|f| f.price * f.quantity).sum::<f64>() / total_filled
        } else {
            limit_price
        };

        let status = if remaining == 0.0 {
            OrderStatus::FullyFilled
        } else if total_filled > 0.0 {
            OrderStatus::PartiallyFilled
        } else {
            OrderStatus::PostedToBook
        };

        MatchResult {
            order_id: order.id,
            fills,
            status,
            total_filled,
            average_price,
            remaining_quantity: remaining,
            timestamp: Utc::now(),
        }
    }

    /// Match a post-only order (only adds liquidity, never takes)
    fn match_post_only_order(
        &self,
        order: SimulatedOrder,
        order_book: &LocalOrderBook,
    ) -> MatchResult {
        let limit_price = order.price.unwrap_or(0.0);

        // Check if order would immediately match
        let would_match = match order.side {
            OrderSide::Buy => {
                order_book.best_ask()
                    .map(|ask| limit_price >= ask.price)
                    .unwrap_or(false)
            }
            OrderSide::Sell => {
                order_book.best_bid()
                    .map(|bid| limit_price <= bid.price)
                    .unwrap_or(false)
            }
        };

        if would_match {
            // Post-only order would take liquidity, so reject it
            return MatchResult {
                order_id: order.id,
                fills: Vec::new(),
                status: OrderStatus::Rejected,
                total_filled: 0.0,
                average_price: limit_price,
                remaining_quantity: order.quantity,
                timestamp: Utc::now(),
            };
        }

        // Order can be posted to book
        MatchResult {
            order_id: order.id,
            fills: Vec::new(),
            status: OrderStatus::PostedToBook,
            total_filled: 0.0,
            average_price: limit_price,
            remaining_quantity: order.quantity,
            timestamp: Utc::now(),
        }
    }

    /// Validate an order
    fn validate_order(
        &self,
        order: &SimulatedOrder,
        order_book: &LocalOrderBook,
    ) -> Result<(), String> {
        // Check minimum size
        if order.quantity < self.config.min_order_size {
            return Err(format!(
                "Order quantity {} below minimum {}",
                order.quantity, self.config.min_order_size
            ));
        }

        // Check maximum size
        if order.quantity > self.config.max_order_size {
            return Err(format!(
                "Order quantity {} exceeds maximum {}",
                order.quantity, self.config.max_order_size
            ));
        }

        // Check if order book has liquidity (for market orders)
        if order.order_type == OrderType::Market {
            let has_liquidity = match order.side {
                OrderSide::Buy => order_book.ask_vwap(order.quantity).is_some(),
                OrderSide::Sell => order_book.bid_vwap(order.quantity).is_some(),
            };

            if !has_liquidity {
                return Err("Insufficient liquidity in order book".to_string());
            }
        }

        // Validate limit price for limit orders
        if order.order_type != OrderType::Market {
            if order.price.is_none() {
                return Err("Limit order requires price".to_string());
            }

            let price = order.price.unwrap();
            if price <= 0.0 {
                return Err(format!("Invalid price: {}", price));
            }
        }

        Ok(())
    }

    /// Calculate market impact for an order
    pub fn calculate_market_impact(
        &self,
        order: &SimulatedOrder,
        order_book: &LocalOrderBook,
    ) -> MarketImpact {
        let reference_price = order_book.mid_price().unwrap_or(0.0);
        
        let (execution_price, available_volume) = match order.side {
            OrderSide::Buy => {
                order_book.ask_vwap(order.quantity)
                    .unwrap_or((reference_price, 0.0))
            }
            OrderSide::Sell => {
                order_book.bid_vwap(order.quantity)
                    .unwrap_or((reference_price, 0.0))
            }
        };

        let slippage = if reference_price > 0.0 {
            ((execution_price - reference_price) / reference_price).abs()
        } else {
            0.0
        };

        let impact_bps = slippage * 10000.0;
        
        let liquidity_depth = match order.side {
            OrderSide::Buy => order_book.ask_volume_at_or_below(execution_price),
            OrderSide::Sell => order_book.bid_volume_at_or_above(execution_price),
        };

        MarketImpact {
            reference_price,
            execution_price,
            slippage_pct: slippage * 100.0,
            impact_bps,
            available_volume,
            liquidity_depth,
            can_fill: available_volume >= order.quantity,
        }
    }
}

/// Market impact analysis
#[derive(Debug, Clone)]
pub struct MarketImpact {
    pub reference_price: f64,
    pub execution_price: f64,
    pub slippage_pct: f64,
    pub impact_bps: f64, // Basis points
    pub available_volume: f64,
    pub liquidity_depth: f64,
    pub can_fill: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::order_book::{LocalOrderBook, OrderBookSnapshot};

    fn create_test_order_book() -> LocalOrderBook {
        let snapshot = OrderBookSnapshot {
            pair: "ETHGBP".to_string(),
            bids: vec![
                (2000.0, 1.0),
                (1999.0, 2.0),
                (1998.0, 3.0),
            ],
            asks: vec![
                (2001.0, 1.0),
                (2002.0, 2.0),
                (2003.0, 3.0),
            ],
            timestamp: Utc::now(),
        };
        LocalOrderBook::from_snapshot(snapshot)
    }

    #[test]
    fn test_market_buy_order() {
        let mut engine = OrderMatchingEngine::with_default_config();
        let order_book = create_test_order_book();

        let order = SimulatedOrder {
            id: "test-1".to_string(),
            pair: "ETHGBP".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            price: None,
            quantity: 2.0,
            timestamp: Utc::now(),
        };

        let result = engine.match_order(order, &order_book);
        
        assert_eq!(result.status, OrderStatus::FullyFilled);
        assert_eq!(result.total_filled, 2.0);
        assert_eq!(result.fills.len(), 2); // Should fill at 2001 and 2002
    }

    #[test]
    fn test_limit_buy_order() {
        let mut engine = OrderMatchingEngine::with_default_config();
        let order_book = create_test_order_book();

        let order = SimulatedOrder {
            id: "test-2".to_string(),
            pair: "ETHGBP".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            price: Some(2001.5),
            quantity: 1.5,
            timestamp: Utc::now(),
        };

        let result = engine.match_order(order, &order_book);
        
        // Should partially fill: 1.0 @ 2001, then 0.5 @ 2002 (within limit)
        assert_eq!(result.status, OrderStatus::PartiallyFilled);
        assert_eq!(result.total_filled, 1.0); // Only fills at 2001, 2002 exceeds limit
    }

    #[test]
    fn test_post_only_order() {
        let mut engine = OrderMatchingEngine::with_default_config();
        let order_book = create_test_order_book();

        let order = SimulatedOrder {
            id: "test-3".to_string(),
            pair: "ETHGBP".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::PostOnly,
            price: Some(2001.0), // Would match, so should be rejected
            quantity: 1.0,
            timestamp: Utc::now(),
        };

        let result = engine.match_order(order, &order_book);
        
        assert_eq!(result.status, OrderStatus::Rejected);
    }
}
