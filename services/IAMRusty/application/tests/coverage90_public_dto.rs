//! QA coverage90 A: public DTO projections must not expose credentials/internal token fields.
use iam_application::dto::{CheckUsernameResponse, ProviderTokenResponseDto, UserProfileDto};
use iam_application::usecase::provider::ProviderTokenResponse;
use iam_domain::entity::{provider::ProviderTokens, user::User};
use serde_json::json;

#[test]
fn public_user_profile_excludes_password_hash_for_complete_and_incomplete_users() {
    let users = [
        User::new_with_password(
            "reader".into(),
            "sentinel-private-hash".into(),
            Some("https://aiforall.test/avatar".into()),
        ),
        User::new_incomplete_with_password("sentinel-private-hash".into(), None),
    ];
    for user in users {
        let expected_id = user.id.to_string();
        let expected_username = user.username.clone();
        let expected_avatar = user.avatar_url.clone();
        let dto = UserProfileDto::from_user_and_email(user.clone(), "reader@aiforall.test".into());
        let body = serde_json::to_value(dto).expect("public profile serialization");
        assert_eq!(
            body,
            json!({"id":expected_id, "username":expected_username, "email":"reader@aiforall.test", "avatar":expected_avatar})
        );
        assert!(!body.to_string().contains("sentinel-private-hash"));
        let legacy: UserProfileDto = user.into();
        let legacy_body = serde_json::to_value(legacy).expect("legacy projection");
        assert_eq!(
            legacy_body,
            json!({"id":expected_id, "username":expected_username, "email":"", "avatar":expected_avatar})
        );
    }
}

#[test]
fn provider_token_projections_never_expose_provider_refresh_credentials() {
    for expires_in in [None, Some(0), Some(3600)] {
        let tokens = ProviderTokens {
            access_token: "public-access-ticket".into(),
            refresh_token: Some("sentinel-private-refresh".into()),
            expires_in,
        };
        let dto: ProviderTokenResponseDto = tokens.clone().into();
        let response: ProviderTokenResponse = tokens.into();
        let expected = json!({"access_token":"public-access-ticket", "expires_in":expires_in});
        assert_eq!(
            serde_json::to_value(dto).expect("DTO serialization"),
            expected
        );
        assert_eq!(
            serde_json::to_value(response).expect("response serialization"),
            expected
        );
    }
}

#[test]
fn username_availability_omits_absent_suggestions_but_preserves_explicit_empty_list() {
    for (suggestions, expected) in [
        (None, json!({"available":true})),
        (Some(vec![]), json!({"available":true,"suggestions":[]})),
        (
            Some(vec!["reader_2".into()]),
            json!({"available":true,"suggestions":["reader_2"]}),
        ),
    ] {
        let response = CheckUsernameResponse {
            available: true,
            suggestions,
        };
        assert_eq!(
            serde_json::to_value(response).expect("availability serialization"),
            expected
        );
    }
}
