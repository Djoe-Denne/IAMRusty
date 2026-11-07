//! Minimal typed values retain optional presence until provenance/conflict checks.

use idp_connect_contract::{FederatedOAuthError, ProviderUserProfile};
use openidconnect::core::CoreUserInfoClaims;

pub(super) struct Profile {
    pub(super) subject: String,
    username: Option<String>,
    pub(super) email: Option<String>,
    verified: Option<bool>,
    picture: Option<String>,
}

impl Profile {
    pub(super) fn new(
        subject: &str,
        username: Option<&str>,
        email: Option<&str>,
        verified: Option<bool>,
        picture: Option<&str>,
    ) -> Result<Self, FederatedOAuthError> {
        if subject.is_empty() || email.is_some_and(str::is_empty) {
            return Err(FederatedOAuthError::UserProfile);
        }
        Ok(Self {
            subject: subject.to_owned(),
            username: username.map(str::to_owned),
            email: email.map(str::to_owned),
            verified,
            picture: picture.map(str::to_owned),
        })
    }

    pub(super) fn from_userinfo(info: &CoreUserInfoClaims) -> Result<Self, FederatedOAuthError> {
        Self::new(
            info.subject().as_str(),
            info.preferred_username().map(|value| value.as_str()),
            info.email().map(|value| value.as_str()),
            info.email_verified(),
            info.picture()
                .and_then(|localized| localized.get(None))
                .map(|value| value.as_str()),
        )
    }

    pub(super) fn merge(&mut self, other: Self) -> Result<(), FederatedOAuthError> {
        if self.subject != other.subject
            || conflicts(self.username.as_ref(), other.username.as_ref())
            || conflicts(self.email.as_ref(), other.email.as_ref())
            || conflicts(self.verified.as_ref(), other.verified.as_ref())
            || conflicts(self.picture.as_ref(), other.picture.as_ref())
        {
            return Err(FederatedOAuthError::UserProfile);
        }
        self.username = self.username.take().or(other.username);
        self.picture = self.picture.take().or(other.picture);
        if self.email.is_none() {
            self.email = other.email;
            // Verification comes from the email's source, not an isolated ID-token bool.
            self.verified = other.verified;
        }
        Ok(())
    }

    pub(super) fn into_profile(self) -> ProviderUserProfile {
        ProviderUserProfile {
            id: self.subject,
            username: self.username.unwrap_or_default(),
            email_verified: self.email.is_some() && self.verified.unwrap_or(false),
            email: self.email,
            avatar_url: self.picture,
        }
    }
}

fn conflicts<T: PartialEq>(left: Option<&T>, right: Option<&T>) -> bool {
    left.zip(right).is_some_and(|(left, right)| left != right)
}
