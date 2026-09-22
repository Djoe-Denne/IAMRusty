// Common fixtures are now in rustycog::testing
// pub mod common;
pub mod db;
pub mod idp_connect;

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

#[allow(unused_imports)]
pub use db::DbFixtures;
#[allow(unused_imports)]
pub use idp_connect::IdpConnectFixtures;
