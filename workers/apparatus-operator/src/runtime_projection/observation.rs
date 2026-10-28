//! Bounded Pod inventory/registration iteration. Cursor/offset are scheduling
//! hints only: no absence-derived revocation, identity, permission or deletion.
use super::*;
use crate::runtime_observation::RuntimePodObserver;
use apparatus_contracts::runtime::{RuntimeServiceRoleV1, RuntimeWriteAckV1};
use std::time::Duration;
use tokio::time::{timeout_at, Instant};

#[derive(Default)]
pub struct RuntimePodObservationCursor {
    continue_token: Option<String>,
    offset: usize,
}

impl ServingRuntimeBridge {
    /// One <=2s read capture followed by <=2s writer consultation/registration.
    /// Each Pod is fully re-proven before registration. Slow work is suspended,
    /// not promoted to a fresh heartbeat or retried with a different instance ID.
    pub async fn observe_instances_once(
        &self,
        project_id: uuid::Uuid,
        binding_id: uuid::Uuid,
        progress: &mut RuntimePodObservationCursor,
    ) -> Result<Vec<RuntimeWriteAckV1>, RuntimeErrorCodeV1> {
        let caller = RuntimeControlCallerV1 {
            role: RuntimeServiceRoleV1::Projector,
        };
        let selector = RuntimeInstanceSelectorV1::Page {
            cursor: None,
            page_size: Some(1),
        };
        let read_deadline = Instant::now() + Duration::from_secs(2);
        let (snapshot, lease, page) = timeout_at(read_deadline, async {
            let snapshot = self
                .control
                .get_binding(&caller, project_id, binding_id, &selector)
                .await
                .map_err(|error| error.code)?;
            if snapshot.state != RuntimeBindingStateV1::Active {
                return Err(RuntimeErrorCodeV1::RuntimeBindingClosed);
            }
            let observation = snapshot
                .observation
                .as_ref()
                .ok_or(RuntimeErrorCodeV1::RuntimeProjectionUnavailable)?;
            let image = self.preparation_image(&snapshot).await?;
            let lease = self
                .control
                .writer_lease(
                    &caller,
                    project_id,
                    binding_id,
                    &RuntimeWriterLeaseRequestV1 {
                        schema_version: 1,
                        holder_id: self.holder_id,
                        expected: snapshot.cas.clone(),
                        renew: None,
                    },
                )
                .await
                .map_err(|error| error.code)?;
            let observer = RuntimePodObserver::new(self.parents.parents.clone().into_client());
            let page = observer
                .observe_page(
                    &snapshot,
                    &observation.service_uid,
                    &observation.revision_name,
                    &image.image,
                    progress.continue_token.as_deref(),
                )
                .await?;
            self.require_same_admission(&image).await?;
            Ok((snapshot, lease, page))
        })
        .await
        .map_err(|_| RuntimeErrorCodeV1::RuntimeProjectionUnavailable)??;
        if progress.offset >= page.items.len() {
            progress.offset = 0;
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut pending_map = timeout_at(deadline, self.pending_instances.lock())
            .await
            .map_err(|_| RuntimeErrorCodeV1::RuntimeDeadlineExceeded)?;
        let mut acks = Vec::new();
        while progress.offset < page.items.len() {
            let pod = &page.items[progress.offset];
            let key = (project_id, binding_id, pod.identity().pod_uid.clone());
            let mut pending = pending_map.get(&key).copied();
            // Keeping an uncertain ID is not retaining an authority snapshot.
            // Limit process memory; an exhausted map closes new registration.
            if pending.is_none() && pending_map.len() >= 2000 {
                return Err(RuntimeErrorCodeV1::RuntimeCapacityExhausted);
            }
            let result = timeout_at(
                deadline,
                pod.register_with_pending(
                    self.control.as_ref(),
                    &caller,
                    &snapshot,
                    &lease,
                    &mut pending,
                ),
            )
            .await;
            if let Some(id) = pending {
                pending_map.insert(key.clone(), id);
            }
            progress.offset += 1;
            match result {
                Ok(Ok(ack)) => {
                    pending_map.remove(&key);
                    acks.push(ack);
                }
                Ok(Err(error)) => return Err(error),
                Err(_) => return Err(RuntimeErrorCodeV1::RuntimeAuthorityUnavailable),
            }
        }
        progress.continue_token = page.next_continue;
        progress.offset = 0;
        Ok(acks)
    }
}
