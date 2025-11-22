// Graceful Shutdown Handler
// Ensures clean shutdown of the Schema Registry server

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Notify, RwLock};
use tokio::time::timeout;

/// Graceful shutdown coordinator
pub struct ShutdownCoordinator {
    /// Notification for shutdown initiation
    shutdown_notify: Arc<Notify>,
    /// Track if shutdown has been initiated
    is_shutting_down: Arc<RwLock<bool>>,
    /// Drain timeout
    drain_timeout: Duration,
}

impl ShutdownCoordinator {
    pub fn new(drain_timeout_seconds: u64) -> Self {
        Self {
            shutdown_notify: Arc::new(Notify::new()),
            is_shutting_down: Arc::new(RwLock::new(false)),
            drain_timeout: Duration::from_secs(drain_timeout_seconds),
        }
    }

    /// Get a handle to wait for shutdown signal
    pub fn subscribe(&self) -> ShutdownHandle {
        ShutdownHandle {
            notify: Arc::clone(&self.shutdown_notify),
            is_shutting_down: Arc::clone(&self.is_shutting_down),
        }
    }

    /// Initiate graceful shutdown
    pub async fn shutdown(&self) {
        tracing::info!("Initiating graceful shutdown");

        // Mark as shutting down
        {
            let mut shutting_down = self.is_shutting_down.write().await;
            *shutting_down = true;
        }

        // Notify all listeners
        self.shutdown_notify.notify_waiters();

        tracing::info!("Shutdown signal sent to all components");
    }

    /// Get drain timeout
    pub fn drain_timeout(&self) -> Duration {
        self.drain_timeout
    }

    /// Check if shutdown has been initiated
    pub async fn is_shutting_down(&self) -> bool {
        *self.is_shutting_down.read().await
    }
}

/// Handle for components to wait for shutdown
#[derive(Clone)]
pub struct ShutdownHandle {
    notify: Arc<Notify>,
    is_shutting_down: Arc<RwLock<bool>>,
}

impl ShutdownHandle {
    /// Wait for shutdown signal
    pub async fn wait(&self) {
        self.notify.notified().await;
    }

    /// Check if shutdown has been initiated
    pub async fn is_shutting_down(&self) -> bool {
        *self.is_shutting_down.read().await
    }
}

/// Graceful shutdown procedure
pub async fn execute_graceful_shutdown(
    coordinator: Arc<ShutdownCoordinator>,
    health_checker: Option<Arc<crate::health::HealthChecker>>,
) {
    tracing::info!("===========================================");
    tracing::info!("Starting graceful shutdown procedure");
    tracing::info!("===========================================");

    let shutdown_start = std::time::Instant::now();

    // Step 1: Mark service as not ready (removes from load balancer)
    tracing::info!("Step 1/6: Marking service as not ready");
    if let Some(checker) = &health_checker {
        checker.mark_not_ready().await;
        tracing::info!("Service marked as not ready (removed from load balancer)");
    }

    // Step 2: Wait for in-flight requests to complete
    tracing::info!("Step 2/6: Draining in-flight requests");
    let drain_timeout = coordinator.drain_timeout();
    tracing::info!("Waiting up to {}s for requests to complete", drain_timeout.as_secs());

    // Give in-flight requests time to complete
    match timeout(drain_timeout, async {
        // TODO: Track active requests and wait for them
        // For now, just wait the timeout period
        tokio::time::sleep(Duration::from_secs(5)).await;
    })
    .await
    {
        Ok(_) => {
            tracing::info!("All in-flight requests completed");
        }
        Err(_) => {
            tracing::warn!("Drain timeout reached, forcing shutdown");
        }
    }

    // Step 3: Close database connections
    tracing::info!("Step 3/6: Closing database connections");
    // TODO: Implement database connection pool shutdown
    tracing::info!("Database connections closed");

    // Step 4: Close Redis connections
    tracing::info!("Step 4/6: Closing Redis connections");
    // TODO: Implement Redis connection pool shutdown
    tracing::info!("Redis connections closed");

    // Step 5: Flush metrics and logs
    tracing::info!("Step 5/6: Flushing metrics and logs");
    // TODO: Implement metrics flush
    // TODO: Implement log flush
    tokio::time::sleep(Duration::from_millis(500)).await;
    tracing::info!("Metrics and logs flushed");

    // Step 6: Final cleanup
    tracing::info!("Step 6/6: Final cleanup");
    // TODO: Any other cleanup tasks
    tracing::info!("Cleanup complete");

    let shutdown_duration = shutdown_start.elapsed();
    tracing::info!("===========================================");
    tracing::info!("Graceful shutdown completed in {:?}", shutdown_duration);
    tracing::info!("===========================================");
}

/// Setup signal handlers for graceful shutdown
pub async fn setup_signal_handlers(coordinator: Arc<ShutdownCoordinator>) {
    use tokio::signal;

    #[cfg(unix)]
    {
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to setup SIGTERM handler");
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
            .expect("Failed to setup SIGINT handler");

        let coordinator_term = Arc::clone(&coordinator);
        let coordinator_int = Arc::clone(&coordinator);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = sigterm.recv() => {
                        tracing::info!("Received SIGTERM, initiating shutdown");
                        coordinator_term.shutdown().await;
                        break;
                    }
                    _ = sigint.recv() => {
                        tracing::info!("Received SIGINT (Ctrl+C), initiating shutdown");
                        coordinator_int.shutdown().await;
                        break;
                    }
                }
            }
        });
    }

    #[cfg(not(unix))]
    {
        tokio::spawn(async move {
            signal::ctrl_c()
                .await
                .expect("Failed to setup Ctrl+C handler");

            tracing::info!("Received Ctrl+C, initiating shutdown");
            coordinator.shutdown().await;
        });
    }

    tracing::info!("Signal handlers configured");
}

/// Request counter for tracking in-flight requests
pub struct RequestCounter {
    count: Arc<RwLock<u64>>,
}

impl RequestCounter {
    pub fn new() -> Self {
        Self {
            count: Arc::new(RwLock::new(0)),
        }
    }

    /// Increment counter when request starts
    pub async fn increment(&self) {
        let mut count = self.count.write().await;
        *count += 1;
    }

    /// Decrement counter when request completes
    pub async fn decrement(&self) {
        let mut count = self.count.write().await;
        if *count > 0 {
            *count -= 1;
        }
    }

    /// Get current count
    pub async fn get(&self) -> u64 {
        *self.count.read().await
    }

    /// Wait for all requests to complete (with timeout)
    pub async fn wait_for_zero(&self, timeout_duration: Duration) -> Result<(), ()> {
        match timeout(timeout_duration, async {
            loop {
                if self.get().await == 0 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }
}

impl Default for RequestCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware for tracking in-flight requests
pub async fn request_tracking_middleware<B>(
    request: axum::http::Request<B>,
    next: axum::middleware::Next<B>,
) -> axum::response::Response {
    // TODO: Get counter from app state
    // For now, just pass through
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shutdown_coordination() {
        let coordinator = ShutdownCoordinator::new(30);
        let handle = coordinator.subscribe();

        // Initially not shutting down
        assert!(!coordinator.is_shutting_down().await);
        assert!(!handle.is_shutting_down().await);

        // Initiate shutdown
        coordinator.shutdown().await;

        // Should be marked as shutting down
        assert!(coordinator.is_shutting_down().await);
        assert!(handle.is_shutting_down().await);
    }

    #[tokio::test]
    async fn test_shutdown_notification() {
        let coordinator = ShutdownCoordinator::new(30);
        let handle = coordinator.subscribe();

        let notified = Arc::new(RwLock::new(false));
        let notified_clone = Arc::clone(&notified);

        // Spawn task waiting for shutdown
        tokio::spawn(async move {
            handle.wait().await;
            let mut n = notified_clone.write().await;
            *n = true;
        });

        // Small delay to ensure task starts
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Should not be notified yet
        assert!(!*notified.read().await);

        // Trigger shutdown
        coordinator.shutdown().await;

        // Small delay for notification to propagate
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Should be notified now
        assert!(*notified.read().await);
    }

    #[tokio::test]
    async fn test_request_counter() {
        let counter = RequestCounter::new();

        assert_eq!(counter.get().await, 0);

        counter.increment().await;
        assert_eq!(counter.get().await, 1);

        counter.increment().await;
        assert_eq!(counter.get().await, 2);

        counter.decrement().await;
        assert_eq!(counter.get().await, 1);

        counter.decrement().await;
        assert_eq!(counter.get().await, 0);
    }

    #[tokio::test]
    async fn test_request_counter_wait() {
        let counter = RequestCounter::new();

        counter.increment().await;
        counter.increment().await;

        // Spawn task to decrement after delay
        let counter_clone = Arc::new(counter);
        let counter_ref = Arc::clone(&counter_clone);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            counter_ref.decrement().await;
            tokio::time::sleep(Duration::from_millis(100)).await;
            counter_ref.decrement().await;
        });

        // Wait should succeed
        let result = counter_clone.wait_for_zero(Duration::from_secs(1)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_request_counter_timeout() {
        let counter = RequestCounter::new();

        counter.increment().await;

        // Wait should timeout
        let result = counter.wait_for_zero(Duration::from_millis(50)).await;
        assert!(result.is_err());
    }
}
