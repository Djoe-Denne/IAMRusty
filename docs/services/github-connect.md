# GitHub Connect

Federated authenticator HTTP : IAM appelle ce service (HMAC S2S) pour OAuth GitHub. Pas un IdP plateforme.

- Préfixe : `/github-connect` — compose : **8085** HTTP / **8446** HTTPS
- JWT utilisateur : non (taxe `AppState` rustycog seulement)
- OpenFGA : non
- Events / Postgres : non
- Confiance IAM → Connect : HMAC [0409](../adr/0409-confiance-callback-oauth-idp-connect.md) (`METHOD\\nPATH\\nTIMESTAMP\\nBODY`, PATH préfixé `/github-connect/v1/…`, fenêtre 30 s)
- Secrets vendor (`client_id` / `client_secret`) : config du connecteur, **pas** IAM
- Allowlist `redirect_uris` = callbacks publics IAM (`/iam/api/auth/github/callback` et `relink-callback`)
- Compose interne : IAM joint `http://github-connect-service:8080/github-connect` (HTTP réseau, pas le port mesh 8446)

## Docs

- Canon : [ADR-0408](../adr/0408-connecteurs-idp-services-http.md), [0409](../adr/0409-confiance-callback-oauth-idp-connect.md)
- Extension : [`IAMRusty/docs/PROVIDER_FACTORY_GUIDE.md`](../../IAMRusty/docs/PROVIDER_FACTORY_GUIDE.md)
