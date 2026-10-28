pub mod bindings;
pub mod components;
pub mod members;
pub mod projects;
pub mod runtime_control;

pub use bindings::*;
pub use components::*;
pub use members::*;
pub use projects::*;

/// Only explicitly public-read handlers use this view. A foreign principal is
/// retained for audit but never enters UUID-only membership/admin lookups.
/// The application still requires Public+Active when the local UUID is None;
/// the outer permission guard has already checked the exact authenticated
/// subject, and its denials never reach this function.
fn public_read_identity(
    caller: &rustycog::http::OptionalAuthPrincipal,
) -> (Option<uuid::Uuid>, rustycog::command::CommandContext) {
    let mut context = rustycog::command::CommandContext::new();
    let Some(actor) = &caller.principal else {
        return (None, context);
    };
    context = context
        .with_metadata("principal_issuer".into(), actor.principal.issuer.clone())
        .with_metadata(
            "principal_subject".into(),
            actor.principal.subject.to_string(),
        );
    match actor.platform_user_id() {
        Ok(user_id) => (Some(user_id), context.with_user_id(user_id)),
        Err(_) => (
            None,
            context.with_metadata("public_read_only".into(), "true".into()),
        ),
    }
}

#[cfg(test)]
mod principal_tests {
    use super::*;
    use axum::{extract::FromRequestParts, http::Request};
    use rustycog::http::{JwtPrincipal, OptionalAuthPrincipal};
    use rustycog::permission::PrincipalPolicy;
    use std::sync::Arc;
    use uuid::Uuid;

    #[tokio::test]
    async fn public_read_keeps_foreign_authority_out_of_uuid_only_branches() {
        let policy = Arc::new(PrincipalPolicy::new("https://platform.example/iam", false).unwrap());
        let subject = Uuid::new_v4();
        for issuer in [
            None,
            Some("https://platform.example/iam"),
            Some("https://org.example/iam"),
        ] {
            let (mut parts, _) = Request::new(()).into_parts();
            if let Some(issuer) = issuer {
                parts.extensions.insert(JwtPrincipal {
                    iss: issuer.into(),
                    sub: subject,
                    org: None,
                });
                parts.extensions.insert(policy.clone());
            }
            let caller = OptionalAuthPrincipal::from_request_parts(&mut parts, &())
                .await
                .unwrap();
            let (local_user, context) = public_read_identity(&caller);
            match issuer {
                Some("https://platform.example/iam") => {
                    assert_eq!(local_user, Some(subject));
                    assert_eq!(context.user_id, Some(subject));
                }
                Some(issuer) => {
                    assert_eq!(
                        local_user, None,
                        "never call UUID membership/admin branches for a foreign authority"
                    );
                    assert_eq!(context.user_id, None);
                    assert_eq!(context.metadata["principal_issuer"], issuer);
                    assert_eq!(context.metadata["public_read_only"], "true");
                }
                None => {
                    assert_eq!(local_user, None);
                    assert_eq!(context.user_id, None);
                }
            }
        }
    }
}
