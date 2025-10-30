use governor::clock::{Clock, QuantaClock};
use governor::middleware::NoOpMiddleware;
use governor::state::keyed::DefaultKeyedStateStore;
use governor::{Quota, RateLimiter};
use nonzero_ext::nonzero;
use serde::{Deserialize, Serialize};
use std::num::{NonZero, NonZeroU32};
use std::time::Duration;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct Policy {
    pub enabled: bool,
    pub per_seconds: u64,
    pub burst: NonZeroU32,
}

impl Default for Policy {
    fn default() -> Self {
        Self::new(10, nonzero!(3u32))
    }
}

impl Policy {
    pub fn new(per_seconds: u64, burst: NonZeroU32) -> Self {
        Self {
            enabled: true,
            per_seconds,
            burst,
        }
    }

    pub fn disabled(self) -> Self {
        Self {
            enabled: false,
            ..self
        }
    }

    pub fn quota(&self) -> Quota {
        Quota::with_period(Duration::from_secs(self.per_seconds.max(1)))
            .expect("invalid policy period")
            .allow_burst(self.burst)
    }
}

type KeyedLimiter<K> = RateLimiter<K, DefaultKeyedStateStore<K>, QuantaClock, NoOpMiddleware>;
pub type Key = String;

#[derive(Debug, Default)]
pub struct RateLimiters {
    call_invite: Option<KeyedLimiter<Key>>,
    call_invite_per_minute: Option<KeyedLimiter<Key>>,
    failed_auth: Option<KeyedLimiter<Key>>,
    failed_auth_per_minute: Option<KeyedLimiter<Key>>,
}

impl RateLimiters {
    #[inline]
    pub fn check_call_invite(&self, key: &Key) -> Result<(), Duration> {
        tracing::trace!("check_call_invite: {:?}", key);
        Self::check(&self.call_invite_per_minute, key)
            .and_then(|_| Self::check(&self.call_invite, key))
    }

    #[inline]
    pub fn check_failed_auth(&self, key: &Key) -> Result<(), Duration> {
        tracing::trace!("check_failed_auth: {:?}", key);
        Self::check(&self.failed_auth_per_minute, key)
            .and_then(|_| Self::check(&self.failed_auth, key))
    }

    #[inline]
    fn check(limiter: &Option<KeyedLimiter<Key>>, key: &Key) -> Result<(), Duration> {
        if let Some(limiter) = limiter {
            tracing::trace!("check: {:?}", key);
            limiter
                .check_key(key)
                .map_err(|not_until| not_until.wait_time_from(limiter.clock().now()))
        } else {
            tracing::trace!("no limiter: {:?}", key);
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct RateLimitersConfig {
    pub enabled: bool,
    pub call_invite: Policy,
    pub call_invite_per_minute: u32,
    pub failed_auth: Policy,
    pub failed_auth_per_minute: u32,
}

impl Default for RateLimitersConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            call_invite: Policy::new(10, nonzero!(3u32)),
            call_invite_per_minute: 20,
            failed_auth: Policy::new(60, nonzero!(5u32)).disabled(),
            failed_auth_per_minute: 0, // 60
        }
    }
}

impl From<RateLimitersConfig> for RateLimiters {
    fn from(value: RateLimitersConfig) -> Self {
        if !value.enabled {
            return Self {
                call_invite: None,
                call_invite_per_minute: None,
                failed_auth: None,
                failed_auth_per_minute: None,
            };
        }

        let call_invite = if value.call_invite.enabled {
            Some(KeyedLimiter::<Key>::keyed(value.call_invite.quota()))
        } else {
            None
        };
        let call_invite_per_minute = if value.call_invite_per_minute > 0 {
            let val =
                NonZero::new(value.call_invite_per_minute).expect("invalid call_invite_per_minute");
            Some(KeyedLimiter::<Key>::keyed(
                Quota::per_minute(val).allow_burst(val),
            ))
        } else {
            None
        };

        let failed_auth = if value.failed_auth.enabled {
            Some(KeyedLimiter::<Key>::keyed(value.failed_auth.quota()))
        } else {
            None
        };
        let failed_auth_per_minute = if value.failed_auth_per_minute > 0 {
            let val =
                NonZero::new(value.failed_auth_per_minute).expect("invalid failed_auth_per_minute");
            Some(KeyedLimiter::<Key>::keyed(
                Quota::per_minute(val).allow_burst(val),
            ))
        } else {
            None
        };

        Self {
            call_invite,
            call_invite_per_minute,
            failed_auth,
            failed_auth_per_minute,
        }
    }
}
