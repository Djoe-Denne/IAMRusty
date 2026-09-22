#![allow(
    missing_docs,
    clippy::return_self_not_must_use,
    clippy::must_use_candidate
)]

pub mod github;

/// Distinguishes “leave the default” from “assign this value” (including `None`).
#[derive(Debug, Clone, Default)]
pub enum OptionalField<T> {
    #[default]
    Unset,
    Set(Option<T>),
}

impl<T: Clone> OptionalField<T> {
    pub fn resolve(&self, default: Option<T>) -> Option<T> {
        match self {
            Self::Unset => default,
            Self::Set(value) => value.clone(),
        }
    }
}
