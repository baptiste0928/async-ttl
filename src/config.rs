//! Configuration of an [`AsyncTtl`] cache.
//!
//! [`AsyncTtl`]: crate::AsyncTtl

use std::time::Duration;

const DEFAULT_EMPTY_DELAY: Duration = Duration::from_millis(100);
const DEFAULT_DELTA_DELAY: Duration = Duration::from_millis(5);

/// Configuration of an [`AsyncTtl`] cache.
///
/// [`AsyncTtl`]: crate::AsyncTtl
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsyncTtlConfig {
    /// Expiration delay of entries.
    pub expires_after: Duration,
    /// Delay between two checks if the expiration queue is empty.
    ///
    /// Defaults to 100ms.
    pub empty_delay: Duration,
    /// Delay added between each expiration checks.
    ///
    /// This allow to group together expiration of keys with a similar delay.
    /// Setting a large delay lower the accuracy of key expiration. If you want
    /// maximum precision, set this delay to 0.
    ///
    /// Defaults to 5ms.
    pub delta_delay: Duration,
}

impl AsyncTtlConfig {
    pub fn new(expires_after: Duration) -> Self {
        Self {
            expires_after,
            empty_delay: DEFAULT_EMPTY_DELAY,
            delta_delay: DEFAULT_DELTA_DELAY,
        }
    }

    pub fn builder(expires_after: Duration) -> AsyncTtlConfigBuilder {
        AsyncTtlConfigBuilder::new(expires_after)
    }
}

/// Builder for [`AsyncTtlConfig`].
pub struct AsyncTtlConfigBuilder {
    expires_after: Duration,
    empty_delay: Option<Duration>,
    delta_delay: Option<Duration>,
}

impl AsyncTtlConfigBuilder {
    fn new(expires_after: Duration) -> Self {
        Self {
            expires_after,
            empty_delay: None,
            delta_delay: None,
        }
    }

    pub fn empty_delay(mut self, empty_delay: Duration) -> Self {
        self.empty_delay = Some(empty_delay);

        self
    }

    pub fn delta_delay(mut self, delta_delay: Duration) -> Self {
        self.delta_delay = Some(delta_delay);

        self
    }

    pub fn build(self) -> AsyncTtlConfig {
        AsyncTtlConfig {
            expires_after: self.expires_after,
            empty_delay: self.empty_delay.unwrap_or(DEFAULT_EMPTY_DELAY),
            delta_delay: self.delta_delay.unwrap_or(DEFAULT_DELTA_DELAY),
        }
    }
}
