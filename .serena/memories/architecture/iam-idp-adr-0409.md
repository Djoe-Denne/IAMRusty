Jalon : IAM-IdP. Canon : `docs/adr/0409-confiance-callback-oauth-idp-connect.md`. Statut : Accepted. Réalité : Implemented.

- Callback navigateur = IAM ; connecteur S2S. `client_secret` vendor sur le connecteur. Linking reste IAM.
- HMAC v1 : `METHOD\nPATH\nTIMESTAMP\nBODY` ; PATH avec préfixe ; fenêtre 30 s ; compare constant-time.
- `redirect_uris[]` : callback et relink-callback depuis le registry (pas de 8081 hardcodé dans le handler). IT IAM : 8081 = port rustycog-testing.
- CSRF : `OAuthState` exp (TTL 600 s) + HMAC + anti-rejeu. Interdit bearer JWT user sur le connecteur.
- Voir le fichier ADR.