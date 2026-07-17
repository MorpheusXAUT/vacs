use std::time::Duration;

pub(crate) struct BackoffStrategy {
    base: Duration,
    max: Duration,
    multiplier: f32,
}

impl BackoffStrategy {
    pub fn new(base: Duration, max: Duration, multiplier: f32) -> Self {
        Self {
            base,
            max,
            multiplier,
        }
    }

    pub fn timeout(&self, attempt: usize) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(0);
        } else if attempt == 1 {
            return self.base;
        }

        Duration::from_secs_f32(
            self.base.as_secs_f32() * self.multiplier.powi((attempt - 1) as i32),
        )
        .min(self.max)
    }
}
