//! Instance-local JWKS snapshots and cancel-safe, request-owned refresh.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use idp_connect_contract::FederatedOAuthError;
use openidconnect::{core::CoreJsonWebKeySet, JsonWebKeySetUrl};
use tokio::{sync::Mutex as Flight, time::Instant};

use super::{callback::BoundHttp, MonotonicNow};

const TTL: Duration = Duration::from_secs(300);
const COOLDOWN: Duration = Duration::from_secs(30);

pub(super) struct Snapshot {
    pub(super) keys: Arc<CoreJsonWebKeySet>,
    pub(super) generation: u64,
    fresh: bool,
    succeeded: bool,
}

struct State {
    keys: Arc<CoreJsonWebKeySet>,
    success: Instant,
    attempted: Instant,
    generation: u64,
    succeeded: bool,
}

pub(super) struct KeyCache {
    state: Mutex<State>,
    flight: Flight<()>,
    now: MonotonicNow,
}

impl KeyCache {
    pub(super) fn new(keys: CoreJsonWebKeySet, now: MonotonicNow) -> Self {
        let initialized = now();
        Self {
            state: Mutex::new(State {
                keys: Arc::new(keys),
                success: initialized,
                attempted: initialized,
                generation: 0,
                succeeded: true,
            }),
            flight: Flight::new(()),
            now,
        }
    }

    fn snapshot(&self) -> Result<Snapshot, FederatedOAuthError> {
        let state = self
            .state
            .lock()
            .map_err(|_| FederatedOAuthError::UserProfile)?;
        let age = (self.now)()
            .checked_duration_since(state.success)
            .ok_or(FederatedOAuthError::UserProfile)?;
        Ok(Snapshot {
            keys: Arc::clone(&state.keys),
            generation: state.generation,
            fresh: age < TTL,
            succeeded: state.succeeded,
        })
    }

    pub(super) async fn current(
        &self,
        http: &BoundHttp,
        uri: &JsonWebKeySetUrl,
    ) -> Result<Snapshot, FederatedOAuthError> {
        let snapshot = self.snapshot()?;
        if snapshot.fresh {
            return Ok(snapshot);
        }
        self.refresh(snapshot.generation, http, uri).await
    }

    pub(super) async fn refresh(
        &self,
        observed: u64,
        http: &BoundHttp,
        uri: &JsonWebKeySetUrl,
    ) -> Result<Snapshot, FederatedOAuthError> {
        // This guard alone spans I/O. Dropping the owned request releases it;
        // no in-flight flag/task/notify survives cancellation.
        let _flight = self.flight.lock().await;
        let snapshot = self.snapshot()?;
        if snapshot.generation != observed {
            return if snapshot.succeeded && snapshot.fresh {
                Ok(snapshot)
            } else {
                Err(FederatedOAuthError::UserProfile)
            };
        }
        self.record_attempt()?;
        let fetched = CoreJsonWebKeySet::fetch_async(uri, http).await;
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| FederatedOAuthError::UserProfile)?;
            state.generation = state.generation.saturating_add(1);
            state.succeeded = false;
            if let Ok(keys) = &fetched {
                if !keys.keys().is_empty() {
                    state.keys = Arc::new(keys.clone());
                    state.success = (self.now)();
                    state.succeeded = true;
                }
            }
            if !state.succeeded {
                return Err(FederatedOAuthError::UserProfile);
            }
        }
        self.snapshot()
    }

    fn record_attempt(&self) -> Result<(), FederatedOAuthError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| FederatedOAuthError::UserProfile)?;
        let now = (self.now)();
        if now
            .checked_duration_since(state.attempted)
            .ok_or(FederatedOAuthError::UserProfile)?
            < COOLDOWN
        {
            return Err(FederatedOAuthError::UserProfile);
        }
        state.attempted = now;
        state.generation = state.generation.saturating_add(1);
        state.succeeded = false;
        drop(state);
        Ok(())
    }
}
