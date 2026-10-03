//! Vendor-neutral federated OAuth contract (HTTP JSON + HMAC) for IAM ↔ `IdP` Connect.
//!
//! This crate does not talk to GitHub, GitLab, or any vendor URL. Connector
//! services implement [`FederatedOAuthClient`]; IAM consumes the same JSON
//! schema via [`ProviderTokens`] and [`ProviderUserProfile`].
//!
//! Provider identity on the wire is a string slug (`github`, `gitlab`, …), not
//! the IAM domain enum.
//!
//! Enable feature `server` for Axum 0.8 HMAC middleware and the three POST
//! handlers. Default features do not depend on Axum.

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub mod client;
pub mod dto;
pub mod error;
pub mod hmac;

#[cfg(feature = "server")]
pub mod server;

pub use client::FederatedOAuthClient;
pub use dto::{
    AuthorizeRequest, AuthorizeResponse, ProfileRequest, ProviderTokens, ProviderUserProfile,
    TokenRequest, AUTHORIZE_PATH, PROFILE_PATH, TOKEN_PATH,
};
pub use error::FederatedOAuthError;
pub use hmac::{
    canonical_string, sign, unix_timestamp_secs, verify, HmacError, HmacKey, MAX_SKEW_SECS,
    SIGNATURE_HEADER, TIMESTAMP_HEADER,
};
