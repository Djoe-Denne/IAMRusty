# GitHubConnect — résultats secrets de démarrage OpenBao

## Mandat et périmètre

Mission utilisateur du 2026-10-31, **pas** acceptation d’ADR : acquérir au boot
le secret OAuth `github.client_secret` et le HMAC partagé avec IAM, depuis des
références KV v2, via une capacité SDK générique optionnelle. Aucun changement
IAM/Lazaret, déploiement/Compose/ops/ESO/E2E, seed, Transit, rotation à chaud ou
flux GitHub App installation/API.

Base constatée : AIForAll `master@4d1a137` (GitHubConnect identique à `8698d5a`),
gitlink `02e5db2`. L’utilisateur a ensuite autorisé la publication de tout travail
durable préexistant et le travail direct sur les principales : AIForAll/master,
RustyCog/main (noms confirmés, pas de renommage).

## Contrat livré

- [Configuration, références, auth, droits et raccord pipeline](../services/github-connect.md#secrets-de-démarrage-openbao).
- [Contrat SDK/service](../contracts/startup-secrets-openbao.md).
- [ADR0505](../adr/0505-startup-secrets-sdk-openbao.md) **Proposed**, sans promotion
  implicite en Accepted.
- Feature SDK `secrets`, `rustycog::config::secrets` ; loader strict additif,
  sans réseau ni fallback. Snapshot acquis au setup async, consommé seulement
  par OAuth/HMAC, absent du config conservé et de sa sérialisation.
- V1 token-file externe, lookup-self non-root/TTL1..3600 ; HTTPS vérifié,
  exception HTTP littérale loopback explicite, aucun proxy/redirect/failover.
  Timeout requête5s, connexion3s, réponse64KiB, secret16KiB, boot20s.
- Mode legacy direct conservé uniquement explicite/exclusif ; aucune rotation,
  relecture par requête, renouvellement ou effacement du fichier credential.

## Intégration du préexistant autorisée

SDK : `9a0a794` conserve les ajouts JWT/config outils locaux ; merge du distant
`9e9ef27`, puis du gitlink `02e5db2` en `54d435e`. Conflit unique combiné dans
`jwt_handler` : conserver à la fois le principal policy explicite et les issuers
plateforme/HS256 distincts. `2bdbc2a` adapte quatre fixtures à la policy requise.
La différence résiduelle des deux `.serena/project.yml` était uniquement la
liste commentée des langages générés ; la liste la plus récente est conservée.

Les livrables documentaires préexistants (rapports de missions, wiki, candidate
locale d’analyse Python) sont préservés et publiés séparément. Aucun log/cache,
secret personnel ou donnée d’exécution n’est intégré. Les worktrees/branches
d’autres missions ne sont ni supprimés ni adoptés.

## Vérification et publication

### SDK

Publié : `93b890ff7b821e9cb2e7d0aeb81957d098778a2f`, branche `origin/main`,
référence distante vérifiée après push. Ce commit descend du distant9e9ef27,
du pin02e5db2 et des intégrations préexistantes décrites ci-dessus. Le checkout
AIForAll/rustycog est sur ce commit ; le contenu local initial de
`.serena/project.yml` a été préservé à l’identique (blob
`e296b1da601f483283809cbe072ab05a04df83d5`) dans le commit publié avant le checkout.

Commandes Windows, slot Cargo unique, jobs24 et target E: conservés :

- `cargo test --manifest-path ../rustycog/Cargo.toml --no-default-features --features secrets --lib config::`
  : **11 tests PASS**, aucun échec/ignore/filtré (5 secrets,1 loader strict,
  5 tests config existants).
- `cargo check --manifest-path ../rustycog/Cargo.toml --no-default-features --features config`
  : PASS sans activer le réseau secrets.
- `cargo clippy --manifest-path ../rustycog/Cargo.toml --no-default-features --features secrets --lib -- -D warnings`
  : PASS.
- Réconciliation JWT : `cargo test --manifest-path ../rustycog/Cargo.toml --no-default-features --features test-utils --lib jwt_`
  : **23 tests PASS**, puis filtre `seed`23 PASS. Ces filtres se recouvrent :
  ne pas additionner leurs nombres comme des tests distincts. Le premier essai
  featurehttp seul ne compilait pas les tests gatedtesting (1 test seulement).
- Revue sécurité indépendante SDK : PASS **statique borné**, aucune exécution
  du reviewer et aucune qualification de composition service par ce verdict.

### GitHubConnect

- `cargo test -p github-connect-configuration -p github-connect-infra -p github-connect-setup -p github-connect-service`
  : **17 tests PASS**, aucun échec/ignore/filtré dans ces suites (config1,
  strict_secrets2, infra4, HMAC7, startup OpenBao3). Les cibles vides et doc-tests
  ne sont pas comptées comme des contrôles supplémentaires.
- OAuth : les credentials HTTP sortants correspondent au secret KV résolu ;
  HMAC : la clé résolue avec le trim historique est acceptée, l’ancienne et la
  clé de remplacement sont refusées. Le config conservé reste vide pour les
  deux valeurs ; Debug/sérialisation/erreurs et tracing du loader n’exposent
  pas les sentinelles.
- Les erreurs403/404, type invalide, valeur vide/HMAC trop court et endpoint
  inaccessible empêchent l’application. Modes absents/ambigus et référence
  manquante sont refusés avant I/O ; le mode direct explicite fonctionne sans
  coffre. Après changement de la fixture, les requêtes métier ne contactent
  pas OpenBao : snapshot jusqu’au redémarrage.
- Correction d’une fixture OAuth existante : `set_body_string` de WireMock
  force `text/plain` malgré l’en-tête manuel ; passage à `set_body_raw` JSON,
  sans assouplir le client OAuth. Le premier run infra était rouge pour cette
  cause ; un diagnostic lancé avec filtre exact incomplet a exécuté0 test et
  n’est pas une preuve. Le run final complet est vert.
- Clippy `--all-targets -W clippy::future_not_send -W clippy::too_many_lines` :
  exit0, aucun nouveau diagnostic après correction des rustdocs. Avertissements
  **préexistants** restants : quatre missing-docs dans transport.rs, un dans
  idp-connect-contract/dto.rs et async_fn_in_trait dans le module serveur SDK.
  Ce n’est pas un PASS Clippy service avec `-D warnings`.
- Formatage des sources touchées et `git diff --check` : PASS. Les trois tests
  config sont aussi rejoués après les dernières corrections de test/rustdoc.

### Limites et clôture

Aucune exécution CI Linux/cross-compilation, fixture TLS avec CA/non-confiance,
test réseau de timeout/chunked sans Content-Length, coffre réel, E2E GitHub ou
qualification de déploiement. Les bornes de transport sont implémentées et
revues statiquement ; les cinq tests SDK ne constituent pas une matrice exhaustive.
Les tests HTTP sont in-process WireMock, sans infra protocolaire/testcontainers.
Aucun runtime Docker/Kind/WSL ni conteneur créé, démarré ou adopté : pas de ligne
de registre Docker à produire, pas de ressource possédée à arrêter.

Préexistant AIForAll conservé en `b19b4ad`. Le delta de composition service a
également reçu un PASS sécurité **statique borné**, sans finding concret, sans
exécution indépendante de tests par le reviewer.

Service, contrats, tests et gitlink publiés dans
`21caa8bd774d94c280b578a97e1992649afa58b7` sur **origin/master** ; référence
distante vérifiée après push. Le gitlink reste de mode160000 et consomme
`93b890ff7b821e9cb2e7d0aeb81957d098778a2f`, publié/vérifié sur **origin/main**
du SDK. Les commits de clôture documentaire ultérieurs ne modifient pas les
sources testées. La fiche de mission utilisateur originale reste inchangée.

Clôture : checkout principal et sous-module propres après livraison ; aucune
branche/worktree de mission à intégrer/retirer, aucun archivage Ensemble requis.
Les deux sous-agents ponctuels sont terminés. Inventaire final comparé à
l’inventaire initial : les quatre worktrees et branches d’autres missions sont
préservés ; seul le préexistant `nul` reste non suivi dans le SDK frère.
Aucune branche/worktree de mission ni équipe Ensemble créée ; les arbres
étrangers sont conservés. Le fichier jetable préexistant `nul` du SDK frère est
exclu du commit et laissé en place, sans suppression non autorisée.
