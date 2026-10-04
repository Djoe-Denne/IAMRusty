//! Test-local browser arrangement: every positive state comes from real BEGIN.
use crate::fixtures::idp_connect::service::IdpConnectMockService;
use reqwest::Client;
use uuid::Uuid;

async fn begin(
    client: &Client,
    base: &str,
    provider: &str,
    route: &str,
    user: Option<Uuid>,
    idp: &IdpConnectMockService,
) -> (String, String) {
    idp.mock_authorize(provider).await;
    let mut request = client.get(format!("{base}/api/auth/{provider}/{route}"));
    if let Some(user) = user {
        let config = iam_configuration::load_config_part::<iam_configuration::JwtConfig>("jwt")
            .expect("test JWT config");
        let token = crate::utils::jwt::create_valid_jwt_token_with_encoder(user, &config)
            .await
            .expect("writer-bound platform test token");
        request = request.bearer_auth(token);
    }
    let response = request.send().await.expect("browser START");
    assert_eq!(
        response.status().as_u16(),
        if route == "relink-start" { 200 } else { 303 }
    );
    let cookie = response
        .headers()
        .get("set-cookie")
        .expect("BEGIN browser cookie")
        .to_str()
        .expect("cookie header")
        .split(';')
        .next()
        .expect("cookie pair")
        .to_owned();
    let location = if route == "relink-start" {
        response
            .json::<serde_json::Value>()
            .await
            .expect("relink START JSON")["auth_url"]
            .as_str()
            .expect("authorization URL")
            .to_owned()
    } else {
        response
            .headers()
            .get("location")
            .expect("START redirect")
            .to_str()
            .expect("redirect header")
            .to_owned()
    };
    let url = url::Url::parse(&location).expect("authorize URL");
    let state = url
        .query_pairs()
        .find(|(key, _)| key == "state")
        .expect("persisted state")
        .1
        .into_owned();
    (state, cookie)
}

pub async fn login(
    client: &Client,
    base: &str,
    provider: &str,
    idp: &IdpConnectMockService,
) -> (String, String) {
    begin(client, base, provider, "login", None, idp).await
}
pub async fn link(
    client: &Client,
    base: &str,
    provider: &str,
    user: Uuid,
    idp: &IdpConnectMockService,
) -> (String, String) {
    begin(client, base, provider, "link", Some(user), idp).await
}
pub async fn relink(
    client: &Client,
    base: &str,
    provider: &str,
    user: Uuid,
    idp: &IdpConnectMockService,
) -> (String, String) {
    begin(client, base, provider, "relink-start", Some(user), idp).await
}
