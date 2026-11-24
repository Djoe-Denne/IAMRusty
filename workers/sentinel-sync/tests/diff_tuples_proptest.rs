use std::collections::HashSet;

use proptest::prelude::*;
use proptest::test_runner::{RngAlgorithm, RngSeed};
use sentinel_sync::{fga_client::Tuple, reconcile::diff_tuples};
use uuid::Uuid;

const TUPLE_DOMAIN_SIZE: usize = 6;

fn bounded_tuple_domain() -> [Tuple; TUPLE_DOMAIN_SIZE] {
    [
        Tuple::user("project", Uuid::from_u128(1), "viewer", Uuid::from_u128(11)),
        Tuple::user("project", Uuid::from_u128(1), "editor", Uuid::from_u128(11)),
        Tuple::wildcard_user("project", Uuid::from_u128(1), "viewer"),
        Tuple::user(
            "component",
            Uuid::from_u128(2),
            "viewer",
            Uuid::from_u128(12),
        ),
        Tuple::object(
            "component",
            Uuid::from_u128(2),
            "parent",
            "project",
            Uuid::from_u128(1),
        ),
        Tuple::userset(
            "project",
            Uuid::from_u128(3),
            "viewer",
            "organization",
            Uuid::from_u128(4),
            "member",
        ),
    ]
}

fn collection_from_membership(
    membership: &[bool],
    domain: &[Tuple; TUPLE_DOMAIN_SIZE],
    duplicate: bool,
    reverse: bool,
) -> Vec<Tuple> {
    let mut tuples: Vec<_> = membership
        .iter()
        .zip(domain)
        .filter(|(present, _)| **present)
        .map(|(_, tuple)| tuple.clone())
        .collect();
    if duplicate {
        tuples.extend(tuples.clone());
    }
    if reverse {
        tuples.reverse();
    }
    tuples
}

fn as_set(tuples: &[Tuple]) -> HashSet<Tuple> {
    tuples.iter().cloned().collect()
}

#[derive(Debug, PartialEq, Eq)]
enum PlanError {
    DeleteMissing,
    WriteAlreadyPresent,
}

#[derive(Debug, PartialEq, Eq)]
enum OracleError {
    InvalidPlan(PlanError),
    ReconciledSetMismatch,
}

fn apply_plan(
    existing: &[Tuple],
    writes: &[Tuple],
    deletes: &[Tuple],
) -> Result<HashSet<Tuple>, PlanError> {
    let mut reconciled = as_set(existing);
    for tuple in writes {
        if !reconciled.insert(tuple.clone()) {
            return Err(PlanError::WriteAlreadyPresent);
        }
    }
    for tuple in deletes {
        if !reconciled.remove(tuple) {
            return Err(PlanError::DeleteMissing);
        }
    }
    Ok(reconciled)
}

fn reconcile_with_oracle(
    desired: &[Tuple],
    existing: &[Tuple],
    writes: &[Tuple],
    deletes: &[Tuple],
) -> Result<HashSet<Tuple>, OracleError> {
    let reconciled = apply_plan(existing, writes, deletes).map_err(OracleError::InvalidPlan)?;
    if reconciled != as_set(desired) {
        return Err(OracleError::ReconciledSetMismatch);
    }
    Ok(reconciled)
}

#[test]
fn empty_collections_reconcile_without_operations() {
    let (writes, deletes) = diff_tuples(&[], &[]);

    assert_eq!(writes, Vec::<Tuple>::new());
    assert_eq!(deletes, Vec::<Tuple>::new());
    assert_eq!(
        reconcile_with_oracle(&[], &[], &writes, &deletes),
        Ok(HashSet::new())
    );
}

#[test]
fn overlap_reconciles_only_the_difference() {
    let domain = bounded_tuple_domain();
    let desired = vec![domain[0].clone(), domain[1].clone()];
    let existing = vec![domain[1].clone(), domain[2].clone()];
    let (writes, deletes) = diff_tuples(&desired, &existing);

    assert_eq!(as_set(&writes), as_set(&[domain[0].clone()]));
    assert_eq!(as_set(&deletes), as_set(&[domain[2].clone()]));
    assert_eq!(
        reconcile_with_oracle(&desired, &existing, &writes, &deletes),
        Ok(as_set(&desired))
    );
}

#[test]
fn duplicates_and_reordering_preserve_operation_sets() {
    let domain = bounded_tuple_domain();
    let desired = vec![domain[0].clone(), domain[1].clone(), domain[0].clone()];
    let existing = vec![domain[1].clone(), domain[2].clone(), domain[2].clone()];
    let (writes, deletes) = diff_tuples(&desired, &existing);

    let mut changed_desired = desired;
    changed_desired.rotate_left(1);
    changed_desired.push(domain[1].clone());
    let mut changed_existing = existing;
    changed_existing.reverse();
    changed_existing.push(domain[2].clone());
    let (changed_writes, changed_deletes) = diff_tuples(&changed_desired, &changed_existing);

    assert_eq!(as_set(&writes), as_set(&changed_writes));
    assert_eq!(as_set(&deletes), as_set(&changed_deletes));
}

#[test]
fn oracle_rejects_a_missing_delete() {
    let domain = bounded_tuple_domain();
    let desired = vec![domain[0].clone()];
    let existing = vec![domain[0].clone(), domain[1].clone()];

    assert_eq!(
        reconcile_with_oracle(&desired, &existing, &[], &[]),
        Err(OracleError::ReconciledSetMismatch)
    );
}

#[test]
fn oracle_rejects_a_missing_write() {
    let domain = bounded_tuple_domain();
    let desired = vec![domain[0].clone(), domain[1].clone()];
    let existing = vec![domain[0].clone()];

    assert_eq!(
        reconcile_with_oracle(&desired, &existing, &[], &[]),
        Err(OracleError::ReconciledSetMismatch)
    );
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 512,
        max_shrink_iters: 1_000,
        max_shrink_time: 1_000,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(20_261_124),
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn diff_plan_is_exact_idempotent_and_invariant(
        desired_membership in prop::collection::vec(any::<bool>(), TUPLE_DOMAIN_SIZE),
        existing_membership in prop::collection::vec(any::<bool>(), TUPLE_DOMAIN_SIZE),
        duplicate_desired in any::<bool>(),
        duplicate_existing in any::<bool>(),
        reverse_desired in any::<bool>(),
        reverse_existing in any::<bool>(),
    ) {
        let domain = bounded_tuple_domain();
        let base_desired =
            collection_from_membership(&desired_membership, &domain, false, false);
        let base_existing =
            collection_from_membership(&existing_membership, &domain, false, false);
        let desired = collection_from_membership(
            &desired_membership,
            &domain,
            duplicate_desired,
            reverse_desired,
        );
        let existing = collection_from_membership(
            &existing_membership,
            &domain,
            duplicate_existing,
            reverse_existing,
        );
        let desired_set = as_set(&desired);
        let existing_set = as_set(&existing);
        let (writes, deletes) = diff_tuples(&desired, &existing);
        let writes_set = as_set(&writes);
        let deletes_set = as_set(&deletes);

        prop_assert_eq!(writes.len(), writes_set.len(), "duplicate planned write");
        prop_assert_eq!(deletes.len(), deletes_set.len(), "duplicate planned delete");

        let reconciled = reconcile_with_oracle(&desired, &existing, &writes, &deletes);
        prop_assert_eq!(
            reconciled.as_ref(),
            Ok(&desired_set),
            "applying the plan did not produce exactly the desired set: {:?}",
            reconciled
        );
        let reconciled = reconciled.unwrap_or_default();
        let reconciled_vec: Vec<_> = reconciled.iter().cloned().collect();
        let (repeat_writes, repeat_deletes) = diff_tuples(&desired, &reconciled_vec);
        prop_assert!(repeat_writes.is_empty(), "reconciled set still needs writes");
        prop_assert!(repeat_deletes.is_empty(), "reconciled set still needs deletes");

        let expected_writes: HashSet<_> =
            desired_set.difference(&existing_set).cloned().collect();
        let expected_deletes: HashSet<_> =
            existing_set.difference(&desired_set).cloned().collect();
        prop_assert_eq!(
            &writes_set,
            &expected_writes,
            "writes are not exactly desired minus existing"
        );
        prop_assert_eq!(
            &deletes_set,
            &expected_deletes,
            "deletes are not exactly existing minus desired"
        );

        let (base_writes, base_deletes) = diff_tuples(&base_desired, &base_existing);
        prop_assert_eq!(
            &writes_set,
            &as_set(&base_writes),
            "duplicates or ordering changed the write set"
        );
        prop_assert_eq!(
            &deletes_set,
            &as_set(&base_deletes),
            "duplicates or ordering changed the delete set"
        );
    }
}
