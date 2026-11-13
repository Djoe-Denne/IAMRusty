//! QA coverage90-local B: component lifecycle refuses skipped/backwards stages.
use manifesto_domain::{ComponentStatus, DomainError, ProjectComponent};
use uuid::Uuid;

#[test]
fn component_lifecycle_records_each_first_transition_only_once() {
    let mut component = ProjectComponent::new(Uuid::new_v4(), "hive".to_owned()).unwrap();
    component
        .transition_status(ComponentStatus::Pending)
        .unwrap();
    assert_eq!(component.configured_at, None);
    assert_eq!(component.activated_at, None);
    assert_eq!(component.disabled_at, None);

    component
        .transition_status(ComponentStatus::Configured)
        .unwrap();
    let configured = component.configured_at.expect("configuration timestamp");
    component
        .transition_status(ComponentStatus::Configured)
        .unwrap();
    assert_eq!(component.configured_at, Some(configured));
    assert!(!component.is_active());

    component
        .transition_status(ComponentStatus::Active)
        .unwrap();
    let activated = component.activated_at.expect("activation timestamp");
    assert!(activated >= configured);
    assert!(component.is_active());
    component
        .transition_status(ComponentStatus::Active)
        .unwrap();
    assert_eq!(component.activated_at, Some(activated));

    component
        .transition_status(ComponentStatus::Disabled)
        .unwrap();
    let disabled = component.disabled_at.expect("disabling timestamp");
    assert!(disabled >= activated);
    assert!(!component.is_active());
    component
        .transition_status(ComponentStatus::Disabled)
        .unwrap();
    assert_eq!(component.disabled_at, Some(disabled));
    assert_eq!(component.configured_at, Some(configured));
    assert_eq!(component.activated_at, Some(activated));
}

#[test]
fn illegal_component_transitions_preserve_status_and_all_timestamps() {
    use ComponentStatus::{Active, Configured, Disabled, Pending};
    for (source, target) in [
        (Pending, Active),
        (Pending, Disabled),
        (Configured, Pending),
        (Configured, Disabled),
        (Active, Pending),
        (Active, Configured),
        (Disabled, Pending),
        (Disabled, Configured),
        (Disabled, Active),
    ] {
        let mut component = ProjectComponent::new(Uuid::new_v4(), "hive".to_owned()).unwrap();
        component.status = source;
        let before = component.clone();
        let result = component.transition_status(target);
        assert!(
            matches!(result, Err(DomainError::BusinessRuleViolation { .. })),
            "{source:?} -> {target:?}: {result:?}"
        );
        assert_eq!(component.status, source);
        assert_eq!(component.configured_at, before.configured_at);
        assert_eq!(component.activated_at, before.activated_at);
        assert_eq!(component.disabled_at, before.disabled_at);
    }
}

#[test]
fn component_type_creation_and_revalidation_enforce_nonblank_and_maximum_length() {
    for component_type in [String::new(), " \t\n".to_owned(), "c".repeat(101)] {
        assert!(matches!(
            ProjectComponent::new(Uuid::new_v4(), component_type.clone()),
            Err(DomainError::InvalidInput { .. })
        ));
        let mut component = ProjectComponent::new(Uuid::new_v4(), "hive".to_owned()).unwrap();
        component.component_type = component_type;
        assert!(matches!(
            component.validate(),
            Err(DomainError::InvalidInput { .. })
        ));
    }
    let component = ProjectComponent::new(Uuid::new_v4(), "c".repeat(100)).unwrap();
    assert!(
        component.validate().is_ok(),
        "100-character component type is valid"
    );
    assert_eq!(component.status, ComponentStatus::Pending);
}
