# Lazaret

Frontière de capacités et de données, distincte de Manifesto.

- Préfixe : `/lazaret` — compose : **8084** (`8084:8080`)
- Santé : `GET /lazaret/health`, `GET /lazaret/ready`
- JWT utilisateur rustycog : `[auth.jwt]` consommateur (HS256, `iss=iamrusty`, `aud=aiforall`) pour `UserIdExtractor` / `AppState` seulement — **ce n’est pas** l’identité workload
- Identité workload (T3) : `[identity]` — CSR → certificat client (CA logicielle `platform-internal-ca`), puis jeton de session dédié (`iss`/`aud`=`lazaret`, EdDSA). `POST /lazaret/enroll`, `POST /lazaret/session` (pas de middleware IAM). Un jeton crypto-valide n’autorise pas l’appel (T4+)
- Base : `lazaret_dev`
- OpenFGA : aucun type ce slice

## Docs

- [../guides/jwt-consommateur.md](../guides/jwt-consommateur.md)
