// Health Check Endpoints
// Implements liveness, readiness, and startup probes for Kubernetes

use axum::{
    extract::State,
    http::StatusCode,
    Json,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: HealthState,
    pub timestamp: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub checks: Vec<HealthCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: CheckStatus,
    pub message: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

pub struct HealthChecker {
    start_time: Instant,
    version: String,
    database_url: String,
    redis_url: String,
    s3_configured: bool,
    readiness_state: Arc<RwLock<bool>>,
}

impl HealthChecker {
    pub fn new(
        version: String,
        database_url: String,
        redis_url: String,
        s3_configured: bool,
    ) -> Self {
        Self {
            start_time: Instant::now(),
            version,
            database_url,
            redis_url,
            s3_configured,
            readiness_state: Arc::new(RwLock::new(false)),
        }
    }

    /// Mark service as ready to accept traffic
    pub async fn mark_ready(&self) {
        let mut ready = self.readiness_state.write().await;
        *ready = true;
        tracing::info!("Service marked as ready");
    }

    /// Mark service as not ready (during shutdown)
    pub async fn mark_not_ready(&self) {
        let mut ready = self.readiness_state.write().await;
        *ready = false;
        tracing::info!("Service marked as not ready");
    }

    /// Liveness probe - indicates if the process is alive
    /// Kubernetes will restart the pod if this fails
    pub async fn liveness(&self) -> Result<Json<HealthStatus>, StatusCode> {
        let uptime = self.start_time.elapsed().as_secs();

        // Very basic check - just verify process is running
        // Don't check dependencies here, use readiness for that
        let health_status = HealthStatus {
            status: HealthState::Healthy,
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: self.version.clone(),
            uptime_seconds: uptime,
            checks: vec![HealthCheck {
                name: "process".to_string(),
                status: CheckStatus::Pass,
                message: Some("Process is running".to_string()),
                duration_ms: 0,
            }],
        };

        Ok(Json(health_status))
    }

    /// Readiness probe - indicates if service can handle traffic
    /// Kubernetes will remove pod from service if this fails
    pub async fn readiness(&self) -> Result<Json<HealthStatus>, (StatusCode, Json<HealthStatus>)> {
        let uptime = self.start_time.elapsed().as_secs();
        let mut checks = Vec::new();
        let mut overall_status = HealthState::Healthy;

        // Check if manually marked as not ready (during shutdown)
        let is_ready = *self.readiness_state.read().await;
        if !is_ready {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(HealthStatus {
                    status: HealthState::Unhealthy,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    version: self.version.clone(),
                    uptime_seconds: uptime,
                    checks: vec![HealthCheck {
                        name: "readiness".to_string(),
                        status: CheckStatus::Fail,
                        message: Some("Service not ready (shutting down)".to_string()),
                        duration_ms: 0,
                    }],
                }),
            ));
        }

        // Check database connectivity
        let db_check = self.check_database().await;
        if db_check.status == CheckStatus::Fail {
            overall_status = HealthState::Unhealthy;
        } else if db_check.status == CheckStatus::Warn {
            overall_status = HealthState::Degraded;
        }
        checks.push(db_check);

        // Check Redis connectivity
        let redis_check = self.check_redis().await;
        if redis_check.status == CheckStatus::Fail {
            // Redis failure is degraded, not unhealthy (we can operate without cache)
            if overall_status == HealthState::Healthy {
                overall_status = HealthState::Degraded;
            }
        }
        checks.push(redis_check);

        // Check S3 connectivity (optional)
        if self.s3_configured {
            let s3_check = self.check_s3().await;
            if s3_check.status == CheckStatus::Fail {
                // S3 failure is degraded, not unhealthy
                if overall_status == HealthState::Healthy {
                    overall_status = HealthState::Degraded;
                }
            }
            checks.push(s3_check);
        }

        let health_status = HealthStatus {
            status: overall_status.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: self.version.clone(),
            uptime_seconds: uptime,
            checks,
        };

        if overall_status == HealthState::Unhealthy {
            Err((StatusCode::SERVICE_UNAVAILABLE, Json(health_status)))
        } else {
            Ok(Json(health_status))
        }
    }

    /// Startup probe - indicates if service has finished initialization
    /// Kubernetes will wait for this before checking liveness/readiness
    pub async fn startup(&self) -> Result<Json<HealthStatus>, (StatusCode, Json<HealthStatus>)> {
        let uptime = self.start_time.elapsed().as_secs();

        // Service is considered started once it's marked ready
        let is_ready = *self.readiness_state.read().await;

        let health_status = if is_ready {
            HealthStatus {
                status: HealthState::Healthy,
                timestamp: chrono::Utc::now().to_rfc3339(),
                version: self.version.clone(),
                uptime_seconds: uptime,
                checks: vec![HealthCheck {
                    name: "startup".to_string(),
                    status: CheckStatus::Pass,
                    message: Some("Service initialization complete".to_string()),
                    duration_ms: 0,
                }],
            }
        } else {
            HealthStatus {
                status: HealthState::Unhealthy,
                timestamp: chrono::Utc::now().to_rfc3339(),
                version: self.version.clone(),
                uptime_seconds: uptime,
                checks: vec![HealthCheck {
                    name: "startup".to_string(),
                    status: CheckStatus::Fail,
                    message: Some("Service still initializing".to_string()),
                    duration_ms: 0,
                }],
            }
        };

        if is_ready {
            Ok(Json(health_status))
        } else {
            Err((StatusCode::SERVICE_UNAVAILABLE, Json(health_status)))
        }
    }

    /// Check database connectivity
    async fn check_database(&self) -> HealthCheck {
        let start = Instant::now();

        match self.test_database_connection().await {
            Ok(_) => HealthCheck {
                name: "database".to_string(),
                status: CheckStatus::Pass,
                message: Some("Database connection healthy".to_string()),
                duration_ms: start.elapsed().as_millis() as u64,
            },
            Err(e) => HealthCheck {
                name: "database".to_string(),
                status: CheckStatus::Fail,
                message: Some(format!("Database connection failed: {}", e)),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        }
    }

    /// Check Redis connectivity
    async fn check_redis(&self) -> HealthCheck {
        let start = Instant::now();

        match self.test_redis_connection().await {
            Ok(_) => HealthCheck {
                name: "redis".to_string(),
                status: CheckStatus::Pass,
                message: Some("Redis connection healthy".to_string()),
                duration_ms: start.elapsed().as_millis() as u64,
            },
            Err(e) => HealthCheck {
                name: "redis".to_string(),
                status: CheckStatus::Warn,
                message: Some(format!("Redis connection degraded: {}", e)),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        }
    }

    /// Check S3 connectivity
    async fn check_s3(&self) -> HealthCheck {
        let start = Instant::now();

        match self.test_s3_connection().await {
            Ok(_) => HealthCheck {
                name: "s3".to_string(),
                status: CheckStatus::Pass,
                message: Some("S3 connection healthy".to_string()),
                duration_ms: start.elapsed().as_millis() as u64,
            },
            Err(e) => HealthCheck {
                name: "s3".to_string(),
                status: CheckStatus::Warn,
                message: Some(format!("S3 connection degraded: {}", e)),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        }
    }

    /// Test database connection
    async fn test_database_connection(&self) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Implement actual database connection test
        // For now, simulate check
        tokio::time::sleep(Duration::from_millis(5)).await;
        Ok(())
    }

    /// Test Redis connection
    async fn test_redis_connection(&self) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Implement actual Redis connection test
        // For now, simulate check
        tokio::time::sleep(Duration::from_millis(2)).await;
        Ok(())
    }

    /// Test S3 connection
    async fn test_s3_connection(&self) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Implement actual S3 connection test
        // For now, simulate check
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(())
    }
}

// Axum handlers
pub async fn liveness_handler(
    State(health_checker): State<Arc<HealthChecker>>,
) -> impl IntoResponse {
    health_checker.liveness().await
}

pub async fn readiness_handler(
    State(health_checker): State<Arc<HealthChecker>>,
) -> impl IntoResponse {
    match health_checker.readiness().await {
        Ok(response) => (StatusCode::OK, response).into_response(),
        Err((status, response)) => (status, response).into_response(),
    }
}

pub async fn startup_handler(
    State(health_checker): State<Arc<HealthChecker>>,
) -> impl IntoResponse {
    match health_checker.startup().await {
        Ok(response) => (StatusCode::OK, response).into_response(),
        Err((status, response)) => (status, response).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_liveness_always_passes() {
        let checker = HealthChecker::new(
            "1.0.0".to_string(),
            "postgresql://localhost/test".to_string(),
            "redis://localhost".to_string(),
            false,
        );

        let result = checker.liveness().await;
        assert!(result.is_ok());

        let health = result.unwrap();
        assert_eq!(health.0.status, HealthState::Healthy);
    }

    #[tokio::test]
    async fn test_readiness_fails_when_not_ready() {
        let checker = HealthChecker::new(
            "1.0.0".to_string(),
            "postgresql://localhost/test".to_string(),
            "redis://localhost".to_string(),
            false,
        );

        // Should fail before marking ready
        let result = checker.readiness().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_startup_succeeds_after_ready() {
        let checker = HealthChecker::new(
            "1.0.0".to_string(),
            "postgresql://localhost/test".to_string(),
            "redis://localhost".to_string(),
            false,
        );

        // Should fail before ready
        assert!(checker.startup().await.is_err());

        // Mark as ready
        checker.mark_ready().await;

        // Should pass after ready
        assert!(checker.startup().await.is_ok());
    }
}
