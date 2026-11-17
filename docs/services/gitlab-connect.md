# GitLab provider

The standalone GitLabConnect service and its dedicated tests were removed on
2026-11-15. Use the `gitlab` OIDC profile hosted by
[OAuthConnect](oauth-connect.md), at `/oauth-connect/gitlab`.

The public IAM callback URLs and provider ID remain unchanged. Repository
retirement does not prove live migration or equivalence between historical
GitLab IDs and OIDC subjects; a mismatch still blocks cutover.

See the [retirement contract](../contracts/legacy-connect-retirement.md) and
[historical handbook](../history/gitlab-connect-retired.md).
