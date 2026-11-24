#[cfg(test)]
mod tests {
    use proptest::{prelude::*, test_runner::RngSeed};
    use serde::{Deserialize, Serialize};

    #[cfg(not(feature = "historical"))]
    include!("current.rs");
    #[cfg(feature = "historical")]
    include!("historical.rs");

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 512,
            rng_seed: RngSeed::Fixed(20261124),
            failure_persistence: None,
            ..ProptestConfig::default()
        })]

        #[test]
        fn pagination_crosses_the_exact_capacity_boundary(
            full_pages in prop_oneof![1u32..=1_000, (1u32 << 31)..u32::MAX, Just(u32::MAX)],
            page_size in prop_oneof![1u32..=100, Just(1u32 << 22), Just(1u32 << 28)],
        ) {
            // Construct an exact boundary by multiplication, independently of the DTO's division.
            let capacity = i64::from(full_pages) * i64::from(page_size);
            let exact = PaginationResponse::new(full_pages, page_size, Some(capacity));
            prop_assert_eq!(exact.total_pages, Some(full_pages));
            prop_assert!(!exact.has_next);

            let total = capacity + 1;
            let spill = PaginationResponse::new(full_pages, page_size, Some(total));
            prop_assert_eq!(
                spill.total_pages, Some(full_pages.saturating_add(1)),
                "item count = {}, page size = {}", total, page_size
            );
            prop_assert_eq!(spill.has_next, full_pages < u32::MAX);
        }
    }
}
