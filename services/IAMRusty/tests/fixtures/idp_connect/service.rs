use super::resources::{github_arthur, github_bob, gitlab_alice, success_tokens, TEST_HMAC_SECRET};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use idp_connect_contract::dto::{
    valid_pkce_verifier, AuthorizeAuthenticationRequest, AuthorizeRequest, AuthorizeResponse,
    CompleteAuthorizationRequest, ProviderAuthentication, ProviderTokens, ProviderUserProfile,
    TokenRequest, AUTHORIZE_AUTHENTICATION_PATH, AUTHORIZE_PATH, COMPLETE_AUTHORIZATION_PATH,
    PROFILE_PATH, TOKEN_PATH,
};
use idp_connect_contract::hmac::{unix_timestamp_secs, verify, SIGNATURE_HEADER, TIMESTAMP_HEADER};
use rustycog::testing::wiremock::MockServerFixture;
use std::sync::Arc;
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

/// `WireMock` collaborator for IAM → `IdP` Connect (`/github-connect`, `/gitlab-connect`).
pub struct IdpConnectMockService {
    server: Arc<MockServer>,
    fixture: MockServerFixture,
}

impl IdpConnectMockService {
    pub async fn new() -> Self {
        let fixture = MockServerFixture::new().await;
        let server = fixture.server();
        Self { server, fixture }
    }

    #[must_use]
    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    pub async fn reset(&self) {
        self.fixture.reset().await;
    }

    /// Captured real outbound requests; callers must not print credential bodies.
    pub async fn received_requests(&self) -> Vec<Request> {
        self.server.received_requests().await.unwrap_or_default()
    }

    pub async fn mock_github_happy_arthur(&self) -> &Self {
        self.mock_authorize_authentication("github")
            .await
            .mock_complete_success("github", success_tokens(), github_arthur())
            .await
    }

    pub async fn mock_github_happy_bob(&self) -> &Self {
        self.mock_authorize_authentication("github")
            .await
            .mock_complete_success("github", success_tokens(), github_bob())
            .await
    }

    pub async fn mock_gitlab_happy_alice(&self) -> &Self {
        self.mock_authorize_authentication("gitlab")
            .await
            .mock_complete_success("gitlab", success_tokens(), gitlab_alice())
            .await
    }

    /// G2/G8: separate receipts, never count token+profile as one completion.
    pub async fn endpoint_counts(&self) -> [usize; 5] {
        let requests = self.received_requests().await;
        [
            AUTHORIZE_AUTHENTICATION_PATH,
            COMPLETE_AUTHORIZATION_PATH,
            AUTHORIZE_PATH,
            TOKEN_PATH,
            PROFILE_PATH,
        ]
        .map(|suffix| {
            requests
                .iter()
                .filter(|request| request.url.path().ends_with(suffix))
                .count()
        })
    }

    pub async fn mock_authorize_authentication(&self, slug: &str) -> &Self {
        self.mock_authorize_observed(slug, |_| {}).await
    }

    /// Observe durable BEGIN from a test-local closure before returning authorize.
    pub async fn mock_authorize_observed<F>(&self, slug: &str, observe: F) -> &Self
    where
        F: Fn(&AuthorizeAuthenticationRequest) + Send + Sync + 'static,
    {
        let slug = slug.to_owned();
        let full_path = format!("/{slug}-connect{AUTHORIZE_AUTHENTICATION_PATH}");
        Mock::given(method("POST"))
            .and(path(full_path))
            .respond_with(move |request: &Request| {
                if !hmac_ok(request) {
                    return ResponseTemplate::new(401);
                }
                let Ok(body) =
                    serde_json::from_slice::<AuthorizeAuthenticationRequest>(&request.body)
                else {
                    return ResponseTemplate::new(400);
                };
                if !canonical_nonce(&body.nonce) {
                    return ResponseTemplate::new(400);
                }
                observe(&body);
                authorize_response(
                    &slug,
                    AuthorizeRequest {
                        redirect_uri: body.redirect_uri,
                        state: body.state,
                        code_challenge: body.code_challenge,
                        code_challenge_method: body.code_challenge_method,
                    },
                )
            })
            .mount(&self.server)
            .await;
        self
    }

    pub async fn mock_complete_success(
        &self,
        slug: &str,
        tokens: ProviderTokens,
        profile: ProviderUserProfile,
    ) -> &Self {
        let authentication = ProviderAuthentication { tokens, profile };
        self.mock_complete_with(slug, move |_| {
            ResponseTemplate::new(200).set_body_json(&authentication)
        })
        .await
    }

    /// Refusal is atomic: there is no preliminary token response or profile fallback.
    pub async fn mock_complete_failure(&self, slug: &str, status: u16) -> &Self {
        self.mock_complete_with(slug, move |_| ResponseTemplate::new(status))
            .await
    }

    /// G2/G5: inspect committed storage before releasing the network result.
    /// Call reset before overriding an already mounted response (first match wins).
    pub async fn mock_complete_with<F>(&self, slug: &str, respond: F) -> &Self
    where
        F: Fn(&CompleteAuthorizationRequest) -> ResponseTemplate + Send + Sync + 'static,
    {
        let full_path = format!("/{slug}-connect{COMPLETE_AUTHORIZATION_PATH}");
        Mock::given(method("POST"))
            .and(path(full_path))
            .respond_with(move |request: &Request| {
                if !hmac_ok(request) {
                    return ResponseTemplate::new(401);
                }
                let Ok(body) =
                    serde_json::from_slice::<CompleteAuthorizationRequest>(&request.body)
                else {
                    return ResponseTemplate::new(400);
                };
                if !canonical_nonce(&body.expected_nonce)
                    || body
                        .code_verifier
                        .as_deref()
                        .is_some_and(|value| !valid_pkce_verifier(value))
                {
                    return ResponseTemplate::new(400);
                }
                respond(&body)
            })
            .mount(&self.server)
            .await;
        self
    }

    pub async fn mock_authorize(&self, slug: &str) -> &Self {
        self.mount_hmac(
            slug,
            AUTHORIZE_PATH,
            HmacResponder::Authorize(slug.to_string()),
        )
        .await
    }

    pub async fn mock_token(&self, slug: &str) -> &Self {
        self.mount_hmac(slug, TOKEN_PATH, HmacResponder::Token)
            .await
    }

    pub async fn mock_profile(&self, slug: &str, profile: ProviderUserProfile) -> &Self {
        self.mount_hmac(slug, PROFILE_PATH, HmacResponder::Profile(profile))
            .await
    }

    /// Always 401 on the given connector path (IAM error path).
    pub async fn mock_s2s_unauthorized(&self, slug: &str, path_suffix: &str) -> &Self {
        let full_path = format!("/{slug}-connect{path_suffix}");
        Mock::given(method("POST"))
            .and(path(full_path))
            .respond_with(ResponseTemplate::new(401))
            .mount(&self.server)
            .await;
        self
    }

    pub async fn mock_profile_status(&self, slug: &str, status: u16) -> &Self {
        let full_path = format!("/{slug}-connect{PROFILE_PATH}");
        Mock::given(method("POST"))
            .and(path(full_path))
            .respond_with(ResponseTemplate::new(status))
            .mount(&self.server)
            .await;
        self
    }

    async fn mount_hmac(&self, slug: &str, suffix: &str, responder: HmacResponder) -> &Self {
        let full_path = format!("/{slug}-connect{suffix}");
        Mock::given(method("POST"))
            .and(path(full_path))
            .respond_with(responder)
            .mount(&self.server)
            .await;
        self
    }
}

#[derive(Clone)]
enum HmacResponder {
    Authorize(String),
    Token,
    Profile(ProviderUserProfile),
}

impl Respond for HmacResponder {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        if !hmac_ok(request) {
            return ResponseTemplate::new(401);
        }
        match self {
            Self::Authorize(slug) => authorize_ok(slug, request),
            Self::Token => {
                let Ok(body) = serde_json::from_slice::<TokenRequest>(&request.body) else {
                    return ResponseTemplate::new(400);
                };
                if body
                    .code_verifier
                    .as_deref()
                    .is_some_and(|value| !valid_pkce_verifier(value))
                {
                    return ResponseTemplate::new(400);
                }
                ResponseTemplate::new(200).set_body_json(success_tokens())
            }
            Self::Profile(profile) => ResponseTemplate::new(200).set_body_json(profile),
        }
    }
}

fn hmac_ok(request: &Request) -> bool {
    let Some(timestamp) = header_str(request, TIMESTAMP_HEADER).and_then(|v| v.parse::<i64>().ok())
    else {
        return false;
    };
    let Some(signature) = header_str(request, SIGNATURE_HEADER) else {
        return false;
    };
    let Ok(body) = std::str::from_utf8(&request.body) else {
        return false;
    };
    let Ok(now) = unix_timestamp_secs() else {
        return false;
    };
    verify(
        TEST_HMAC_SECRET.as_bytes(),
        "POST",
        request.url.path(),
        timestamp,
        body,
        signature,
        now,
    )
    .is_ok()
}

fn header_str<'a>(request: &'a Request, name: &str) -> Option<&'a str> {
    request
        .headers
        .get(name)
        .and_then(|value| value.to_str().ok())
}

fn authorize_ok(slug: &str, request: &Request) -> ResponseTemplate {
    let Ok(body) = serde_json::from_slice::<AuthorizeRequest>(&request.body) else {
        return ResponseTemplate::new(401);
    };
    authorize_response(slug, body)
}

fn canonical_nonce(nonce: &str) -> bool {
    URL_SAFE_NO_PAD
        .decode(nonce)
        .is_ok_and(|bytes| bytes.len() == 32 && URL_SAFE_NO_PAD.encode(bytes) == nonce)
}

fn authorize_response(slug: &str, body: AuthorizeRequest) -> ResponseTemplate {
    let Ok(challenge) = body.pkce_challenge() else {
        return ResponseTemplate::new(400);
    };
    let (base, client_id, scope) = if slug == "gitlab" {
        (
            "http://localhost:3000/oauth/authorize",
            "test_gitlab_client_id",
            "read_user",
        )
    } else {
        (
            "http://localhost:3000/login/oauth/authorize",
            "test_github_client_id",
            "user",
        )
    };
    let Ok(mut url) = Url::parse(base) else {
        return ResponseTemplate::new(500);
    };
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", &body.redirect_uri)
        .append_pair("scope", scope)
        .append_pair("response_type", "code")
        .append_pair("state", &body.state);
    if let Some(challenge) = challenge {
        url.query_pairs_mut()
            .append_pair("code_challenge", challenge)
            .append_pair("code_challenge_method", "S256");
    }
    ResponseTemplate::new(200).set_body_json(AuthorizeResponse {
        authorization_url: url.to_string(),
        scope: scope.to_string(),
    })
}
