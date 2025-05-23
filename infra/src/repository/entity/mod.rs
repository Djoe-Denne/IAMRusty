//! SeaORM entity definitions

pub mod user;
pub mod provider_token;
pub mod refresh_token;

pub mod prelude {
    pub use super::user::Entity as User;
    pub use super::provider_token::Entity as ProviderToken;
    pub use super::refresh_token::Entity as RefreshToken;
} 