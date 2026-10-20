//! Eight internal routes, one authenticated command surface, zero retries.

use apparatus_contracts::runtime::{
    parse_runtime_json, RuntimeBeginEffectRequestV1, RuntimeCompleteEffectRequestV1,
    RuntimeControlCallerV1, RuntimeControlErrorV1, RuntimeCursorV1, RuntimeEffectOutcomeV1,
    RuntimeErrorCodeV1, RuntimeInstanceCursorV1, RuntimeInstanceRegistrationV1,
    RuntimeInstanceRevokeRequestV1, RuntimeInstanceSelectorV1, RuntimeObservationV1,
    RuntimeServiceRoleV1, RuntimeWriterLeaseRequestV1,
};
use axum::{
    body::Bytes,
    extract::{
        rejection::{BytesRejection, PathRejection},
        Path, Query, State,
    },
    http::{StatusCode, Uri},
    response::{IntoResponse, Response},
    Extension, Json,
};
use manifesto_application::{RuntimeControlAction, RuntimeControlCommand, RuntimeControlReply};
use rustycog::{command::CommandContext, http::AppState};
use serde::{de::DeserializeOwned, Deserialize};
use uuid::Uuid;

/// Structured runtime error; never exposes framework/SQL diagnostic strings.
pub struct RuntimeControlHttpError(pub RuntimeControlErrorV1);
impl From<RuntimeControlErrorV1> for RuntimeControlHttpError {
    fn from(error: RuntimeControlErrorV1) -> Self {
        Self(error)
    }
}
impl IntoResponse for RuntimeControlHttpError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.0.code.http_status())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self.0)).into_response()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectQuery {
    project_id: Uuid,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListQuery {
    schema_version: u16,
    project_id: Uuid,
    cursor: Option<String>,
    page_size: Option<u16>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BindingQuery {
    schema_version: u16,
    project_id: Uuid,
    instances_mode: String,
    instances_cursor: Option<String>,
    instances_page_size: Option<u16>,
    pod_uid: Option<String>,
    instance_id: Option<Uuid>,
}

fn query<T: DeserializeOwned>(uri: &Uri) -> Result<T, RuntimeControlHttpError> {
    Query::<T>::try_from_uri(uri)
        .map(|Query(value)| value)
        .map_err(|_| fail(RuntimeErrorCodeV1::RuntimeInvalidRequest))
}
fn version(value: u16) -> Result<(), RuntimeControlHttpError> {
    if value != 1 {
        return Err(fail(RuntimeErrorCodeV1::RuntimeUnsupportedVersion));
    }
    Ok(())
}
fn identifier(value: &str) -> Result<Uuid, RuntimeControlHttpError> {
    Uuid::parse_str(value).map_err(|_| fail(RuntimeErrorCodeV1::RuntimeInvalidRequest))
}
fn body<T: DeserializeOwned>(
    body: Result<Bytes, BytesRejection>,
) -> Result<T, RuntimeControlHttpError> {
    let bytes = body.map_err(|error| {
        fail(if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            RuntimeErrorCodeV1::RuntimePayloadTooLarge
        } else {
            RuntimeErrorCodeV1::RuntimeInvalidRequest
        })
    })?;
    parse_runtime_json(&bytes).map_err(fail)
}
fn fail(code: RuntimeErrorCodeV1) -> RuntimeControlHttpError {
    code.error(
        RuntimeServiceRoleV1::Projector,
        RuntimeEffectOutcomeV1::NotStarted,
    )
    .into()
}

async fn execute(
    state: AppState,
    caller: RuntimeControlCallerV1,
    action: RuntimeControlAction,
) -> Result<Json<RuntimeControlReply>, RuntimeControlHttpError> {
    let ambiguous_effect = matches!(
        &action,
        RuntimeControlAction::Begin { .. } | RuntimeControlAction::Complete { .. }
    );
    let command = RuntimeControlCommand::new(caller.clone(), action);
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        state
            .command_service
            .execute_once(command, CommandContext::new()),
    )
    .await;
    match result {
        Ok(Ok(Ok(reply))) => Ok(Json(reply)),
        Ok(Ok(Err(error))) => Err(error.into()),
        _ if ambiguous_effect => Err(RuntimeErrorCodeV1::RuntimeEffectOutcomeUnknown
            .error(caller.role, RuntimeEffectOutcomeV1::MayHaveStarted)
            .into()),
        _ => Err(RuntimeErrorCodeV1::RuntimeAuthorityUnavailable
            .error(caller.role, RuntimeEffectOutcomeV1::NotStarted)
            .into()),
    }
}

/// List current writer metadata, instances explicitly not read.
///
/// # Errors
/// Rejects malformed query, role/proof, writer failure or stale cursor.
pub async fn list_runtime_bindings(
    State(state): State<AppState>,
    Extension(caller): Extension<RuntimeControlCallerV1>,
    uri: Uri,
) -> Result<Json<RuntimeControlReply>, RuntimeControlHttpError> {
    let query: ListQuery = query(&uri)?;
    version(query.schema_version)?;
    execute(
        state,
        caller,
        RuntimeControlAction::List {
            project_id: query.project_id,
            cursor: query.cursor.map(RuntimeCursorV1),
            page_size: query.page_size,
        },
    )
    .await
}

/// Get one writer snapshot with an explicit, bounded instance selection.
///
/// # Errors
/// Rejects mixed selectors, missing project/version, invalid IDs or writer errors.
pub async fn get_runtime_binding(
    State(state): State<AppState>,
    Extension(caller): Extension<RuntimeControlCallerV1>,
    uri: Uri,
    path: Result<Path<String>, PathRejection>,
) -> Result<Json<RuntimeControlReply>, RuntimeControlHttpError> {
    let query: BindingQuery = query(&uri)?;
    version(query.schema_version)?;
    let Path(binding) = path.map_err(|_| fail(RuntimeErrorCodeV1::RuntimeInvalidRequest))?;
    let instances = match query.instances_mode.as_str() {
        "page" if query.pod_uid.is_none() && query.instance_id.is_none() => {
            RuntimeInstanceSelectorV1::Page {
                cursor: query.instances_cursor.map(RuntimeInstanceCursorV1),
                page_size: query.instances_page_size,
            }
        }
        "by_pod_uid"
            if query.instances_cursor.is_none()
                && query.instances_page_size.is_none()
                && query.instance_id.is_none() =>
        {
            let pod_uid = query
                .pod_uid
                .ok_or_else(|| fail(RuntimeErrorCodeV1::RuntimeInvalidRequest))?;
            apparatus_contracts::runtime::validate_runtime_uid(&pod_uid).map_err(fail)?;
            RuntimeInstanceSelectorV1::ByPodUid { pod_uid }
        }
        "by_instance_id"
            if query.instances_cursor.is_none()
                && query.instances_page_size.is_none()
                && query.pod_uid.is_none() =>
        {
            RuntimeInstanceSelectorV1::ByInstanceId {
                instance_id: query
                    .instance_id
                    .ok_or_else(|| fail(RuntimeErrorCodeV1::RuntimeInvalidRequest))?,
            }
        }
        _ => return Err(fail(RuntimeErrorCodeV1::RuntimeInvalidRequest)),
    };
    execute(
        state,
        caller,
        RuntimeControlAction::Get {
            project_id: query.project_id,
            binding_id: identifier(&binding)?,
            instances,
        },
    )
    .await
}

macro_rules! single_handler {
    ($name:ident, $request:ty, $variant:ident) => {
        /// Execute one authenticated control command, without business retries.
        ///
        /// # Errors
        /// Rejects malformed inputs or the writer's finite versioned error.
        pub async fn $name(
            State(state): State<AppState>,
            Extension(caller): Extension<RuntimeControlCallerV1>,
            uri: Uri,
            path: Result<Path<String>, PathRejection>,
            bytes: Result<Bytes, BytesRejection>,
        ) -> Result<Json<RuntimeControlReply>, RuntimeControlHttpError> {
            let project: ProjectQuery = query(&uri)?;
            let Path(binding) =
                path.map_err(|_| fail(RuntimeErrorCodeV1::RuntimeInvalidRequest))?;
            let request: $request = body(bytes)?;
            execute(
                state,
                caller,
                RuntimeControlAction::$variant {
                    project_id: project.project_id,
                    binding_id: identifier(&binding)?,
                    request,
                },
            )
            .await
        }
    };
}
single_handler!(runtime_writer_lease, RuntimeWriterLeaseRequestV1, Lease);
single_handler!(
    runtime_report_observation,
    RuntimeObservationV1,
    Observation
);
single_handler!(runtime_begin_effect, RuntimeBeginEffectRequestV1, Begin);

macro_rules! pair_handler {
    ($name:ident, $request:ty, $variant:ident, $id:ident) => {
        /// Execute one authenticated control command with exact path IDs.
        ///
        /// # Errors
        /// Rejects malformed inputs or the writer's finite versioned error.
        pub async fn $name(
            State(state): State<AppState>,
            Extension(caller): Extension<RuntimeControlCallerV1>,
            uri: Uri,
            path: Result<Path<(String, String)>, PathRejection>,
            bytes: Result<Bytes, BytesRejection>,
        ) -> Result<Json<RuntimeControlReply>, RuntimeControlHttpError> {
            let project: ProjectQuery = query(&uri)?;
            let Path((binding, second)) =
                path.map_err(|_| fail(RuntimeErrorCodeV1::RuntimeInvalidRequest))?;
            let request: $request = body(bytes)?;
            execute(
                state,
                caller,
                RuntimeControlAction::$variant {
                    project_id: project.project_id,
                    binding_id: identifier(&binding)?,
                    $id: identifier(&second)?,
                    request,
                },
            )
            .await
        }
    };
}
pair_handler!(
    runtime_register_instance,
    RuntimeInstanceRegistrationV1,
    Register,
    instance_id
);
pair_handler!(
    runtime_revoke_instance,
    RuntimeInstanceRevokeRequestV1,
    Revoke,
    instance_id
);
pair_handler!(
    runtime_complete_effect,
    RuntimeCompleteEffectRequestV1,
    Complete,
    effect_id
);
