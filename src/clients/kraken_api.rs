// Kraken Historical Data API Client

use reqwest;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime};
use tokio::time::sleep;
use chrono::{DateTime, Utc};
use crate::backtesting::{OHLCData, HistoricalData};

#[derive(Debug)]
pub struct KrakenHistoricalClient {
    client: reqwest::Client,
    base_url: String,
    rate_limiter: RateLimiter,
    cache: DataCache,
}

impl KrakenHistoricalClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api.kraken.com".to_string(),
            rate_limiter: RateLimiter::new(60, Duration::from_secs(60)), // 60 calls per minute
            cache: DataCache::new(),
        }
    }

    /// Fetch OHLC data for a trading pair
    pub async fn fetch_ohlc(
        &mut self,
        pair: &str,
        interval: u32,
        since: Option<DateTime<Utc>>,
    ) -> Result<HistoricalData, KrakenApiError> {
        // Check cache first
        let cache_key = format!("{}_{}", pair, interval);
        if let Some(cached_data) = self.cache.get(&cache_key) {
            if self.is_cache_valid(&cached_data, since) {
                return Ok(cached_data.clone());
            }
        }

        // Rate limiting
        self.rate_limiter.wait_if_needed().await;

        // Build request parameters
        let mut params = vec![
            ("pair", pair.to_string()),
            ("interval", interval.to_string()),
        ];

        if let Some(since_time) = since {
            let since_timestamp = since_time.timestamp().to_string();
            params.push(("since", since_timestamp));
        }

        let url = format!("{}/0/public/OHLC", self.base_url);
        
        let response = self.client
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| KrakenApiError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(KrakenApiError::HttpError(response.status().as_u16()));
        }

        let json: Value = response
            .json()
            .await
            .map_err(|e| KrakenApiError::ParseError(e.to_string()))?;

        // Parse Kraken response
        let ohlc_data = self.parse_ohlc_response(json, pair)?;
        
        // Convert to HistoricalData
        let historical_data = HistoricalData::from_ohlc(
            ohlc_data,
            pair.to_string(),
            format!("{}m", interval),
        );

        // Cache the result
        self.cache.insert(cache_key, historical_data.clone());

        Ok(historical_data)
    }

    /// Fetch data for multiple trading pairs in parallel
    pub async fn fetch_multiple_pairs(
        &mut self,
        pairs: &[&str],
        interval: u32,
        since: Option<DateTime<Utc>>,
    ) -> Result<HashMap<String, HistoricalData>, KrakenApiError> {
        let mut results = HashMap::new();
        
        // Process pairs with rate limiting
        for pair in pairs {
            match self.fetch_ohlc(pair, interval, since).await {
                Ok(data) => {
                    results.insert(pair.to_string(), data);
                }
                Err(e) => {
                    eprintln!("Failed to fetch data for {}: {:?}", pair, e);
                    // Continue with other pairs
                }
            }
            
            // Small delay between requests to be respectful
            sleep(Duration::from_millis(100)).await;
        }
        
        Ok(results)
    }

    fn parse_ohlc_response(&self, json: Value, _pair: &str) -> Result<Vec<OHLCData>, KrakenApiError> {
        let result = json["result"].as_object()
            .ok_or_else(|| KrakenApiError::ParseError("Missing result field".to_string()))?;

        // Kraken returns pair data with normalized pair names
        let pair_data = result.values().next()
            .and_then(|v| v.as_array())
            .ok_or_else(|| KrakenApiError::ParseError("Invalid OHLC data format".to_string()))?;

        let mut ohlc_data = Vec::with_capacity(pair_data.len());

        for candle in pair_data {
            let candle_array = candle.as_array()
                .ok_or_else(|| KrakenApiError::ParseError("Invalid candle format".to_string()))?;

            if candle_array.len() < 7 {
                continue; // Skip malformed candles
            }

            let timestamp = candle_array[0].as_u64()
                .ok_or_else(|| KrakenApiError::ParseError("Invalid timestamp".to_string()))?;
            
            let open = candle_array[1].as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or_else(|| KrakenApiError::ParseError("Invalid open price".to_string()))?;
                
            let high = candle_array[2].as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or_else(|| KrakenApiError::ParseError("Invalid high price".to_string()))?;
                
            let low = candle_array[3].as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or_else(|| KrakenApiError::ParseError("Invalid low price".to_string()))?;
                
            let close = candle_array[4].as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or_else(|| KrakenApiError::ParseError("Invalid close price".to_string()))?;
                
            let volume = candle_array[6].as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or_else(|| KrakenApiError::ParseError("Invalid volume".to_string()))?;

            let dt = DateTime::from_timestamp(timestamp as i64, 0)
                .ok_or_else(|| KrakenApiError::ParseError("Invalid timestamp conversion".to_string()))?;

            ohlc_data.push(OHLCData {
                timestamp: dt,
                open,
                high,
                low,
                close,
                volume,
            });
        }

        // Sort by timestamp to ensure chronological order
        ohlc_data.sort_by_key(|candle| candle.timestamp);

        Ok(ohlc_data)
    }

    fn is_cache_valid(&self, cached_data: &HistoricalData, since: Option<DateTime<Utc>>) -> bool {
        // Simple cache validation - in production, you'd want more sophisticated logic
        if let Some(since_time) = since {
            if let Some(last_timestamp) = cached_data.timestamps.last() {
                return *last_timestamp >= since_time;
            }
        }
        
        // For now, consider cache valid for 5 minutes
        // In production, this would depend on the timeframe
        false
    }
}

impl Default for KrakenHistoricalClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct RateLimiter {
    max_calls: u32,
    window_duration: Duration,
    calls: Vec<Instant>,
}

impl RateLimiter {
    fn new(max_calls: u32, window_duration: Duration) -> Self {
        Self {
            max_calls,
            window_duration,
            calls: Vec::new(),
        }
    }

    async fn wait_if_needed(&mut self) {
        let now = Instant::now();
        
        // Remove old calls outside the window
        self.calls.retain(|&call_time| now.duration_since(call_time) <= self.window_duration);
        
        // If we're at the limit, wait
        if self.calls.len() >= self.max_calls as usize {
            if let Some(&oldest_call) = self.calls.first() {
                let wait_time = self.window_duration.saturating_sub(now.duration_since(oldest_call));
                if wait_time > Duration::from_millis(0) {
                    sleep(wait_time).await;
                }
            }
        }
        
        self.calls.push(now);
    }
}

#[derive(Debug)]
struct DataCache {
    data: HashMap<String, CachedData>,
    max_entries: usize,
}

#[derive(Debug, Clone)]
struct CachedData {
    data: HistoricalData,
    cached_at: SystemTime,
    ttl: Duration,
}

impl DataCache {
    fn new() -> Self {
        Self {
            data: HashMap::new(),
            max_entries: 100, // Limit cache size
        }
    }

    fn get(&self, key: &str) -> Option<&HistoricalData> {
        if let Some(cached) = self.data.get(key) {
            if cached.cached_at.elapsed().unwrap_or(Duration::MAX) <= cached.ttl {
                return Some(&cached.data);
            }
        }
        None
    }

    fn insert(&mut self, key: String, data: HistoricalData) {
        // Simple eviction policy - remove oldest if at capacity
        if self.data.len() >= self.max_entries {
            if let Some(oldest_key) = self.find_oldest_key() {
                self.data.remove(&oldest_key);
            }
        }

        let cached_data = CachedData {
            data,
            cached_at: SystemTime::now(),
            ttl: Duration::from_secs(300), // 5 minutes TTL
        };

        self.data.insert(key, cached_data);
    }

    fn find_oldest_key(&self) -> Option<String> {
        self.data
            .iter()
            .min_by_key(|(_, cached)| cached.cached_at)
            .map(|(key, _)| key.clone())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KrakenApiError {
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("HTTP error: {0}")]
    HttpError(u16),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Invalid trading pair: {0}")]
    InvalidPair(String),
}

/// Utility function to get available trading pairs from Kraken
pub async fn get_available_pairs() -> Result<Vec<String>, KrakenApiError> {
    let client = reqwest::Client::new();
    let url = "https://api.kraken.com/0/public/AssetPairs";
    
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| KrakenApiError::NetworkError(e.to_string()))?;

    let json: Value = response
        .json()
        .await
        .map_err(|e| KrakenApiError::ParseError(e.to_string()))?;

    let result = json["result"].as_object()
        .ok_or_else(|| KrakenApiError::ParseError("Missing result field".to_string()))?;

    let pairs: Vec<String> = result.keys().cloned().collect();
    Ok(pairs)
}

/// Utility function to normalize pair names (e.g., "XRPGBP" -> "XRP/GBP")
pub fn normalize_pair_name(pair: &str) -> String {
    // This is a simplified version - Kraken has complex pair naming rules
    if pair.len() == 6 {
        format!("{}/{}", &pair[..3], &pair[3..])
    } else {
        pair.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kraken_client_creation() {
        let client = KrakenHistoricalClient::new();
        assert_eq!(client.base_url, "https://api.kraken.com");
    }

    #[test]
    fn test_pair_normalization() {
        assert_eq!(normalize_pair_name("XRPGBP"), "XRP/GBP");
        assert_eq!(normalize_pair_name("BTCUSD"), "BTC/USD");
        assert_eq!(normalize_pair_name("ETH/USD"), "ETH/USD");
    }
}