# Ops OAuthConnect — GitHub et fournisseur simulé

## Mandat et choix retenus

Adapter les ops existantes au service OAuthConnect sans contourner la configuration
ni la distribution des secrets. GitHub réutilise ses identifiants et références
KVv2 existants ; la seconde instance, choisie par l'utilisateur, est `simulated`
avec le profil OAuth, pas un fournisseur OIDC signé à implémenter maintenant.

L'utilisateur a préféré une entrée navigateur **HTTPS** à une exception de
validation des callbacks HTTP. Le profil nouveau utilise donc
`https://127.0.0.1:18080` et le certificat Envoy existant avec SAN IP. Il n'ajoute
aucune exception dans le validateur Rust ni désactivation TLS pour GitHub.

## Contenu livré

- Profil explicite `--oauth-connect` dans plan/build, consommation des images,
  rendu et déploiement. Cible image `oauth`, Dockerfile runtime sans migrations,
  utilisateur1000, base Debian épinglée ; octets SDK/source et références
  immuables vérifiés par la chaîne d'images existante.
- Un seul Deployment OAuthConnect, catalogue public de deux instances OAuth,
  registry IAM v2 avec PATH propre à chaque instance et HMAC distinct. Les profils
  legacy restent disponibles et incompatibles avec le nouveau flag ; aucun
  retrait de service ou cutover d'un environnement existant n'est réalisé.
- TLS navigateur sans certificat client, mTLS upstream préservé, callbacks HTTPS,
  origine/cookie IAM et issuer des consommateurs alignés. Seule la route exacte
  GET/HEAD `simulated/relink-callback` rejoint les exceptions déjà bornées.
- Extension PKI additive `openbao-oauthconnect`, sous la CA transport existante,
  sans modification des anciennes feuilles/clés. Extension incomplète refusée.
- GitHub : vérification TLS conservée, scope `user:email`, mapping `/user` +
  `/user/emails` sélectionnant l'email primaire, sans fallback vers l'email public.
- Simulateur réutilisé sous le slug `simulated`, callbacks HTTPS et PKCE S256,
  endpoint profil `/user` avec `email_verified` booléen. Le mode historique GitLab
  conserve ses callbacks HTTP et son endpoint `/api/v4/user`.
- Politiques réseau explicites IAM→Connect, Connect→simulateur et HTTPS vendor,
  accès OpenBao et DNS sous default-deny. Aucun nouveau hop Connect→IAM/JWKS,
  DB, OpenFGA, événement ou queue.

## Secrets : propriété et cycle

GitHub conserve `secret/github-connect/oauth` et `secret/github-connect/iam`.
Les credentials simulés sont dédiés à `secret/oauth-connect/simulated/oauth` et
`secret/oauth-connect/simulated/iam`. L'opérateur fournit les deux fichiers privés
en lecture seule via leurs chemins ; aucune valeur n'est passée en argument.

Le seed non-root vérifie toutes les préimages avant d'écrire : réutilisation
exacte, création CAS0, refus d'un conflit/d'une suppression, sans écrasement ni
rollback destructif d'un résultat partiel. Les droits sont séparés : Connect lit
les quatre documents, IAM lit seulement les HMAC, le simulateur seulement son
secret OAuth. Les privilèges initiaux restent dans le bootstrap du coffre local.

L'Agent Connect fournit le token-file à partir d'une identité Kubernetes projetée
avec audience dédiée. Les secrets applicatifs sont résolus au boot ; les
snapshots IAM/simulateur et le token-file sont en volumes Memory. Le Secret
d'entrée éphémère du seed est lié au Job par UID/TTL : ce n'est pas une garantie
de mémoire seule dans etcd. Aucune lecture de Secret Kubernetes live.

`local-up` enregistre les deux références privées inchangées. Une nouvelle epoch
du coffre exige ces sources, revalide les identités/configurations, seed puis
redémarre uniquement Lazaret, IAM, OAuthConnect et le simulateur, sous tests
UID/resourceVersion. Une source changée exige une rotation explicite.

## Vérifications et limites

Base de travail : `8575620c6ab3bf1bf37156484a7b49d5f5084026`, `master` propre avant
mission ; un checkout, aucune branche/worktree créé. SDK inchangé :
`93b890ff7b821e9cb2e7d0aeb81957d098778a2f`.

- Suite Python ops initiale complète : 215 tests, 213 réussites, 2 ignorés, 0 échec.
  Les ignorés sont les preuves opt-in de cache Cargo et de registre Docker,
  pas des preuves live. Après ajout des deux oracles de reprise/CAS :
  18/18 OAuthConnect et 19/19 OpenBao
  exécutés, dont PKI native synthétique étendue et refus d'extension partielle.
  Les autres sources de la suite complète sont inchangées.
- Configuration Rust complète : 25 déclaratifs + 1 ops + 13 loader verts. Le premier
  test ops rouge sur les callbacks HTTP a précédé le passage HTTPS. Un wrapper
  PowerShell a rendu code1 malgré les sorties libtest vertes ; ce statut n'a pas
  été présenté comme succès de commande.
- Commande native finale via `cmd /c` : 3/3 tests Rust ops (configuration réelle,
  mapping GitHub/email primaire/absence/ambiguïté, mapping simulé/type booléen).
- Clippy `-D warnings --all-targets` et fmt ciblés sur les deux crates verts.
  Avertissement SDK préexistant `async_fn_in_trait` observé sans modification du
  sous-module ni suppression de lint.
- Logs privés dans `.tmp/ops-oauthconnect/`, hors Git. Revue infra indépendante :
  **PASS à portée statique/offline**, aucun blocage détecté. Le live demeure
  INCONCLUSIVE. Observation faible conservée : la copie de la route relink
  GitHub suppose sa présence dans la source Envoy (présence actuelle vérifiée et
  testée) ; sa disparition future produirait un refus `StopIteration`, pas une
  exception de validation dédiée. Aucun élargissement de route en cas d'échec.
  Le port HTTP interne 8080 reste lié par le SDK mais n'est pas exposé par le
  Service/les nouvelles policies ; les certificats suivent la durée locale
  existante de 90 jours. Rotation et validation live restent hors périmètre.

## Publication et nettoyage

Implémentation publiée : `f78108bec6288b52517abb51546d8a044001fc62` sur
`origin/master` ; push réussi et égalité HEAD / référence de suivi / véritable
`refs/heads/master` distante vérifiée par `git ls-remote`. Le présent amendement
documentaire clôture le rapport sans modifier les sources testées.

Inventaire de clôture : seul `master`, unique checkout principal propre après
publication de l'implémentation, sous-module SDK inchangé. Aucune branche ou
worktree créé pour cette mission, aucune équipe Ensemble créée. Le reviewer
read-only a terminé ; aucun runtime possédé à arrêter, aucune suppression ni
prune effectué.

**Non exécutés** : build image Linux, déploiement local, coffre/Agent réels,
qualification navigateur, confiance CA système, connexion GitHub réelle, E2E
simulé ou chaîne intégrée IAM→Connect→vendor. Aucun secret opérateur lu, aucune
application GitHub ou URL enregistrée modifiée. Le callback HTTPS doit être
vérifié dans l'enregistrement GitHub existant avant activation.

Le simulateur reste un fournisseur de test local HTTP : seul ce profil porte
`isolated_test`. Son futur parcours navigateur exige l'ouverture explicite du
tunnel ancré loopback18081 ; le déploiement ne l'ouvre pas. Les anciens outils
E2E supposant HTTP/GitLab ne sont pas automatiquement qualifiés pour ce profil.
ADR0413 demeure Accepted/Partial ; les gates GitLab/live/cutover antérieurs restent
inchangés. Aucun Docker/Kind/WSL démarré, aucune ressource Docker créée ou adoptée,
aucun cleanup runtime applicable ; le modèle8Brésident n'a pas été arrêté.

## Sources

- [Instructions opérateur](../../ops/deploy/README.md#oauthconnect--github--fournisseur-simulé-entrée-https).
- [ADR0413](../adr/0413-oauthconnect-multi-instances-profils.md) et contrats
  [Connect](../contracts/oauthconnect-multi-instances.md),
  [raccord IAM](../contracts/iam-oauthconnect-v2.md).
- Documentation OpenBao2.6.x vérifiée :
  <https://openbao.org/docs/2.6.x/agent-and-proxy/autoauth/methods/kubernetes>.
- Base Docker : digest du manifest list `debian:bookworm-slim` résolu depuis le
  registre officiel Docker Hub, sans pull/build ; digest inscrit au Dockerfile.
