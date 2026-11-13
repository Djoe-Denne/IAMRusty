//! QA coverage90-local B: project lifecycle and metadata validation boundaries.
use manifesto_domain::{
    DataClassification, DomainError, FieldUpdate, OwnerType, Project, ProjectStatus, Visibility,
};
use uuid::Uuid;

fn project() -> Project {
    Project::builder()
        .name("Hive migration".to_owned())
        .description(Some("Migrate the project catalog".to_owned()))
        .owner_type(OwnerType::Personal)
        .owner_id(Uuid::new_v4())
        .created_by(Uuid::new_v4())
        .build()
        .unwrap()
}

#[test]
fn publication_revision_and_publication_date_survive_suspend_resume_archive() {
    let mut project = project();
    assert!(project.can_publish());
    project.transition_status(ProjectStatus::Active).unwrap();
    assert_eq!(project.revision, 1);
    let published = project
        .published_at
        .expect("activation must record publication");
    assert!(!project.can_publish());
    project.transition_status(ProjectStatus::Suspended).unwrap();
    project.transition_status(ProjectStatus::Active).unwrap();
    assert_eq!(project.published_at, Some(published));
    project.transition_status(ProjectStatus::Archived).unwrap();
    assert_eq!(project.revision, 4);
    assert_eq!(project.status, ProjectStatus::Archived);
    assert_eq!(project.published_at, Some(published));
}

#[test]
fn suspended_project_can_be_archived_without_changing_publication_date() {
    // T-3: archiving directly from suspension is a supported terminal transition.
    let mut project = project();
    project.transition_status(ProjectStatus::Active).unwrap();
    let published = project
        .published_at
        .expect("activation must record publication");
    project.transition_status(ProjectStatus::Suspended).unwrap();
    let revision_before_archive = project.revision;

    project.transition_status(ProjectStatus::Archived).unwrap();

    assert_eq!(project.status, ProjectStatus::Archived);
    assert_eq!(project.revision, revision_before_archive + 1);
    assert_eq!(project.published_at, Some(published));
}

#[test]
fn rejected_project_transitions_do_not_change_revision_or_timestamps() {
    use ProjectStatus::{Active, Archived, Draft, Suspended};
    for (source, target) in [
        (Draft, Archived),
        (Draft, Suspended),
        (Active, Draft),
        (Archived, Active),
        (Archived, Draft),
        (Archived, Suspended),
        (Suspended, Draft),
    ] {
        let mut project = project();
        project.status = source;
        let before = project.clone();
        let result = project.transition_status(target);
        assert!(
            matches!(result, Err(DomainError::BusinessRuleViolation { .. })),
            "{source:?} -> {target:?}: {result:?}"
        );
        assert_eq!(project.status, source);
        assert_eq!(project.revision, before.revision);
        assert_eq!(project.updated_at, before.updated_at);
        assert_eq!(project.published_at, before.published_at);
    }
}

#[test]
fn metadata_patch_distinguishes_omitted_description_from_explicit_clear() {
    let mut project = project();
    let description = project.description.clone();
    project
        .update_metadata(
            Some("Catalog".to_owned()),
            FieldUpdate::Unchanged,
            Some(Visibility::Public),
            Some(true),
            Some(DataClassification::Public),
        )
        .unwrap();
    assert_eq!(project.name, "Catalog");
    assert_eq!(project.description, description);
    assert!(project.is_public());
    assert!(project.external_collaboration_enabled);
    assert_eq!(project.data_classification, DataClassification::Public);
    assert_eq!(project.revision, 1);
    project
        .update_metadata(None, FieldUpdate::Set(None), None, None, None)
        .unwrap();
    assert_eq!(project.description, None);
    assert_eq!(project.name, "Catalog");
    assert_eq!(project.revision, 2);
    project
        .update_metadata(
            None,
            FieldUpdate::Set(Some("Rebuilt".to_owned())),
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(project.description.as_deref(), Some("Rebuilt"));
    assert_eq!(project.revision, 3);
}

#[test]
fn invalid_metadata_field_is_rejected_without_changing_existing_metadata() {
    for name in [String::new(), " \t\n".to_owned(), "x".repeat(256)] {
        let mut project = project();
        let before = project.clone();
        let result = project.update_metadata(Some(name), FieldUpdate::Unchanged, None, None, None);
        assert!(matches!(result, Err(DomainError::InvalidInput { .. })));
        assert_eq!(project.name, before.name);
        assert_eq!(project.description, before.description);
        assert_eq!(project.revision, before.revision);
        assert_eq!(project.updated_at, before.updated_at);
    }
    let mut project = project();
    let before = project.clone();
    assert!(matches!(
        project.update_metadata(
            None,
            FieldUpdate::Set(Some("x".repeat(1001))),
            None,
            None,
            None
        ),
        Err(DomainError::InvalidInput { .. })
    ));
    assert_eq!(project.description, before.description);
    assert_eq!(project.revision, before.revision);
}

#[test]
fn project_validation_accepts_limits_and_rejects_one_over_limits() {
    let mut project = project();
    project.name = "n".repeat(255);
    project.description = Some("d".repeat(1000));
    assert!(
        project.validate().is_ok(),
        "documented size limits are inclusive"
    );
    project.name.push('n');
    assert!(matches!(
        project.validate(),
        Err(DomainError::InvalidInput { .. })
    ));
    project.name = "Valid project".to_owned();
    project.description = Some("d".repeat(1001));
    assert!(matches!(
        project.validate(),
        Err(DomainError::InvalidInput { .. })
    ));
    project.description = None;
    project.name = " \t\n".to_owned();
    assert!(matches!(
        project.validate(),
        Err(DomainError::InvalidInput { .. })
    ));
}

#[test]
fn builder_requires_name_owner_type_owner_id_and_creator() {
    let owner = Uuid::new_v4();
    let builders = [
        Project::builder()
            .owner_type(OwnerType::Personal)
            .owner_id(owner)
            .created_by(owner),
        Project::builder()
            .name("Catalog".to_owned())
            .owner_id(owner)
            .created_by(owner),
        Project::builder()
            .name("Catalog".to_owned())
            .owner_type(OwnerType::Personal)
            .created_by(owner),
        Project::builder()
            .name("Catalog".to_owned())
            .owner_type(OwnerType::Personal)
            .owner_id(owner),
    ];
    for builder in builders {
        assert!(matches!(
            builder.build(),
            Err(DomainError::InvalidInput { .. })
        ));
    }
}
