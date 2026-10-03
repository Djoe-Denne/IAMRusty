use super::resources::{github_arthur, github_bob, gitlab_alice, success_tokens, TEST_HMAC_SECRET};
use idp_connect_contract::dto::{
    AuthorizeRequest, AuthorizeResponse, ProviderUserProfile, AUTHORIZE_PATH, PROFILE_PATH,
    TOKEN_PATH,
};
use idp_connect_contract::hmac::{unix_timestamp_secs, verify, SIGNATURE_HEADER, TIMESTAMP_HEADER};
use rustycog::testing::wiremock::MockServerFixture;
use std::sync::Arc;
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

/// WireMock collaborator for IAM → IdP Connect (`/github-connect`, `/gitlab-connect`).
pub struct IdpConnectMockService {
    server: Arc<MockServer>,
    _fixture: MockServerFixture,
}

impl IdpConnectMockService {
    pub async fn new() -> Self {
        let fixture = MockServerFixture::new().await;
        let server = fixture.server();
        Self {
            server,
            _fixture: fixture,
        }
    }

    #[must_use]
    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    pub async fn reset(&self) {
        self._fixture.reset().await;
    }

    pub async fn mock_github_happy_arthur(&self) -> &Self {
        self.mock_authorize("github")
            .await
            .mock_token("github")
            .await
            .mock_profile("github", github_arthur())
            .await
    }

    pub async fn mock_github_happy_bob(&self) -> &Self {
        self.mock_authorize("github")
            .await
            .mock_token("github")
            .await
            .mock_profile("github", github_bob())
            .await
    }

    pub async fn mock_gitlab_happy_alice(&self) -> &Self {
        self.mock_authorize("gitlab")
            .await
            .mock_token("gitlab")
            .await
            .mock_profile("gitlab", gitlab_alice())
            .await
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
            Self::Token => ResponseTemplate::new(200).set_body_json(success_tokens()),
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
    ResponseTemplate::new(200).set_body_json(AuthorizeResponse {
        authorization_url: url.to_string(),
        scope: scope.to_string(),
    })
}
