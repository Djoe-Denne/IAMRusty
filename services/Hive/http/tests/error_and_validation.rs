use axum::http::StatusCode;
use axum::response::IntoResponse;
use hive_application::ApplicationError;
use hive_http::{validate_pagination, validate_query_params, HttpError, ValidatedJson};
use rustycog::core::error::DomainError;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
struct QuerySample {
    #[validate(length(min = 1))]
    query: String,
}

#[derive(Debug, Deserialize, Validate)]
struct BodySample {
    #[validate(length(min = 1))]
    name: String,
}

#[test]
fn pagination_and_query_validation() {
    assert_eq!(validate_pagination(None, None, 100).unwrap(), (1, 20));
    assert!(validate_pagination(Some(0), Some(10), 100).is_err());
    assert!(validate_pagination(Some(1), Some(0), 100).is_err());
    assert!(validate_pagination(Some(1), Some(200), 50).is_err());
    assert_eq!(
        validate_pagination(Some(2), Some(10), 100).unwrap(),
        (2, 10)
    );

    let ok = QuerySample {
        query: "org".to_string(),
    };
    assert!(validate_query_params(&ok).is_ok());
    let bad = QuerySample {
        query: String::new(),
    };
    assert!(validate_query_params(&bad).is_err());
}

#[test]
fn http_error_into_response_covers_all_variants() {
    let cases: Vec<(HttpError, StatusCode)> = vec![
        (
            HttpError::Application(ApplicationError::Domain(DomainError::entity_not_found(
                "Org", "1",
            ))),
            StatusCode::NOT_FOUND,
        ),
        (
            HttpError::Application(ApplicationError::Domain(DomainError::invalid_input("bad"))),
            StatusCode::BAD_REQUEST,
        ),
        (
            HttpError::Application(ApplicationError::Domain(
                DomainError::business_rule_violation("rule"),
            )),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            HttpError::Application(ApplicationError::Domain(DomainError::unauthorized("op"))),
            StatusCode::UNAUTHORIZED,
        ),
        (
            HttpError::Application(ApplicationError::Domain(
                DomainError::resource_already_exists("Org", "slug"),
            )),
            StatusCode::CONFLICT,
        ),
        (
            HttpError::Application(ApplicationError::Domain(DomainError::permission_denied(
                "no",
            ))),
            StatusCode::FORBIDDEN,
        ),
        (
            HttpError::Application(ApplicationError::Domain(
                DomainError::external_service_error("github", "down"),
            )),
            StatusCode::BAD_GATEWAY,
        ),
        (
            HttpError::Application(ApplicationError::Domain(DomainError::internal_error(
                "boom",
            ))),
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
        (
            HttpError::Application(ApplicationError::validation_error(vec![])),
            StatusCode::BAD_REQUEST,
        ),
        (
            HttpError::Application(ApplicationError::external_service_error("github", "x")),
            StatusCode::BAD_GATEWAY,
        ),
        (
            HttpError::Application(ApplicationError::rate_limit("slow")),
            StatusCode::TOO_MANY_REQUESTS,
        ),
        (
            HttpError::Application(ApplicationError::internal_error("x")),
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
        (
            HttpError::BadRequest {
                message: "bad".into(),
            },
            StatusCode::BAD_REQUEST,
        ),
        (
            HttpError::Validation {
                message: "invalid".into(),
            },
            StatusCode::BAD_REQUEST,
        ),
        (HttpError::Unauthorized, StatusCode::UNAUTHORIZED),
        (HttpError::Forbidden, StatusCode::FORBIDDEN),
        (HttpError::NotFound, StatusCode::NOT_FOUND),
        (
            HttpError::Conflict {
                message: "dup".into(),
            },
            StatusCode::CONFLICT,
        ),
        (HttpError::PayloadTooLarge, StatusCode::PAYLOAD_TOO_LARGE),
        (HttpError::RateLimit, StatusCode::TOO_MANY_REQUESTS),
        (
            HttpError::Internal {
                message: "oops".into(),
            },
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
    ];

    for (error, expected) in cases {
        assert_eq!(error.into_response().status(), expected);
    }
}

#[tokio::test]
async fn validated_json_accepts_valid_payloads_and_rejects_invalid_ones() {
    use axum::body::Body;
    use axum::extract::FromRequest;
    use axum::http::{header, Request};

    async fn from_json(body: &'static str) -> Result<ValidatedJson<BodySample>, HttpError> {
        let request = Request::builder()
            .method("POST")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();
        ValidatedJson::<BodySample>::from_request(request, &()).await
    }

    let ok = from_json(r#"{"name":"hive"}"#).await.unwrap();
    assert_eq!(ok.0.name, "hive");
    assert!(from_json("{not-json").await.is_err());
    assert!(from_json(r#"{"name":""}"#).await.is_err());
}
