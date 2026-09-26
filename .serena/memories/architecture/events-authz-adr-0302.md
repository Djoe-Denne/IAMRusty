Jalon : architecture actuelle / AuthN AuthZ (rétroactif).
Chemin : docs/adr/0302-authn-jwt-authz-openfga.md
Statut : Accepted · Réalité : Implemented

- AuthN Bearer HS256 rustycog-http ; iss=iamrusty, aud=aiforall (réalité runtime). AuthZ OpenFGA inchangée.
- SuperSédée par : — (l’ADR entière n’est pas SuperSédée).
- Cible issuer par trust domain = 0304/0305 ; seule la décision d’issuer unique est appelée à être remplacée à l’implémentation.

Voir le fichier ADR.