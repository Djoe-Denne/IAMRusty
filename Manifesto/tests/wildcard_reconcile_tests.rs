//! Sweep leftover `viewer@user:*` tuples with the same ops as sentinel-sync.

mod common;

use common::*;
use rustycog::permission::Permission;
use serial_test::serial;
use uuid::Uuid;

fn project_resource(project_id: Uuid) -> rustycog::permission::ResourceRef {
    rustycog::permission::ResourceRef::new("project", project_id)
}

#[tokio::test]
#[serial]
async fn reconcile_wildcards_deletes_stale_public_tuple() {
    let (_fixture, _base_url, _client, openfga, _components) =
        setup_test_server().await.expect("setup");
    let stale_id = Uuid::new_v4();
    let keep_id = Uuid::new_v4();

    openfga
        .allow_wildcard(Permission::Read, project_resource(stale_id))
        .await
        .expect("stale wildcard");
    openfga
        .allow_wildcard(Permission::Read, project_resource(keep_id))
        .await
        .expect("kept wildcard");

    let desired = vec![(stale_id, false), (keep_id, true)];
    for (project_id, wants_wildcard) in desired {
        let object = format!("project:{project_id}");
        if wants_wildcard {
            let _ = openfga
                .write_tuple("user:*", "viewer", object.as_str())
                .await;
        } else {
            openfga
                .delete_tuple("user:*", "viewer", object.as_str())
                .await
                .expect("delete stale wildcard");
        }
    }

    let leftover = openfga
        .read_tuples(
            Some("user:*"),
            Some("viewer"),
            Some(&format!("project:{stale_id}")),
        )
        .await
        .expect("read stale");
    assert!(leftover.is_empty(), "non-public wildcard must be gone");

    let kept = openfga
        .read_tuples(
            Some("user:*"),
            Some("viewer"),
            Some(&format!("project:{keep_id}")),
        )
        .await
        .expect("read kept");
    assert!(!kept.is_empty(), "public wildcard must remain");
}
