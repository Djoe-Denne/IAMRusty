use chrono::Utc;
use iam_domain::entity::provider::Provider;
use iam_domain::entity::provider_link::ProviderLink;
use sea_orm::DbErr;

use super::entity::provider_tokens;

/// Map a `provider_tokens` row to a domain [`ProviderLink`].
///
/// # Errors
///
/// Returns [`DbErr::Custom`] when `model.provider` is not a valid provider slug.
pub(crate) fn to_provider_link(model: provider_tokens::Model) -> Result<ProviderLink, DbErr> {
    let provider = Provider::parse_slug(&model.provider).map_err(|_| {
        DbErr::Custom(format!(
            "invalid provider slug in provider_tokens: {}",
            model.provider
        ))
    })?;
    Ok(ProviderLink {
        user_id: model.user_id,
        provider,
        provider_user_id: model.provider_user_id,
        linked_at: chrono::DateTime::<Utc>::from_naive_utc_and_offset(model.created_at, Utc),
    })
}
