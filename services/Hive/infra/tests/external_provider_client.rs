use hive_configuration::ExternalProviderServiceConfig;
use hive_domain::port::service::ExternalProviderClient;
use hive_infra::HttpExternalProviderClient;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client(base_url: &str, api_key: Option<String>) -> HttpExternalProviderClient {
    HttpExternalProviderClient::new(base_url, api_key, 5, 1).unwrap()
}

#[tokio::test]
async fn from_config_and_success_paths() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/config/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/connection/test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"connected": true})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/organization/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "external_id": "42",
            "name": "acme",
            "display_name": "Acme",
            "description": null,
            "avatar_url": null,
            "member_count": 3,
            "is_public": false,
            "provider_source": "github"
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/members"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "members": [{
                "external_id": "1",
                "username": "ada",
                "email": "ada@example.test",
                "display_name": "Ada",
                "avatar_url": null,
                "roles": [],
                "is_active": true,
                "provider_source": "github"
            }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/members/check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"is_member": true})))
        .mount(&server)
        .await;

    let uri = server.uri();
    let config = ExternalProviderServiceConfig {
        base_url: format!("{uri}/"),
        api_key: Some("secret".into()),
        timeout_seconds: 5,
        max_retries: 1,
    };
    let client = HttpExternalProviderClient::from_config(&config).unwrap();
    let body = json!({"org": "acme"});
    client.validate_config("github", &body).await.unwrap();
    assert!(client.test_connection("github", &body).await.unwrap());
    let info = client.get_organization_info("github", &body).await.unwrap();
    assert_eq!(info.external_id, "42");
    let members = client.get_members("github", &body).await.unwrap();
    assert_eq!(members[0].username, "ada");
    let synced = client.sync_members("github", &body).await.unwrap();
    assert_eq!(synced.len(), 1);
    assert!(client.is_member("github", &body, "ada").await.unwrap());
}

#[tokio::test]
async fn http_error_and_invalid_payloads() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/config/validate"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({"error": "bad"})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/connection/test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/members"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not-json"))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/members/check"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"members": []})))
        .mount(&server)
        .await;

    let http_client = client(&server.uri(), None);
    let body = json!({"org": "acme"});
    assert!(http_client.validate_config("github", &body).await.is_err());
    assert!(http_client.test_connection("github", &body).await.is_err());
    assert!(http_client.get_members("github", &body).await.is_err());
    assert!(http_client.is_member("github", &body, "ada").await.is_err());
    assert!(client("http://127.0.0.1:1", None)
        .validate_config("github", &body)
        .await
        .is_err());
}
