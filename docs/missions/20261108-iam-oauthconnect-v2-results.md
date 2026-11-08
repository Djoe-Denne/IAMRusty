# IAM → OAuthConnect v2 — résultats locaux du 8 novembre 2026

## Résultat et portée

LOGIN, LINK et RELINK utilisent `authorize_authentication` et `complete_authorization` v2 exclusivement, avec nonce IAM indépendant durable et consommation/double-purge commitée avant I/O. Migration additive nullable, règles UUID/liens/email préservées, aucune bascule de déploiement ni migration de comptes. Contrat : [IAM v2](../contracts/iam-oauthconnect-v2.md), ADR0413 **Accepted / Partial**. SDK inchangé : `93b890ff7b821e9cb2e7d0aeb81957d098778a2f`.

**325 unités +80 IT passent**, sans échec, ignoré ni filtrage dans les suites finales retenues. Ce résultat qualifie ce périmètre local, pas tout IAM, les fournisseurs réels ou le déploiement. Production finale revue au commit `9d10e3b`, ajouts G7 au `79105b8`, préimage upgrade corrigée dans les tests seulement au `b28a922ea479f74457adc484e7fcc7c63fecf235`.

## Commandes et résultats réels

Windows natif, un seul processus Cargo à la fois,24jobs et target `E:\cargo-target\AIForAll` du dépôt, sans override contradictoire.

- `cargo test --locked -p iam-domain -p iam-application --lib` : domaine87, application30 PASS.
- `cargo test --locked -p iam-infra -p iam-http_server --lib --no-fail-fast` : infra146, HTTP62 PASS. Les4unités SQL writer et22tests transport (11v2+11v1) sont **inclus** dans infra146, pas ajoutés une deuxième fois.
- `cargo check --locked -p iam-domain -p iam-application -p iam-infra -p iam-http_server -p iam-setup -p iam-migration -p iam-service -p oodhive-monolith --all-targets` : PASS.
- Fmt ciblé et `cargo clippy --locked -p iam-domain -p iam-application -p iam-migration --all-targets -- -D warnings` : PASS.
- Clippy strict `iam-infra --all-targets -- -D warnings` : **rouge**, neuf diagnostics `dead_code` préexistants/non modifiés ; aucun allow ou masquage. Le diagnostic non strict avec `-W clippy::future_not_send -W clippy::too_many_lines` passe sans nouveaux warnings de ces familles ; ce n'est pas un strict GREEN. Aucun scan Sonar nouveau exécuté.
- Onze binaires IT compilés avec `--no-run`, puis **réellement exécutés** chacun par `ops/tests/it-native.ps1 -Suite iam -TestBinary <nom>` ; jamais la suite IAM entière, qui active aussi des queues hors périmètre.

IT finales, sans ignoré/filtré :

- `auth_transactions_postgres` :9 PASS ; `oauth_browser_transactions` :17 PASS.
- `oauth_cleanup_lifecycle` :2 ; `auth_oauth_start` :11 ; `auth_oauth_callback` :17.
- `auth_oauth_provider_failures` :4 ; `relink_provider` :11 ; `verified_idp_https` :5.
- `organization_account_guard` :2 ; `app_cleanup_supervision` :1 ; `request_trace_redaction` :1.

Logs originaux et validés distincts : `.tmp/iam-oauthconnect-v2/`. Les résultats unitaires encore valides sont réutilisés ; les derniers changements n'affectent que les tests IT et documents.

## TDD, précédences et cas difficiles

RED compilables avant production : transport3PASS/8FAIL, nonce9PASS/13FAIL, writer SQL3PASS/1FAIL, LOGIN4FAIL, LINK/RELINK8PASS/6FAIL. Puis GREEN sur les mêmes oracles. Échecs de compilation, de préconditions ou d'environnement non comptés comme RED métier.

Les IT prouvent notamment le verrou writer/concurrence, un seul gagnant, les lectures indépendantes après commit, le nonce persistant observé avant authorize, la purge des deux secrets observée avant la réponse complete, les bindings invalides intacts, le refus legacy/corrompu après consommation, le replay sans I/O et l'absence d'effets IAM après refus ou tokens sans profil. Upgrade : vrai Migrator enregistré1→all dans un schéma UUID transactionnel sans `public`/reset, conservation comptes/liens/emails/contraintes et colonne nullable sans défaut. Ce n'est pas une répétition d'un déploiement avec commits séparés.

T-1/G7 fermé par deux témoins réels : RELINK ignore l'email d'un autre compte sans le réaffecter ; LINK ajoute un nouvel email à la cible exacte avec `new_email_added=true` et `is_verified=false`, même si le provider le dit vérifié. Q1 : collision RELINK vers l'identité déjà liée ailleurs, sans email, refusée par la frontière existante et préserve les deux liens/tokens ; aucune nouvelle garde métier ajoutée.

**Bilan fonctionnel groupé : aucun cas rare/ambigu encore rouge dans les onze suites exécutées.** Aucun test supprimé, ignoré ou oracle affaibli pour verdir.

## Deux incidents conservés, sans correction métier

1. Premier run Postgres :8PASS/1échec avant l'upgrade. Un INSERT de préimage `user_emails` omettait son UUID obligatoire. Correction du montage test uniquement, mêmes oracles, puis9PASS réels.
2. Premier run callback :17échecs de setup, avant assertions, sur `opaque fixture start failure; identity UNKNOWN`, puis collisions du mapping déclaré54070. Réessai unique explicitement demandé :17PASS sans changement de code/harness/SDK. Cause initiale non établie ; ni le retry ni le précédent QMD du19octobre ne prouvent une cause identique. Les logs d'échec restent conservés.

## Revues et limites

Revues indépendantes sécurité (producteurs/LOGIN puis delta LINK) et correctness : PASS **statique borné**, aucun défaut confirmé ; empreintes recoupées parent. Revue tests : T-1 MEDIUM traité et exécuté ; T-2 LOW accepté comme gap. Rapports/briefings et INDEX locaux persistés avant corrections, référencés par le ledger.

Restent explicitement non qualifiés :

- G5 chaîne IAM→Connect réel→vendor signé, faute couture existante dans le périmètre sans nouveau harness/dépendance ; mocks S2S ne constituent pas cette preuve OIDC.
- Streaming sans `Content-Length`, timeout pendant lecture d'un corps au-goutte et garantie RSS/décompression : pas d'oracle dynamique ajouté. Reader à comptage de chunks et portée du timeout revus dans les sources ; borne1MiB±1/timeout avant réponse/TLS CA-SAN-redirect exécutés.
- GitHub/GitLab réels, équivalence exacte `sub` GitLab/ancien ID, déploiement/cutover et retrait legacy : propriétaire externe. Aucun changement Compose/Kind/SDK pour ces gates.
- Pas de retry applicatif/fallback ; les reprises HTTP2 conditionnelles de reqwest0.11.27 ne sont pas démontrées actives avec les features TLS inspectées. Rouvrir le point si ces features changent, pas de HTTP1 forcé.

## Runtime et clôture

Le daemon était déjà actif pour d'autres travaux. Aucun démarrage Docker/Kind/WSL par cette mission. **Onze conteneurs avec propriété prouvée ont été retirés, zéro possédé running restant.** Les quatre nœuds Kind préexistants sont toujours running et ne sont pas arrêtés. Aucune image nouvelle observée.

**Nettoyage global non déclaré terminé** : le conteneur `5f5d1b72938b8346ad5c234e4f1ddecab3ad58e04de0fcea817576fae36881e0` reste `created`, événement Engine sans ID de création retourné à la fixture ; conservé `ask`, pas adopté/supprimé. Douze nouveaux volumes anonymes sont observés entre les inventaires, mais leurs noms n'ont pas été capturés à la création par le harness : enregistrés tardivement et honnêtement `UNKNOWN/ask` avec `observed_name`, sans inférer la propriété du seul delta. Aucun volume supprimé. Cette limite du registre n'est pas masquée par les PASS des tests et ne donne pas autorisation de modifier le harness dans ce lot.

Registre append-only : `.tmp/docker-resources-20261108.log`. Inventaires avant/après et noms observés : `.tmp/iam-oauthconnect-v2/runtime-{before,after}-{containers,images,volumes}.jsonl`. Les13anciens conteneurs exited disparus pendant la campagne ne sont pas attribués à notre nettoyage. Aucun arrêt global Desktop/WSL ni mesure RAM libérée revendiquée. Publication et retrait des worktrees sont vérifiés séparément dans le ledger de clôture.
