# QA Test Scenarios

This directory contains comprehensive test scenarios for the IAM Service API, organized by functional area.

## User Creation (Primary Authentication)

### OAuth Providers
- `01_user_creation_github_oauth_success.md` - GitHub OAuth registration flow
- `02_user_creation_github_oauth_failure.md` - GitHub OAuth error handling
- `03_user_creation_gitlab_oauth_success.md` - GitLab OAuth registration flow
- `04_user_creation_gitlab_oauth_failure.md` - GitLab OAuth error handling

### Email/Password
- `05_user_creation_email_password_success.md` - Email/password registration flow
- `06_user_creation_email_password_failure.md` - Email/password error handling

## Provider Linking (Secondary Authentication)

- `07_link_gitlab_to_github_user.md` - Link GitLab to GitHub-primary user
- `08_link_github_to_gitlab_user.md` - Link GitHub to GitLab-primary user
- `09_add_password_to_oauth_user.md` - Add password auth to OAuth users
- `10_add_oauth_to_password_user.md` - Add OAuth providers to password users

## Authentication & Login

- `11_login_with_oauth_existing_user.md` - OAuth login for existing users
- `12_login_email_password_success.md` - Email/password login success
- `13_login_email_password_failure.md` - Email/password login failures

## Token Management

- `14_token_refresh_success.md` - Token refresh with rotation
- `15_token_refresh_failure.md` - Token refresh error scenarios

## Password Management

- `16_password_reset_unauthenticated_success.md` - Forgot password flow
- `17_password_reset_authenticated_success.md` - Change password while logged in

## Account Management

- `18_username_availability_check.md` - Username availability and suggestions
- `19_email_verification_success.md` - Email verification flow

## Provider Management

- `20_provider_token_retrieval.md` - Internal provider token access
- `21_provider_token_revocation.md` - Provider token removal
- `22_provider_relinking_success.md` - Update provider credentials

## Security & Edge Cases

- `23_link_provider_conflicts.md` - Provider linking conflict resolution
- `24_jwt_token_validation.md` - JWT token security and validation
- `25_user_profile_management.md` - User profile consistency
- `26_security_edge_cases.md` - Security vulnerability testing

## Integration & Workflow

- `27_complete_multi_provider_workflow.md` - End-to-end multi-provider testing
- `28_api_resilience_testing.md` - API resilience and error handling

## Usage

Each scenario file follows a consistent format:
- **What we want to test**: Objective description
- **Why**: Business rationale  
- **How**: Step-by-step API calls and data flow
- **Expectation**: Expected results and status codes

These scenarios can be implemented using any testing framework that can make HTTP requests and validate responses. 