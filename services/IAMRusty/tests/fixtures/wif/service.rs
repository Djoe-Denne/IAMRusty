//! Wiremock WIF token-exchange mocks (ADR-0307).

use rustycog::testing::wiremock::MockServerFixture;
use std::sync::Arc;
use wiremock::{
    matchers::{body_string_contains, method, path},
    Mock, MockServer, ResponseTemplate,
};

use super::resources::*;

pub struct WifMockService {
    server: Arc<MockServer>,
    _fixture: MockServerFixture,
}

impl WifMockService {
    pub async fn new() -> Self {
        let fixture = MockServerFixture::new().await;
        let server = fixture.server();
        Self {
            server,
            _fixture: fixture,
        }
    }

    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    /// Shared wiremock server (for mounting sibling stubs, e.g. OpenBao Transit).
    pub fn server(&self) -> Arc<MockServer> {
        Arc::clone(&self.server)
    }

    /// Reset mounted stubs (skill contract; used by mid-test re-arrangement).
    #[allow(dead_code)]
    pub async fn reset(&self) {
        self._fixture.reset().await;
    }

    /// Stub AWS STS `AssumeRoleWithWebIdentity` returning `SessionToken`.
    pub async fn mock_aws_assume_role_ok(&self, session_token: &str) -> &Self {
        let body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<AssumeRoleWithWebIdentityResponse>
  <AssumeRoleWithWebIdentityResult>
    <Credentials>
      <AccessKeyId>ASIATEST</AccessKeyId>
      <SecretAccessKey>secret</SecretAccessKey>
      <SessionToken>{session_token}</SessionToken>
    </Credentials>
  </AssumeRoleWithWebIdentityResult>
</AssumeRoleWithWebIdentityResponse>"#
        );
        Mock::given(method("POST"))
            .and(path("/"))
            .and(body_string_contains("AssumeRoleWithWebIdentity"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/xml")
                    .set_body_string(body),
            )
            .mount(&*self.server)
            .await;
        self
    }

    /// Stub GCP STS `POST /v1/token` returning `access_token`.
    pub async fn mock_gcp_token_ok(&self, access_token: &str) -> &Self {
        let body = GcpAccessTokenBody {
            access_token: access_token.to_string(),
            token_type: "Bearer".into(),
            expires_in: 3600,
        };
        Mock::given(method("POST"))
            .and(path("/v1/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&*self.server)
            .await;
        self
    }

    /// Stub Azure AD `POST /{tenant}/oauth2/v2.0/token` returning `access_token`.
    pub async fn mock_azure_token_ok(&self, tenant_id: &str, access_token: &str) -> &Self {
        let body = AzureAccessTokenBody {
            access_token: access_token.to_string(),
            token_type: "Bearer".into(),
            expires_in: 3600,
        };
        Mock::given(method("POST"))
            .and(path(format!("/{tenant_id}/oauth2/v2.0/token")))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&*self.server)
            .await;
        self
    }
}
