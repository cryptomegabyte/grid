// Comprehensive error handling system with circuit breakers and retry mechanisms

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TradingError {
    #[error("Market data error: {0}")]
    MarketDataError(String),
    
    #[error("API connection error: {0}")]
    ApiConnectionError(String),
    
    #[error("Position management error: {0}")]
    PositionError(String),
    
    #[error("Risk management violation: {0}")]
    RiskViolation(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("System overload")]
    SystemOverload,
    
    #[error("Circuit breaker open")]
    CircuitBreakerOpen,
    
    #[error("Maximum retries exceeded")]
    MaxRetriesExceeded,
}

/// Circuit breaker pattern for fault tolerance
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    state: Arc<Mutex<CircuitBreakerState>>,
    failure_threshold: u32,
    recovery_timeout: Duration,
    success_threshold: u32,
}

#[derive(Debug)]
struct CircuitBreakerState {
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
    state: CircuitState,
}

#[derive(Debug, PartialEq)]
enum CircuitState {
    Closed,    // Normal operation
    Open,      // Failing fast
    HalfOpen,  // Testing recovery
}

impl CircuitBreaker {
    pub fn new(
        failure_threshold: u32,
        recovery_timeout: Duration,
        success_threshold: u32,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(CircuitBreakerState {
                failure_count: 0,
                success_count: 0,
                last_failure_time: None,
                state: CircuitState::Closed,
            })),
            failure_threshold,
            recovery_timeout,
            success_threshold,
        }
    }

    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, TradingError>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: Into<TradingError>,
    {
        // Check if circuit is open
        {
            let mut state = self.state.lock().unwrap();
            if state.state == CircuitState::Open {
                if let Some(last_failure) = state.last_failure_time {
                    if last_failure.elapsed() < self.recovery_timeout {
                        return Err(TradingError::CircuitBreakerOpen);
                    } else {
                        // Try to recover
                        state.state = CircuitState::HalfOpen;
                        state.success_count = 0;
                    }
                }
            }
        }

        // Execute operation
        match operation.await {
            Ok(result) => {
                self.on_success();
                Ok(result)
            }
            Err(error) => {
                self.on_failure();
                Err(error.into())
            }
        }
    }

    fn on_success(&self) {
        let mut state = self.state.lock().unwrap();
        state.failure_count = 0;
        
        match state.state {
            CircuitState::HalfOpen => {
                state.success_count += 1;
                if state.success_count >= self.success_threshold {
                    state.state = CircuitState::Closed;
                }
            }
            _ => {
                state.state = CircuitState::Closed;
            }
        }
    }

    fn on_failure(&self) {
        let mut state = self.state.lock().unwrap();
        state.failure_count += 1;
        state.last_failure_time = Some(Instant::now());

        if state.failure_count >= self.failure_threshold {
            state.state = CircuitState::Open;
        }
    }

    pub fn is_open(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.state == CircuitState::Open
    }
}

/// Retry mechanism with exponential backoff
pub struct RetryPolicy {
    max_retries: u32,
    base_delay: Duration,
    max_delay: Duration,
    backoff_multiplier: f64,
}

impl RetryPolicy {
    pub fn new(
        max_retries: u32,
        base_delay: Duration,
        max_delay: Duration,
        backoff_multiplier: f64,
    ) -> Self {
        Self {
            max_retries,
            base_delay,
            max_delay,
            backoff_multiplier,
        }
    }

    pub async fn execute<F, T, E>(&self, mut operation: F) -> Result<T, TradingError>
    where
        F: FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
        E: Into<TradingError> + std::fmt::Debug,
    {
        let mut delay = self.base_delay;
        
        for attempt in 0..=self.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    if attempt == self.max_retries {
                        return Err(TradingError::MaxRetriesExceeded);
                    }
                    
                    // Log retry attempt
                    println!("Operation failed (attempt {}), retrying in {:?}: {:?}", 
                             attempt + 1, delay, error);
                    
                    sleep(delay).await;
                    
                    // Exponential backoff
                    delay = std::cmp::min(
                        Duration::from_millis(
                            (delay.as_millis() as f64 * self.backoff_multiplier) as u64
                        ),
                        self.max_delay,
                    );
                }
            }
        }
        
        Err(TradingError::MaxRetriesExceeded)
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new(
            3,                                    // 3 retries
            Duration::from_millis(500),          // 500ms base delay
            Duration::from_secs(30),             // 30s max delay
            2.0,                                 // Double delay each time
        )
    }
}

/// Health check system for monitoring system components
#[derive(Debug)]
pub struct HealthMonitor {
    api_circuit_breaker: CircuitBreaker,
    websocket_circuit_breaker: CircuitBreaker,
    last_health_check: Mutex<Instant>,
    health_check_interval: Duration,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            api_circuit_breaker: CircuitBreaker::new(
                5,                               // 5 failures to open
                Duration::from_secs(60),         // 1 minute recovery
                3,                               // 3 successes to close
            ),
            websocket_circuit_breaker: CircuitBreaker::new(
                3,                               // 3 failures to open
                Duration::from_secs(30),         // 30 seconds recovery
                2,                               // 2 successes to close
            ),
            last_health_check: Mutex::new(Instant::now()),
            health_check_interval: Duration::from_secs(30),
        }
    }

    pub fn api_circuit_breaker(&self) -> &CircuitBreaker {
        &self.api_circuit_breaker
    }

    pub fn websocket_circuit_breaker(&self) -> &CircuitBreaker {
        &self.websocket_circuit_breaker
    }

    pub async fn perform_health_check(&self) -> Result<HealthStatus, TradingError> {
        let mut last_check = self.last_health_check.lock().unwrap();
        
        if last_check.elapsed() < self.health_check_interval {
            return Ok(HealthStatus::Healthy);
        }
        
        *last_check = Instant::now();
        drop(last_check);

        let mut status = HealthStatus::Healthy;
        let mut issues = Vec::new();

        // Check API health
        if self.api_circuit_breaker.is_open() {
            status = HealthStatus::Degraded;
            issues.push("API circuit breaker is open".to_string());
        }

        // Check WebSocket health
        if self.websocket_circuit_breaker.is_open() {
            status = HealthStatus::Degraded;
            issues.push("WebSocket circuit breaker is open".to_string());
        }

        // Additional health checks could go here
        // - Memory usage
        // - CPU usage
        // - Disk space
        // - Network connectivity

        if !issues.is_empty() {
            println!("Health check issues detected: {:?}", issues);
        }

        Ok(status)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Graceful shutdown handler
pub struct GracefulShutdown {
    shutdown_signal: Arc<Mutex<bool>>,
    active_operations: Arc<Mutex<u32>>,
}

impl GracefulShutdown {
    pub fn new() -> Self {
        Self {
            shutdown_signal: Arc::new(Mutex::new(false)),
            active_operations: Arc::new(Mutex::new(0)),
        }
    }

    pub fn initiate_shutdown(&self) {
        let mut shutdown = self.shutdown_signal.lock().unwrap();
        *shutdown = true;
        println!("🛑 Graceful shutdown initiated");
    }

    pub fn is_shutting_down(&self) -> bool {
        *self.shutdown_signal.lock().unwrap()
    }

    pub fn register_operation(&self) -> OperationGuard {
        let mut count = self.active_operations.lock().unwrap();
        *count += 1;
        OperationGuard {
            counter: self.active_operations.clone(),
        }
    }

    pub async fn wait_for_completion(&self, timeout: Duration) {
        let start = Instant::now();
        
        while start.elapsed() < timeout {
            let active_count = *self.active_operations.lock().unwrap();
            if active_count == 0 {
                println!("✅ All operations completed gracefully");
                return;
            }
            
            println!("⏳ Waiting for {} operations to complete...", active_count);
            sleep(Duration::from_millis(100)).await;
        }
        
        let remaining = *self.active_operations.lock().unwrap();
        if remaining > 0 {
            println!("⚠️  Shutdown timeout: {} operations still active", remaining);
        }
    }
}

pub struct OperationGuard {
    counter: Arc<Mutex<u32>>,
}

impl Drop for OperationGuard {
    fn drop(&mut self) {
        let mut count = self.counter.lock().unwrap();
        *count -= 1;
    }
}

impl Default for GracefulShutdown {
    fn default() -> Self {
        Self::new()
    }
}