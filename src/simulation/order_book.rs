// Local Order Book Manager
// Maintains real-time order book state from Kraken WebSocket data

use std::collections::BTreeMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a single price level in the order book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookLevel {
    pub price: f64,
    pub volume: f64,
    pub timestamp: DateTime<Utc>,
}

/// Complete order book state for a trading pair
#[derive(Debug, Clone)]
pub struct LocalOrderBook {
    pub pair: String,
    /// Bids sorted by price (descending) - BTreeMap provides automatic sorting
    pub bids: BTreeMap<OrderedFloat, OrderBookLevel>,
    /// Asks sorted by price (ascending)
    pub asks: BTreeMap<OrderedFloat, OrderBookLevel>,
    pub last_update: DateTime<Utc>,
    pub sequence: u64,
    /// Checksum for order book validation (Kraken provides this)
    pub checksum: Option<u32>,
}

/// Wrapper for f64 to use as BTreeMap key (handles NaN/Inf properly)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct OrderedFloat(pub f64);

impl Eq for OrderedFloat {}

impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl From<f64> for OrderedFloat {
    fn from(f: f64) -> Self {
        OrderedFloat(f)
    }
}

/// Order book snapshot (full state)
#[derive(Debug, Clone)]
pub struct OrderBookSnapshot {
    pub pair: String,
    pub bids: Vec<(f64, f64)>, // (price, volume)
    pub asks: Vec<(f64, f64)>,
    pub timestamp: DateTime<Utc>,
}

/// Order book update (incremental change)
#[derive(Debug, Clone)]
pub enum OrderBookUpdate {
    /// Add or update a level
    Update {
        side: OrderBookSide,
        price: f64,
        volume: f64,
    },
    /// Remove a level (volume = 0)
    Remove {
        side: OrderBookSide,
        price: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderBookSide {
    Bid,
    Ask,
}

impl LocalOrderBook {
    /// Create new empty order book
    pub fn new(pair: String) -> Self {
        Self {
            pair,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_update: Utc::now(),
            sequence: 0,
            checksum: None,
        }
    }

    /// Initialize from snapshot
    pub fn from_snapshot(snapshot: OrderBookSnapshot) -> Self {
        let mut book = Self::new(snapshot.pair);
        book.last_update = snapshot.timestamp;

        // Insert bids
        for (price, volume) in snapshot.bids {
            book.bids.insert(
                OrderedFloat(price),
                OrderBookLevel {
                    price,
                    volume,
                    timestamp: snapshot.timestamp,
                },
            );
        }

        // Insert asks
        for (price, volume) in snapshot.asks {
            book.asks.insert(
                OrderedFloat(price),
                OrderBookLevel {
                    price,
                    volume,
                    timestamp: snapshot.timestamp,
                },
            );
        }

        book
    }

    /// Apply an incremental update
    pub fn apply_update(&mut self, update: OrderBookUpdate) {
        self.last_update = Utc::now();
        self.sequence += 1;

        match update {
            OrderBookUpdate::Update { side, price, volume } => {
                let level = OrderBookLevel {
                    price,
                    volume,
                    timestamp: self.last_update,
                };

                match side {
                    OrderBookSide::Bid => {
                        if volume > 0.0 {
                            self.bids.insert(OrderedFloat(price), level);
                        } else {
                            self.bids.remove(&OrderedFloat(price));
                        }
                    }
                    OrderBookSide::Ask => {
                        if volume > 0.0 {
                            self.asks.insert(OrderedFloat(price), level);
                        } else {
                            self.asks.remove(&OrderedFloat(price));
                        }
                    }
                }
            }
            OrderBookUpdate::Remove { side, price } => {
                match side {
                    OrderBookSide::Bid => {
                        self.bids.remove(&OrderedFloat(price));
                    }
                    OrderBookSide::Ask => {
                        self.asks.remove(&OrderedFloat(price));
                    }
                }
            }
        }
    }

    /// Get best bid (highest bid price)
    pub fn best_bid(&self) -> Option<&OrderBookLevel> {
        self.bids.iter().next_back().map(|(_, level)| level)
    }

    /// Get best ask (lowest ask price)
    pub fn best_ask(&self) -> Option<&OrderBookLevel> {
        self.asks.iter().next().map(|(_, level)| level)
    }

    /// Get bid-ask spread
    pub fn spread(&self) -> Option<f64> {
        match (self.best_ask(), self.best_bid()) {
            (Some(ask), Some(bid)) => Some(ask.price - bid.price),
            _ => None,
        }
    }

    /// Get mid price
    pub fn mid_price(&self) -> Option<f64> {
        match (self.best_ask(), self.best_bid()) {
            (Some(ask), Some(bid)) => Some((ask.price + bid.price) / 2.0),
            _ => None,
        }
    }

    /// Get total volume at or better than a given price on the bid side
    pub fn bid_volume_at_or_above(&self, price: f64) -> f64 {
        self.bids
            .range(OrderedFloat(price)..)
            .map(|(_, level)| level.volume)
            .sum()
    }

    /// Get total volume at or better than a given price on the ask side
    pub fn ask_volume_at_or_below(&self, price: f64) -> f64 {
        self.asks
            .range(..=OrderedFloat(price))
            .map(|(_, level)| level.volume)
            .sum()
    }

    /// Calculate volume-weighted average price (VWAP) for a given quantity on the bid side
    /// Returns (vwap, total_volume_available)
    pub fn bid_vwap(&self, quantity: f64) -> Option<(f64, f64)> {
        let mut remaining = quantity;
        let mut total_value = 0.0;
        let mut total_volume = 0.0;

        // Iterate from best bid downwards
        for (_, level) in self.bids.iter().rev() {
            if remaining <= 0.0 {
                break;
            }

            let volume_to_take = remaining.min(level.volume);
            total_value += volume_to_take * level.price;
            total_volume += volume_to_take;
            remaining -= volume_to_take;
        }

        if total_volume > 0.0 {
            Some((total_value / total_volume, total_volume))
        } else {
            None
        }
    }

    /// Calculate volume-weighted average price (VWAP) for a given quantity on the ask side
    /// Returns (vwap, total_volume_available)
    pub fn ask_vwap(&self, quantity: f64) -> Option<(f64, f64)> {
        let mut remaining = quantity;
        let mut total_value = 0.0;
        let mut total_volume = 0.0;

        // Iterate from best ask upwards
        for (_, level) in self.asks.iter() {
            if remaining <= 0.0 {
                break;
            }

            let volume_to_take = remaining.min(level.volume);
            total_value += volume_to_take * level.price;
            total_volume += volume_to_take;
            remaining -= volume_to_take;
        }

        if total_volume > 0.0 {
            Some((total_value / total_volume, total_volume))
        } else {
            None
        }
    }

    /// Get order book depth (number of price levels)
    pub fn depth(&self) -> (usize, usize) {
        (self.bids.len(), self.asks.len())
    }

    /// Get liquidity score (total volume in top N levels)
    pub fn liquidity_score(&self, levels: usize) -> f64 {
        let bid_liquidity: f64 = self.bids
            .iter()
            .rev()
            .take(levels)
            .map(|(_, level)| level.volume)
            .sum();

        let ask_liquidity: f64 = self.asks
            .iter()
            .take(levels)
            .map(|(_, level)| level.volume)
            .sum();

        bid_liquidity + ask_liquidity
    }

    /// Check if order book has sufficient liquidity for a trade
    pub fn has_sufficient_liquidity(&self, side: OrderBookSide, quantity: f64) -> bool {
        match side {
            OrderBookSide::Bid => {
                let available = self.bid_vwap(quantity);
                available.map(|(_, vol)| vol >= quantity).unwrap_or(false)
            }
            OrderBookSide::Ask => {
                let available = self.ask_vwap(quantity);
                available.map(|(_, vol)| vol >= quantity).unwrap_or(false)
            }
        }
    }

    /// Get top N levels for display
    pub fn top_levels(&self, n: usize) -> (Vec<OrderBookLevel>, Vec<OrderBookLevel>) {
        let bids: Vec<OrderBookLevel> = self.bids
            .iter()
            .rev()
            .take(n)
            .map(|(_, level)| level.clone())
            .collect();

        let asks: Vec<OrderBookLevel> = self.asks
            .iter()
            .take(n)
            .map(|(_, level)| level.clone())
            .collect();

        (bids, asks)
    }

    /// Clear all levels (for re-initialization)
    pub fn clear(&mut self) {
        self.bids.clear();
        self.asks.clear();
        self.sequence = 0;
        self.last_update = Utc::now();
    }

    /// Validate order book integrity
    pub fn validate(&self) -> Result<(), String> {
        // Check if best bid < best ask
        if let (Some(bid), Some(ask)) = (self.best_bid(), self.best_ask()) {
            if bid.price >= ask.price {
                return Err(format!(
                    "Invalid order book: best bid ({}) >= best ask ({})",
                    bid.price, ask.price
                ));
            }
        }

        // Check for negative volumes
        for (_, level) in self.bids.iter() {
            if level.volume <= 0.0 {
                return Err(format!("Negative or zero bid volume at {}", level.price));
            }
        }

        for (_, level) in self.asks.iter() {
            if level.volume <= 0.0 {
                return Err(format!("Negative or zero ask volume at {}", level.price));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_book_creation() {
        let book = LocalOrderBook::new("ETHGBP".to_string());
        assert_eq!(book.pair, "ETHGBP");
        assert_eq!(book.depth(), (0, 0));
    }

    #[test]
    fn test_snapshot_initialization() {
        let snapshot = OrderBookSnapshot {
            pair: "ETHGBP".to_string(),
            bids: vec![(2000.0, 1.0), (1999.0, 2.0)],
            asks: vec![(2001.0, 1.5), (2002.0, 2.5)],
            timestamp: Utc::now(),
        };

        let book = LocalOrderBook::from_snapshot(snapshot);
        assert_eq!(book.depth(), (2, 2));
        assert_eq!(book.best_bid().unwrap().price, 2000.0);
        assert_eq!(book.best_ask().unwrap().price, 2001.0);
    }

    #[test]
    fn test_order_book_updates() {
        let mut book = LocalOrderBook::new("ETHGBP".to_string());

        // Add bid
        book.apply_update(OrderBookUpdate::Update {
            side: OrderBookSide::Bid,
            price: 2000.0,
            volume: 1.0,
        });

        assert_eq!(book.best_bid().unwrap().price, 2000.0);

        // Add ask
        book.apply_update(OrderBookUpdate::Update {
            side: OrderBookSide::Ask,
            price: 2001.0,
            volume: 1.5,
        });

        assert_eq!(book.best_ask().unwrap().price, 2001.0);
        assert_eq!(book.spread(), Some(1.0));
    }

    #[test]
    fn test_vwap_calculation() {
        let snapshot = OrderBookSnapshot {
            pair: "ETHGBP".to_string(),
            bids: vec![(2000.0, 1.0), (1999.0, 2.0), (1998.0, 3.0)],
            asks: vec![(2001.0, 1.0), (2002.0, 2.0), (2003.0, 3.0)],
            timestamp: Utc::now(),
        };

        let book = LocalOrderBook::from_snapshot(snapshot);

        // Test bid VWAP for 2.0 quantity
        let (vwap, volume) = book.bid_vwap(2.0).unwrap();
        assert_eq!(volume, 2.0);
        assert!((vwap - 1999.5).abs() < 0.01); // (2000*1 + 1999*1) / 2

        // Test ask VWAP for 2.0 quantity
        let (vwap, volume) = book.ask_vwap(2.0).unwrap();
        assert_eq!(volume, 2.0);
        assert!((vwap - 2001.5).abs() < 0.01); // (2001*1 + 2002*1) / 2
    }
}
