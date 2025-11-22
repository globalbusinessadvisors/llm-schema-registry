use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: usize,

    /// Time window duration
    pub window_duration: Duration,

    /// Enable adaptive rate limiting based on system load
    pub adaptive: bool,

    /// Burst allowance (allows temporary spikes)
    pub burst_size: usize,

    /// Queue depth before rejecting requests
    pub max_queue_depth: usize,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 1000,
            window_duration: Duration::from_secs(60),
            adaptive: true,
            burst_size: 100,
            max_queue_depth: 10000,
        }
    }
}

/// Client rate limit state
#[derive(Debug, Clone)]
struct ClientState {
    /// Request count in current window
    request_count: usize,

    /// Window start time
    window_start: Instant,

    /// Tokens available (for token bucket)
    tokens: f64,

    /// Last token refill time
    last_refill: Instant,
}

impl ClientState {
    fn new() -> Self {
        Self {
            request_count: 0,
            window_start: Instant::now(),
            tokens: 100.0,
            last_refill: Instant::now(),
        }
    }

    /// Check if rate limit exceeded (sliding window)
    fn check_rate_limit(&mut self, config: &RateLimitConfig) -> bool {
        let now = Instant::now();

        // Reset window if expired
        if now.duration_since(self.window_start) > config.window_duration {
            self.request_count = 0;
            self.window_start = now;
        }

        // Check limit
        if self.request_count >= config.max_requests {
            return false;
        }

        self.request_count += 1;
        true
    }

    /// Check token bucket (for burst handling)
    fn check_token_bucket(&mut self, config: &RateLimitConfig) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();

        // Refill tokens based on time elapsed
        let refill_rate = config.max_requests as f64 / config.window_duration.as_secs_f64();
        self.tokens = (self.tokens + elapsed * refill_rate).min(config.burst_size as f64);
        self.last_refill = now;

        // Consume one token
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Rate limiter state
pub struct RateLimiter {
    config: RateLimitConfig,
    clients: Arc<RwLock<HashMap<String, ClientState>>>,
    current_queue_depth: Arc<RwLock<usize>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            clients: Arc::new(RwLock::new(HashMap::new())),
            current_queue_depth: Arc::new(RwLock::new(0)),
        }
    }

    /// Extract client identifier from request
    fn extract_client_id(req: &Request) -> String {
        // Try API key first
        if let Some(api_key) = req.headers().get("X-API-Key") {
            if let Ok(key) = api_key.to_str() {
                return format!("api_key:{}", key);
            }
        }

        // Fallback to IP address
        if let Some(forwarded) = req.headers().get("X-Forwarded-For") {
            if let Ok(ip) = forwarded.to_str() {
                return format!("ip:{}", ip.split(',').next().unwrap_or("unknown"));
            }
        }

        // Last resort: remote addr from connection info
        "ip:unknown".to_string()
    }

    /// Check if request should be rate limited
    pub async fn check_rate_limit(&self, req: &Request) -> Result<(), StatusCode> {
        let client_id = Self::extract_client_id(req);

        // Check queue depth first (backpressure)
        let queue_depth = *self.current_queue_depth.read().await;
        if queue_depth >= self.config.max_queue_depth {
            warn!(
                queue_depth = queue_depth,
                max = self.config.max_queue_depth,
                "Request rejected: queue depth exceeded"
            );
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }

        let mut clients = self.clients.write().await;
        let state = clients.entry(client_id.clone()).or_insert_with(ClientState::new);

        // Check token bucket (burst handling)
        if !state.check_token_bucket(&self.config) {
            debug!(client_id = %client_id, "Token bucket exhausted");
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }

        // Check sliding window rate limit
        if !state.check_rate_limit(&self.config) {
            warn!(
                client_id = %client_id,
                count = state.request_count,
                max = self.config.max_requests,
                "Rate limit exceeded"
            );
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }

        Ok(())
    }

    /// Increment queue depth
    pub async fn increment_queue_depth(&self) {
        let mut depth = self.current_queue_depth.write().await;
        *depth += 1;
    }

    /// Decrement queue depth
    pub async fn decrement_queue_depth(&self) {
        let mut depth = self.current_queue_depth.write().await;
        if *depth > 0 {
            *depth -= 1;
        }
    }

    /// Get current queue depth
    pub async fn get_queue_depth(&self) -> usize {
        *self.current_queue_depth.read().await
    }

    /// Cleanup old client states (periodic maintenance)
    pub async fn cleanup_old_states(&self) {
        let mut clients = self.clients.write().await;
        let now = Instant::now();

        clients.retain(|_, state| {
            now.duration_since(state.window_start) < self.config.window_duration * 2
        });

        debug!("Rate limiter cleanup: {} active clients", clients.len());
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(rate_limiter): State<Arc<RateLimiter>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Check rate limit
    rate_limiter.check_rate_limit(&req).await?;

    // Increment queue depth
    rate_limiter.increment_queue_depth().await;

    // Process request
    let response = next.run(req).await;

    // Decrement queue depth
    rate_limiter.decrement_queue_depth().await;

    Ok(response)
}

/// Adaptive rate limiter that adjusts based on system load
pub struct AdaptiveRateLimiter {
    base_limiter: RateLimiter,
    cpu_threshold: f64,
    memory_threshold: f64,
}

impl AdaptiveRateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            base_limiter: RateLimiter::new(config),
            cpu_threshold: 0.80,    // 80% CPU
            memory_threshold: 0.80, // 80% memory
        }
    }

    /// Check system load and adjust rate limits
    pub async fn check_system_load(&self) -> f64 {
        // In a real implementation, this would query actual system metrics
        // For now, return a simulated value based on queue depth

        let queue_depth = self.base_limiter.get_queue_depth().await;
        let max_queue = self.base_limiter.config.max_queue_depth as f64;

        // Calculate load factor (0.0 to 1.0+)
        queue_depth as f64 / max_queue
    }

    /// Adaptive rate limit check
    pub async fn check_rate_limit(&self, req: &Request) -> Result<(), StatusCode> {
        // Check system load
        let load = self.check_system_load().await;

        // If system is overloaded, be more aggressive with rate limiting
        if load > 0.90 {
            warn!(load = load, "System overloaded, rejecting requests");
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }

        // Standard rate limit check
        self.base_limiter.check_rate_limit(req).await
    }
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker for dependency protection
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitBreakerState>>,
    failure_count: Arc<RwLock<usize>>,
    failure_threshold: usize,
    timeout: Duration,
    last_failure: Arc<RwLock<Option<Instant>>>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: usize, timeout: Duration) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitBreakerState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            failure_threshold,
            timeout,
            last_failure: Arc::new(RwLock::new(None)),
        }
    }

    /// Check if circuit is open
    pub async fn is_open(&self) -> bool {
        let state = *self.state.read().await;

        match state {
            CircuitBreakerState::Open => {
                // Check if timeout has passed
                if let Some(last_failure) = *self.last_failure.read().await {
                    if Instant::now().duration_since(last_failure) > self.timeout {
                        // Transition to half-open
                        *self.state.write().await = CircuitBreakerState::HalfOpen;
                        return false;
                    }
                }
                true
            }
            CircuitBreakerState::HalfOpen => false,
            CircuitBreakerState::Closed => false,
        }
    }

    /// Record success
    pub async fn record_success(&self) {
        let state = *self.state.read().await;

        match state {
            CircuitBreakerState::HalfOpen => {
                // Transition to closed
                *self.state.write().await = CircuitBreakerState::Closed;
                *self.failure_count.write().await = 0;
                *self.last_failure.write().await = None;
            }
            CircuitBreakerState::Closed => {
                // Reset failure count on success
                *self.failure_count.write().await = 0;
            }
            CircuitBreakerState::Open => {}
        }
    }

    /// Record failure
    pub async fn record_failure(&self) {
        let mut count = self.failure_count.write().await;
        *count += 1;

        if *count >= self.failure_threshold {
            *self.state.write().await = CircuitBreakerState::Open;
            *self.last_failure.write().await = Some(Instant::now());
            warn!(
                failures = *count,
                threshold = self.failure_threshold,
                "Circuit breaker opened"
            );
        }
    }

    /// Get current state
    pub async fn get_state(&self) -> CircuitBreakerState {
        *self.state.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let config = RateLimitConfig {
            max_requests: 10,
            window_duration: Duration::from_secs(60),
            adaptive: false,
            burst_size: 5,
            max_queue_depth: 100,
        };

        let limiter = RateLimiter::new(config);

        // Should allow requests up to limit
        for _ in 0..10 {
            limiter.increment_queue_depth().await;
        }

        assert_eq!(limiter.get_queue_depth().await, 10);

        for _ in 0..10 {
            limiter.decrement_queue_depth().await;
        }

        assert_eq!(limiter.get_queue_depth().await, 0);
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let breaker = CircuitBreaker::new(3, Duration::from_secs(1));

        // Initially closed
        assert_eq!(breaker.get_state().await, CircuitBreakerState::Closed);
        assert!(!breaker.is_open().await);

        // Record failures
        for _ in 0..3 {
            breaker.record_failure().await;
        }

        // Should be open now
        assert_eq!(breaker.get_state().await, CircuitBreakerState::Open);
        assert!(breaker.is_open().await);

        // Wait for timeout
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Should transition to half-open
        assert!(!breaker.is_open().await);
        assert_eq!(breaker.get_state().await, CircuitBreakerState::HalfOpen);

        // Success should close it
        breaker.record_success().await;
        assert_eq!(breaker.get_state().await, CircuitBreakerState::Closed);
    }

    #[test]
    fn test_client_state_token_bucket() {
        let config = RateLimitConfig {
            max_requests: 100,
            window_duration: Duration::from_secs(60),
            adaptive: false,
            burst_size: 10,
            max_queue_depth: 1000,
        };

        let mut state = ClientState::new();

        // Should allow up to burst_size requests
        for _ in 0..10 {
            assert!(state.check_token_bucket(&config));
        }

        // 11th request should fail (burst exhausted)
        assert!(!state.check_token_bucket(&config));
    }
}
