# GitHub provider

The standalone GitHubConnect service and its dedicated tests were removed on
2026-11-15. GitHub sign-in now belongs to the `github` OAuth profile hosted by
[OAuthConnect](oauth-connect.md), at `/oauth-connect/github`.

Public IAM callback URLs and the OpenBao paths `github-connect/oauth` and
`github-connect/iam` are unchanged. Do not delete or rename those secrets.

See the [retirement contract](../contracts/legacy-connect-retirement.md) and
[historical handbook](../history/github-connect-retired.md).
