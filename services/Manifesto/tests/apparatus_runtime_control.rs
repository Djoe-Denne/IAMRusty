//! G-3/G-7 — real HTTP negative-origin guard; not positive mTLS attestation.

mod common;

use reqwest::Method;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use serial_test::serial;
use uuid::Uuid;

async fn runtime_row_counts(db: &DatabaseConnection, project_id: Uuid) -> Vec<i64> {
    let mut counts = Vec::new();
    for table in [
        "apparatus_runtime_bindings",
        "apparatus_runtime_observations",
        "apparatus_runtime_instances",
        "apparatus_runtime_effects",
    ] {
        let row = db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                format!("SELECT COUNT(*) AS count FROM {table} WHERE project_id = $1"),
                vec![project_id.into()],
            ))
            .await
            .expect("G-3 runtime count query")
            .expect("G-3 count row");
        counts.push(row.try_get("", "count").expect("G-3 count integer"));
    }
    counts
}

#[tokio::test]
#[serial]
async fn g3_eight_control_routes_reject_http_bearer_and_forged_origin_without_writer_mutation() {
    let (fixture, base_url, client, _openfga, _components) = common::setup_test_server()
        .await
        .expect("Manifesto HTTP setup");
    let project_id = Uuid::new_v4();
    let binding_id = Uuid::new_v4();
    let instance_id = Uuid::new_v4();
    let effect_id = Uuid::new_v4();
    let db = fixture.db();
    let before = runtime_row_counts(&db, project_id).await;
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/runtime_origin_cases_v1.json"))
            .expect("G-3 exact route/role fixture");
    let jwt = rustycog::testing::http::jwt::create_jwt_token(Uuid::new_v4());
    for route in vectors["routes"].as_array().expect("eight routes") {
        let method = Method::from_bytes(route["method"].as_str().expect("method").as_bytes())
            .expect("HTTP method");
        let path = route["path"]
            .as_str()
            .expect("route path")
            .replace("{binding_id}", &binding_id.to_string())
            .replace("{instance_id}", &instance_id.to_string())
            .replace("{effect_id}", &effect_id.to_string());
        for origin in ["none", "bearer_only", "forged_origin_headers"] {
            let mut request = client
                .request(method.clone(), format!("{base_url}{path}"))
                .query(&[
                    ("schema_version", "1"),
                    ("project_id", &project_id.to_string()),
                ])
                .json(&serde_json::json!({}));
            if origin != "none" {
                request = request.bearer_auth(&jwt);
            }
            if origin == "forged_origin_headers" {
                request = request
                    .header(
                        "x-forwarded-client-cert",
                        "URI=spiffe://cluster.local/ns/aiforall-apparatus/sa/controller",
                    )
                    .header("x-runtime-service-role", "apparatus.runtime.projector.v1")
                    .header("x-forwarded-for", "127.0.0.1");
            }
            let response = request.send().await.expect("G-3 live HTTP response");
            let status = response.status();
            let body: serde_json::Value = response.json().await.expect("G-3 error envelope");
            assert_eq!(
                status.as_u16(),
                401,
                "G-3 {method} {path} origin={origin}: {body}"
            );
            assert_eq!(body["schema_version"], 1);
            assert_eq!(body["code"], "RUNTIME_UNAUTHENTICATED");
            assert_eq!(
                runtime_row_counts(&db, project_id).await,
                before,
                "G-3 rejected {method} {path} origin={origin} mutated writer"
            );
        }
    }
}
