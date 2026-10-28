use crate::error::HttpError;
use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use hive_application::{
    AddMemberCommand, AddMemberRequest, GetMemberCommand, ListMembersCommand, MemberListResponse,
    MemberResponse, PaginationRequest, RemoveMemberCommand, UpdateMemberCommand,
    UpdateMemberRolesRequest,
};
use rustycog::command::CommandContext;
use rustycog::http::{AppState, AuthPrincipal, ValidatedJson};
use rustycog::permission::{PrincipalRef, ResourceId};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Default, Deserialize)]
pub struct MemberPrincipalQuery {
    pub issuer: Option<String>,
}

fn target_issuer(
    actor: &AuthPrincipal,
    issuer: Option<&str>,
    user_id: Uuid,
) -> Result<String, HttpError> {
    let issuer = issuer.unwrap_or(&actor.principal.issuer);
    PrincipalRef::new(issuer, user_id).map_err(|_| HttpError::Forbidden)?;
    Ok(issuer.to_owned())
}

fn context(actor: &AuthPrincipal, operation: &str) -> CommandContext {
    CommandContext::new()
        .with_user_id(actor.principal.subject)
        .with_metadata("principal_issuer".into(), actor.principal.issuer.clone())
        .with_metadata("operation".into(), operation.into())
}

fn member_command_error(error: rustycog::command::CommandError) -> HttpError {
    let code = match &error {
        rustycog::command::CommandError::Business { code, .. } => code.as_str(),
        _ => "",
    };
    if code == "MEMBER_PRINCIPAL_AMBIGUOUS" {
        HttpError::Conflict {
            message: "MEMBER_PRINCIPAL_AMBIGUOUS".into(),
        }
    } else if code == "PRINCIPAL_INVALID" || code == "PRINCIPAL_INCOMPATIBLE" {
        HttpError::Forbidden
    } else {
        HttpError::Internal {
            message: "Membership operation failed".into(),
        }
    }
}

/// Add a target principal, independently of the authenticated actor.
pub async fn add_member(
    State(state): State<AppState>,
    Path(organization_id): Path<ResourceId>,
    actor: AuthPrincipal,
    ValidatedJson(request): ValidatedJson<AddMemberRequest>,
) -> Result<Json<MemberResponse>, HttpError> {
    target_issuer(&actor, request.issuer.as_deref(), request.user_id)?;
    let command = AddMemberCommand::new(
        organization_id.id(),
        &request,
        actor.principal.subject,
        actor.principal.issuer.clone(),
    );
    let result = state
        .command_service
        .execute(command, context(&actor, "add_member"))
        .await
        .map_err(member_command_error)?;
    Ok(Json(result))
}

/// Membership responses retain each issuer even when several share a UUID.
pub async fn list_members(
    State(state): State<AppState>,
    Path(organization_id): Path<ResourceId>,
    actor: AuthPrincipal,
    Query(pagination): Query<PaginationRequest>,
) -> Result<Json<MemberListResponse>, HttpError> {
    let command = ListMembersCommand::new(
        organization_id.id(),
        pagination,
        Some(actor.principal.subject),
        actor.principal.issuer.clone(),
    );
    let result = state
        .command_service
        .execute(command, context(&actor, "list_members"))
        .await
        .map_err(member_command_error)?;
    Ok(Json(result))
}

/// An omitted target issuer means only the actor's exact issuer.
pub async fn get_member(
    State(state): State<AppState>,
    Path((organization_id, user_id)): Path<(ResourceId, ResourceId)>,
    actor: AuthPrincipal,
    Query(target): Query<MemberPrincipalQuery>,
) -> Result<Json<MemberResponse>, HttpError> {
    let issuer = target_issuer(&actor, target.issuer.as_deref(), user_id.id())?;
    let command = GetMemberCommand::new(
        organization_id.id(),
        user_id.id(),
        Some(actor.principal.subject),
        actor.principal.issuer.clone(),
    )
    .with_target_issuer(issuer);
    let result = state
        .command_service
        .execute(command, context(&actor, "get_member"))
        .await
        .map_err(member_command_error)?;
    Ok(Json(result))
}

/// Delete only the selected principal, never every member sharing its UUID.
pub async fn remove_member(
    State(state): State<AppState>,
    Path((organization_id, user_id)): Path<(ResourceId, ResourceId)>,
    actor: AuthPrincipal,
    Query(target): Query<MemberPrincipalQuery>,
) -> Result<Json<()>, HttpError> {
    let issuer = target_issuer(&actor, target.issuer.as_deref(), user_id.id())?;
    let command = RemoveMemberCommand::new(
        organization_id.id(),
        user_id.id(),
        actor.principal.subject,
        actor.principal.issuer.clone(),
    )
    .with_target_issuer(issuer);
    state
        .command_service
        .execute(command, context(&actor, "remove_member"))
        .await
        .map_err(member_command_error)?;
    Ok(Json(()))
}

/// Update target roles without replacing its authority with the actor's authority.
pub async fn update_member(
    State(state): State<AppState>,
    Path((organization_id, user_id)): Path<(ResourceId, ResourceId)>,
    actor: AuthPrincipal,
    ValidatedJson(request): ValidatedJson<UpdateMemberRolesRequest>,
) -> Result<Json<MemberResponse>, HttpError> {
    target_issuer(&actor, request.issuer.as_deref(), user_id.id())?;
    let command = UpdateMemberCommand::new(
        organization_id.id(),
        user_id.id(),
        &request,
        actor.principal.subject,
        actor.principal.issuer.clone(),
    );
    let result = state
        .command_service
        .execute(command, context(&actor, "update_member"))
        .await
        .map_err(member_command_error)?;
    Ok(Json(result))
}

#[cfg(test)]
mod principal_error_tests {
    use super::*;
    use axum::response::IntoResponse;

    #[test]
    fn member_principal_ambiguity_is_conflict_without_identity_echo() {
        let error = rustycog::command::CommandError::business(
            "MEMBER_PRINCIPAL_AMBIGUOUS",
            "private-identity",
        );
        let mapped = member_command_error(error);
        match &mapped {
            HttpError::Conflict { message } => assert_eq!(message, "MEMBER_PRINCIPAL_AMBIGUOUS"),
            _ => panic!("ambiguous target must map to conflict"),
        }
        assert_eq!(
            mapped.into_response().status(),
            axum::http::StatusCode::CONFLICT
        );
        let untrusted_text = rustycog::command::CommandError::business(
            "member_failed",
            "MEMBER_PRINCIPAL_AMBIGUOUS private-identity",
        );
        assert_eq!(
            member_command_error(untrusted_text)
                .into_response()
                .status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
