//! # async-ttl
//!
//! Cache with asynchronous locking and key expiration with a fixed time-to-live.
//!
//! This crate provides an [`AsyncTtl`] type that represent a cache where each
//! entry expires after a fixed amount of type. It support asynchronous with
//! [tokio] and allow to use custom cache types that implement the [`CacheMap`]
//! trait.
//!
//! Implementations are provided for [`HashMap`] and [`BTreeMap`].
//!
//! ## Usage
//! When creating a new [`AsyncTtl`] cache, the method returns the created cache
//! wrapped in an [`Arc`] and a [`AsyncTtlExpireTask`]. This task must be
//! started for the expired key eviction to work.
//!
//! ### Key eviction
//! The background task automatically removes expired keys from the cache.
//! The algorithm used is the following:
//!
//! - Get the next entry in the expiration queue.
//!   - If an entry is present, wait until its expiration + `delta_delay`
//!     (defaults to 5ms) and delete all expired keys. This allow to group
//!     together expiration of keys inserted in a short time window without
//!     locking the cache in loop.
//!   - If no entry is present, wait `empty_delay` (defaults to 100ms).
//! - Do the previous steps indefinitely.
//!
//! ### Alternatives
//! This crate only support using a **fixed** TTL, which reduce the cost of
//! expired keys eviction and prevents from having expired keys still in the
//! memory. If you need a variable TTL, consider the [retainer] crate.
//!
//! [`HashMap`]: std::collections::HashMap
//! [`BTreeMap`]: std::collections::BTreeMap
//! [retainer]: https://crates.io/crates/retainer

pub mod config;
mod map;

pub use map::CacheMap;

use std::{collections::VecDeque, marker::PhantomData, sync::Arc, time::Duration};

use tokio::{
    sync::{RwLock, RwLockReadGuard},
    time::{self, Instant},
};

use crate::config::AsyncTtlConfig;

/// Async cache with TTL.
///
/// This type provides a cache structure with asynchronous locking and key
/// expiration with a fixed time-to-live.
///
/// See the [crate] documentation to learn more.
#[derive(Debug)]
pub struct AsyncTtl<T, K, V>
where
    T: CacheMap<K, V> + Default,
    K: Clone,
{
    /// Expiration queue.
    expires: RwLock<VecDeque<EntryExpire<K>>>,
    /// Inner cache data.
    data: RwLock<T>,
    /// Cache configuration.
    config: AsyncTtlConfig,
    /// Required for the `V` generic parameter.
    _value: PhantomData<V>,
}

impl<T, K, V> AsyncTtl<T, K, V>
where
    T: CacheMap<K, V> + Default,
    K: Clone,
{
    /// Initialize a new [`AsyncTtl`] cache.
    ///
    /// This method returns the cache wrapped in an [`Arc`] and the expiration
    /// task.
    pub fn new(config: AsyncTtlConfig) -> (Arc<Self>, AsyncTtlExpireTask<T, K, V>) {
        let cache = Arc::new(Self {
            expires: Default::default(),
            data: Default::default(),
            config,
            _value: PhantomData,
        });

        (cache.clone(), AsyncTtlExpireTask::new(cache))
    }

    /// Returns a read-only access to the underlying stored data.
    pub async fn read(&self) -> RwLockReadGuard<'_, T> {
        self.data.read().await
    }

    /// Inserts a new entry into the cache.
    pub async fn insert(&self, key: K, value: V) {
        // Acquire write locks. Locks are acquired in this order to avoid
        // deadlocks with the expiration tasks.
        let mut expires = self.expires.write().await;
        let mut data = self.data.write().await;

        data.insert_cache(key.clone(), value);
        expires.push_back(EntryExpire::new(
            key,
            Instant::now(),
            self.config.expires_after,
        ));
    }
}

#[derive(Debug)]
struct EntryExpire<K> {
    key: K,
    created_at: Instant,
    expires_after: Duration,
}

impl<K> EntryExpire<K> {
    /// Initialize a new [`KeyExpire`].
    fn new(key: K, created_at: Instant, expires_after: Duration) -> Self {
        Self {
            key,
            created_at,
            expires_after,
        }
    }

    /// Returns when the entry expires.
    ///
    /// If the entry has already expired, a zero duration is returned.
    fn expires_in(&self) -> Duration {
        let elapsed = self.created_at.elapsed();

        // Computes EXPIRES_AFTER - elapsed, returning zero if resulting in
        // a negative duration (already expired)
        self.expires_after.saturating_sub(elapsed)
    }
}

/// [`AsyncTtl`] expiration task.
///
/// This type represent the expiration task of a cache and must be started
/// to ensure expired keys are removed.
///
/// See the [crate] documentation to learn more.
pub struct AsyncTtlExpireTask<T, K, V>
where
    T: CacheMap<K, V> + Default,
    K: Clone,
{
    cache: Arc<AsyncTtl<T, K, V>>,
}

impl<T, K, V> AsyncTtlExpireTask<T, K, V>
where
    T: CacheMap<K, V> + Default,
    K: Clone,
{
    /// Initialize a new [`AsyncTtlExpireTask`].
    pub fn new(cache: Arc<AsyncTtl<T, K, V>>) -> Self {
        Self { cache }
    }

    /// Start the cache expiration task.
    ///
    /// This task will automatically expire cached values based on the provided
    /// configuration. It contains an infinite loop so you should start it in a
    /// new [tokio] task.
    pub async fn run(&self) {
        loop {
            // Get next expiration time
            let duration = {
                // Explicit scope to ensure the lock is dropped
                let expires = self.cache.expires.read().await;

                match expires.get(0) {
                    Some(expire) => expire.expires_in() + self.cache.config.delta_delay,
                    None => self.cache.config.empty_delay,
                }
            };

            time::sleep(duration).await;

            {
                // Explicit scope to ensure the lock is dropped
                let mut expires = self.cache.expires.write().await;
                let mut data = self.cache.data.write().await;

                // Remove all expired entries
                loop {
                    if !expires
                        .get(0)
                        .map(|entry| entry.expires_in().is_zero())
                        .unwrap_or(false)
                    {
                        break; // Break if the next entry has not expired
                    }

                    // Remove the entry from the cache
                    let entry = expires.pop_front().unwrap(); // SAFETY: if the entry does not exist, the loop is stopped in the before statement
                    data.remove_cache(&entry.key);
                }
            }
        }
    }
}
