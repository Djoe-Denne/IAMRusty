Jalon : architecture actuelle / AuthN JWT (complète 0302).
Chemin : docs/adr/0304-jwt-acces-plateforme-rs256-jwks.md
Statut : Accepted · Réalité : Unimplemented

- Access JWT = RS256 + kid opaque + JWKS ; SigningProvider (OpenBao Transit Sign, clé non exportée, ou PEM dev) ; issuer par trust domain (`https://{host}/iam` et `https://{host}/iam/orgs/{slug}`, pas un sous-domaine).
- SuperSède « OpenBao ne signe pas » (ancienne 0304) + **cible seulement** la décision 0302 `iss=iamrusty` unique — **pas** 0302 entière (OpenFGA, Bearer, aud=aiforall restent).
- Quatre rôles : émetteur logique IAM, propriétaire clé (plateforme|org), SigningProvider, JWKS `GET /iam/.well-known/jwks.json`. KMS = LOGIN/REFRESH only. Principal = (iss, sub). typ cible = `aiforall-access+jwt`.
- Runtime : HS256 partagé ; boot refuse RS256 ; JWKS inutilisé ; Transit = Cosign Apparatus only.

Voir le fichier ADR.