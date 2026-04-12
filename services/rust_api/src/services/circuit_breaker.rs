use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct CircuitBreaker {
    state: Arc<CircuitBreakerState>,
}

struct CircuitBreakerState {
    is_open: AtomicBool,
    failure_count: AtomicU64,
    last_failure: Mutex<Instant>,
    reset_timeout: Duration,
    failure_threshold: u64,
}

impl CircuitBreaker {
    pub fn new(reset_timeout: Duration, failure_threshold: u64) -> Self {
        CircuitBreaker {
            state: Arc::new(CircuitBreakerState {
                is_open: AtomicBool::new(false),
                failure_count: AtomicU64::new(0),
                last_failure: Mutex::new(Instant::now()),
                reset_timeout,
                failure_threshold,
            }),
        }
    }

    pub async fn record_success(&self) {
        self.state.failure_count.store(0, Ordering::SeqCst);
        self.state.is_open.store(false, Ordering::SeqCst);
    }

    pub async fn record_failure(&self) {
        let mut last_failure = self.state.last_failure.lock().await;
        *last_failure = Instant::now();

        let current_failures = self.state.failure_count.fetch_add(1, Ordering::SeqCst);
        if current_failures + 1 >= self.state.failure_threshold {
            self.state.is_open.store(true, Ordering::SeqCst);
            tracing::warn!(
                "Circuit breaker opened due to {} failures",
                current_failures + 1
            );
        }
    }

    pub async fn can_execute(&self) -> bool {
        if !self.state.is_open.load(Ordering::SeqCst) {
            return true;
        }

        let last_failure = self.state.last_failure.lock().await;
        if last_failure.elapsed() >= self.state.reset_timeout {
            // Try to reset after timeout
            self.state.is_open.store(false, Ordering::SeqCst);
            self.state.failure_count.store(0, Ordering::SeqCst);
            tracing::info!("Circuit breaker reset after timeout");
            true
        } else {
            false
        }
    }
}
