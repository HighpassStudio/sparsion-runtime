use chrono::{DateTime, Utc};
use std::sync::Mutex;

/// Abstraction over time — enables deterministic testing of decay and salience.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// Uses real system time.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Controllable clock for tests. Advance time without waiting.
pub struct MockClock {
    current: Mutex<DateTime<Utc>>,
}

impl MockClock {
    pub fn new(start: DateTime<Utc>) -> Self {
        Self {
            current: Mutex::new(start),
        }
    }

    pub fn now_fixed() -> Self {
        Self::new(Utc::now())
    }

    pub fn advance(&self, duration: chrono::Duration) {
        let mut current = self.current.lock().unwrap();
        *current = *current + duration;
    }

    pub fn set(&self, time: DateTime<Utc>) {
        let mut current = self.current.lock().unwrap();
        *current = time;
    }
}

impl Clock for MockClock {
    fn now(&self) -> DateTime<Utc> {
        *self.current.lock().unwrap()
    }
}
