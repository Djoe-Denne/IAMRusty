use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant},
};

use axum::{
    extract::{rejection::JsonRejection, Extension},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::rate_limit::constant_time_token_eq;

const CHECKPOINT_SPACING: Duration = Duration::from_millis(50);
const WRITE_BUDGET: Duration = Duration::from_millis(300);

type MarkerWrite = Pin<Box<dyn Future<Output = bool> + Send>>;

pub struct SessionCheckpoint {
    token: String,
    next_dispatch: Mutex<Instant>,
    writer: Box<dyn Fn(Uuid) -> MarkerWrite + Send + Sync>,
    capacity: Semaphore,
}

impl SessionCheckpoint {
    /// # Errors
    ///
    /// Returns an error when the checkpoint token is invalid or is also the
    /// general IAM internal RPC token.
    pub fn new<F, W>(
        token: String,
        internal_service_token: &str,
        writer: W,
    ) -> Result<Self, &'static str>
    where
        F: Future<Output = bool> + Send + 'static,
        W: Fn(Uuid) -> F + Send + Sync + 'static,
    {
        if token == internal_service_token {
            return Err("checkpoint capability must be dedicated");
        }
        if token.len() < 32 || !token.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err("checkpoint capability must contain at least 32 visible ASCII bytes");
        }
        Ok(Self {
            token,
            next_dispatch: Mutex::new(Instant::now()),
            writer: Box::new(move |challenge| Box::pin(writer(challenge))),
            capacity: Semaphore::new(2),
        })
    }

    fn reserve_dispatch(&self) -> bool {
        let now = Instant::now();
        let mut next = self
            .next_dispatch
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if now < *next {
            return false;
        }
        *next = now + CHECKPOINT_SPACING;
        true
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointRequest {
    challenge: Uuid,
}

pub async fn session_checkpoint(
    Extension(checkpoint): Extension<Option<Arc<SessionCheckpoint>>>,
    headers: HeaderMap,
    request: Result<Json<CheckpointRequest>, JsonRejection>,
) -> StatusCode {
    let Some(checkpoint) = checkpoint else {
        return StatusCode::FORBIDDEN;
    };
    let presented = headers
        .get("x-iam-checkpoint-token")
        .map_or(&[][..], |value| value.as_bytes());
    if !constant_time_token_eq(presented, checkpoint.token.as_bytes()) {
        return StatusCode::FORBIDDEN;
    }
    let Ok(Json(request)) = request else {
        return StatusCode::BAD_REQUEST;
    };
    let Ok(_permit) = checkpoint.capacity.try_acquire() else {
        return StatusCode::SERVICE_UNAVAILABLE;
    };
    if !checkpoint.reserve_dispatch() {
        return StatusCode::SERVICE_UNAVAILABLE;
    }
    if tokio::time::timeout(WRITE_BUDGET, (checkpoint.writer)(request.challenge))
        .await
        .is_ok_and(|committed| committed)
    {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

#[cfg(test)]
mod tests {
    use super::{session_checkpoint, CheckpointRequest, SessionCheckpoint};
    use axum::{
        extract::Extension,
        http::{HeaderMap, HeaderValue, StatusCode},
        Json,
    };
    use std::{
        future::ready,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
    };
    use uuid::Uuid;

    const FIXTURE_CAPABILITY: &str = "fixture-checkpoint-capability-only";

    #[test]
    fn checkpoint_capability_must_differ_from_internal_token() {
        let result = SessionCheckpoint::new(FIXTURE_CAPABILITY.into(), FIXTURE_CAPABILITY, |_| {
            ready(true)
        });
        assert!(matches!(
            result,
            Err("checkpoint capability must be dedicated")
        ));
    }

    #[tokio::test]
    async fn saturated_checkpoint_does_not_attempt_a_marker_write() {
        let called = Arc::new(AtomicBool::new(false));
        let writer_called = called.clone();
        let checkpoint = Arc::new(
            SessionCheckpoint::new(FIXTURE_CAPABILITY.into(), "", move |_| {
                writer_called.store(true, Ordering::SeqCst);
                ready(true)
            })
            .expect("checkpoint fixture"),
        );
        let _held = checkpoint
            .capacity
            .try_acquire_many(2)
            .expect("reserve both slots");
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-iam-checkpoint-token",
            HeaderValue::from_static(FIXTURE_CAPABILITY),
        );
        let result = session_checkpoint(
            Extension(Some(checkpoint.clone())),
            headers,
            Ok(Json(CheckpointRequest {
                challenge: Uuid::new_v4(),
            })),
        )
        .await;
        assert_eq!(result, StatusCode::SERVICE_UNAVAILABLE);
        assert!(
            !called.load(Ordering::SeqCst),
            "capacity rejection never starts a write"
        );
    }
}
