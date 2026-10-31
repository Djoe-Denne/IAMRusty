# GitHub Connect

Federated authenticator HTTP : IAM appelle ce service (HMAC S2S) pour OAuth GitHub. Pas un IdP plateforme.

- Préfixe : `/github-connect` — compose : **8085** HTTP / **8446** HTTPS
- JWT utilisateur : non (taxe `AppState` rustycog seulement)
- OpenFGA : non
- Events / Postgres : non
- Confiance IAM → Connect : HMAC [0409](../adr/0409-confiance-callback-oauth-idp-connect.md) (`METHOD\\nPATH\\nTIMESTAMP\\nBODY`, PATH préfixé `/github-connect/v1/…`, fenêtre 30 s)
- `client_id` est public ; le secret OAuth est acquis par le connecteur, **pas** IAM.
- Allowlist `redirect_uris` = callbacks publics IAM (`/iam/api/auth/github/callback` et `relink-callback`)
- Compose interne : IAM joint `http://github-connect-service:8080/github-connect` (HTTP réseau, pas le port mesh 8446)

## Docs

- Canon : [ADR-0408](../adr/0408-connecteurs-idp-services-http.md), [0409](../adr/0409-confiance-callback-oauth-idp-connect.md)
- Extension : [`services/IAMRusty/docs/PROVIDER_FACTORY_GUIDE.md`](../../services/IAMRusty/docs/PROVIDER_FACTORY_GUIDE.md)

## Secrets de démarrage OpenBao

Le setup async résout `github.client_secret_ref` et `github.hmac_secret_ref` via
`rustycog::config::secrets` (feature optionnelle `secrets`). Le loader strict
reste déclaratif : `.env`, fichier choisi par `RUN_ENV` relativement au répertoire
de travail, puis variables préfixées. Il ne contacte jamais le coffre et ne
revient pas aux defaults sur erreur. L’application n’est retournée qu’après
authentification et validation des **deux** secrets.

Exemple sans valeurs sensibles (adapter origine, identité et références) :

```toml
[github]
secrets_mode = "openbao"
client_id = "PUBLIC_OAUTH_CLIENT_ID"
# Conserver auth_url, token_url, user_url, redirect_uris et pkce_supported.

[github.client_secret_ref]
mount = "secret"
path = "github-connect/oauth"
field = "client_secret"

[github.hmac_secret_ref]
mount = "secret"
path = "github-connect/iam"
field = "hmac_secret"

[openbao]
endpoint = "https://bao.example.invalid:8200"
allow_loopback_http = false
# ca_cert_file = "/run/secrets/bao-ca.pem" # ajoute des racines PEM vérifiées

[openbao.auth]
method = "token_file"
path = "/run/secrets/github-connect.bao-token" # Windows : C:/secrets/...
```

Préfixe `GITHUB_CONNECT_`, niveaux séparés par `__` :

- `GITHUB_CONNECT_GITHUB__SECRETS_MODE=openbao` ; références :
  `GITHUB_CONNECT_GITHUB__CLIENT_SECRET_REF__MOUNT`, `__PATH`, `__FIELD`,
  et mêmes suffixes pour `HMAC_SECRET_REF`.
- `GITHUB_CONNECT_OPENBAO__ENDPOINT`, `__ALLOW_LOOPBACK_HTTP`, `__CA_CERT_FILE` ;
  `GITHUB_CONNECT_OPENBAO__AUTH__METHOD=token_file`, `__AUTH__PATH`.

Une référence contient les trois champs obligatoires `mount/path/field`. Mount
et path sont relatifs, segments ASCII alphanumériques ou `_.-`, hors `.`/`..` ;
field est une clé littérale, pas JSONPath. Maximum 1 024 octets par composant.
Lecture KV **v2** : `GET /v1/<mount>/data/<path>`, champ string de `data.data`.
Valeur non blanche, au plus 16 KiB ; aucune coercition ni décodage.

### Identité et entrées à provisionner

Créer deux documents dans le mount KV v2 `secret` :

- `github-connect/oauth`, champ `client_secret` : secret OAuth GitHub.
- `github-connect/iam`, champ `hmac_secret` : **même secret partagé que celui
  utilisé par IAM**. Le connecteur conserve exactement `trim()` puis minimum
  16 octets, sans hash, base64 ou changement de protocole. IAM est inchangé.

Fournir un fichier régulier contenant un token OpenBao UTF-8 non-root sur une
ligne (LF/CRLF terminal accepté), protégé par ACL Windows/POSIX. Il est fourni
par un agent authentifié via identité workload ou par un bootstrap dédié borné :
**l’identité reste nécessaire**. Aucun login workload natif, `VAULT_TOKEN`,
`BAO_TOKEN`, token inline ou credential ambiant n’est pris en charge.

`lookup-self` doit renvoyer une TTL restante de 1 à 3 600 s et aucune policy
`root` (y compris identity policies). Droits requis, à adapter aux références :

```hcl
path "auth/token/lookup-self" { capabilities = ["read"] }
path "secret/data/github-connect/oauth" { capabilities = ["read"] }
path "secret/data/github-connect/iam"   { capabilities = ["read"] }
```

Pas de droit list/write/Transit/admin. KV autorise au niveau du document, pas
du champ : ne pas y mélanger d’autres secrets. L’absence de policy `root` ne
prouve pas qu’une policy personnalisée est minimale ; émission, TTL maximale,
ACL du fichier et moindre privilège restent à garantir par le déploiement.

### Transport, erreurs et compatibilité

HTTPS vérifie chaîne et nom d’hôte. Endpoint origine seulement, sans userinfo,
query, fragment ou chemin de base. HTTP exige `allow_loopback_http=true` **et**
une IP littérale loopback ; `localhost` et les réseaux privés ne suffisent pas.
Aucun proxy ambiant, redirect (y compris HA), failover ou retry applicatif.
Connexion 3 s, requête entière 5 s, réponse lue incrémentalement jusqu’à 64 KiB ;
budget d’acquisition au boot 20 s. Les erreurs publiques sont expurgées, sans
URL, token, référence, corps distant ou valeur de secret.

Tout refus d’accès, indisponibilité, référence absente, réponse/secret invalide
ou timeout refuse le démarrage. Aucun retour aux valeurs `.env`, aux fixtures
ou aux defaults. Les champs directs historiques restent disponibles **uniquement**
avec `github.secrets_mode="direct"`, les deux valeurs valides et aucune section
OpenBao/référence. Absence de mode ou mélange = refus avant I/O coffre. Les
configs nominales default/development passent à OpenBao ; les fixtures test
déclarent `direct`. Les anciens déploiements doivent donc être raccordés
explicitement par la mission de déploiement (aucun Compose/ops changé ici).

Après démarrage, les valeurs ne résident que chez les consommateurs OAuth/HMAC,
pas dans la configuration conservée/sérialisée ou un stockage métier. Il ne
s’agit pas d’une garantie de zeroization complète de la RAM. Aucun appel coffre
par requête, cache, renouvellement ou rotation à chaud : une mise à jour exige
un **redémarrage contrôlé**, coordonné avec IAM pour le HMAC. Les deux lectures
KV ne forment pas une transaction atomique.

Contrat détaillé : [startup-secrets-openbao](../contracts/startup-secrets-openbao.md).
L’[ADR0505](../adr/0505-startup-secrets-sdk-openbao.md) reste Proposed ; son statut
n’est pas une acceptation implicite de la mission. GitHub App installation/API
reste un autre jeu d’identifiants logique et un flux hors de ce lot.
