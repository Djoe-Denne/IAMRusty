# GitLab Connect

Federated authenticator HTTP : IAM appelle ce service (HMAC S2S) pour OAuth GitLab. Pas un IdP plateforme.

- Préfixe : `/gitlab-connect` — compose : **8086** HTTP / **8447** HTTPS
- JWT utilisateur : non (taxe `AppState` rustycog seulement)
- OpenFGA : non
- Events / Postgres : non
- Confiance IAM → Connect : HMAC [0409](../adr/0409-confiance-callback-oauth-idp-connect.md) (`METHOD\\nPATH\\nTIMESTAMP\\nBODY`, PATH préfixé `/gitlab-connect/v1/…`, fenêtre 30 s)
- Secrets vendor (`client_id` / `client_secret`) : config du connecteur, **pas** IAM
- Allowlist `redirect_uris` = callbacks publics IAM (`/iam/api/auth/gitlab/callback` et `relink-callback`)
- Compose interne : IAM joint `http://gitlab-connect-service:8080/gitlab-connect` (HTTP réseau, pas le port mesh 8447)

## Docs

- Canon : [ADR-0408](../adr/0408-connecteurs-idp-services-http.md), [0409](../adr/0409-confiance-callback-oauth-idp-connect.md)
- Extension : [`IAMRusty/docs/PROVIDER_FACTORY_GUIDE.md`](../../IAMRusty/docs/PROVIDER_FACTORY_GUIDE.md)
