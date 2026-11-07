# OAuthConnect — prochain lot TDD : configuration et projection pure

## Statut et résultat attendu

**Exécution autorisée par le GO utilisateur du 2026-11-07.** Vérification proportionnée et périmètre ci-dessous retenus à partir du plan présenté ; aucune mission Ensemble ni nouveau travail PM. Le GO n'étend pas le lot aux transports, à IAM, aux déploiements ou aux E2E. Chronologie et preuves d'exécution : `.cursor/handoffs/20261107-oauthconnect-config-mapping.md` ; tant que ces preuves n'existent pas, aucun résultat RED/GREEN n'est revendiqué.

Base vérifiée le 2026-11-07 : `master@ddee62fbfd132424eb20afeeb56d1194b9dbe85e`, arbre propre, checkout canonique seul, SDK rustycog `93b890ff7b821e9cb2e7d0aeb81957d098778a2f` inchangé. La tranche précédente de contrat S2S v2 est publiée ; ses 16 tests v2 et 15 tests lib, checkdefault, fmt et Clippy sont les preuves antérieures, pas des exécutions de ce plan.

Objectif : accepter une configuration OAuthConnect strictement typée et projeter des réponses JSON déjà obtenues en `ProviderUserProfile`, selon les deux profils et les règles ratifiées. Aucun endpoint fournisseur appelé par ce lot. Un profil OIDC peut être décrit et validé déclarativement sans prétendre en vérifier les tokens.

Canon : ADR0413 **Accepted / Partial**, `docs/contracts/oauthconnect-multi-instances.md` §§2.1–2.3, 4, 6–8 ; `.cursor/qa-plans/20261106-oauthconnect-tdd.md` T0, T1/G-1 et T2/G-2. ADR0200–0202 et contrats RustyCog restent applicables. Ce document organise l'exécution, ne modifie aucune décision d'architecture.

## Périmètre et exclusions

Livrables attendus : packages minimaux nécessaires sous `services/OAuthConnect/`, configuration réelle via le loader SDK existant, validation déclarative, validation JSONPath par l'AST de la bibliothèque retenue et projection pure testée. Réutiliser le contrat partagé pour les profils normalisés et les mécanismes RustyCog d'erreur/secrets ; pas de loader ou framework concurrent.

Pas dans ce lot : client token/profil HTTP, résolution OpenBao effective, Discovery/JWKS/UserInfo, validation ID token, registry complet au boot/listener du service, nonce transactionnel ou raccord IAM, Compose/TLS/Kind, bascule ou suppression GitHubConnect/GitLabConnect. Pas d'E2E écrit/exécuté : autre intervenant propriétaire. Pas de bump/modification SDK.

**Limite de couverture explicite :** T1/G-1 n'est fermé que pour sa partie déclarative/loader. Secrets résolus, Discovery et publication atomique du registry avant écoute restent des dépendances des lots HTTP/OIDC/setup. Le lot ne sera pas annoncé « service opérationnel » ou « boot complet validé ».

## Ordre de travail et barrières

### 0 — préconditions compilables et interfaces minimales

- Confirmer dans les manifests les packages nécessaires parmi `oauth-connect-configuration`, `oauth-connect-domain`, `oauth-connect-infra` ; ne pas générer les six crates, binaire et setup vides par simple gabarit.
- Fixer les types partagés strictement nécessaires à la configuration et à la projection, dans les frontières du contrat. Placement final et signatures locales au propriétaire ; si un arbitrage impose de déplacer une frontière ou de changer le contrat, consulter `architecte` avant implémentation.
- Vérifier techniquement `jsonpath-rust = "=1.0.12"` : AST disponible, filtres autorisés et chemins localisés. Réutiliser les sources primaires versionnées du contrat ; confirmer les types réellement compilés, sans second parser ni remplacement silencieux de bibliothèque.
- Examiner le vrai loader strict déjà employé côté GitHub et les syntaxes de tableaux/overrides qu'il expose. Ne pas présumer une notation d'index env.
- Créer seulement le socle permettant aux tests de compiler. Un retour provisoire refusant peut donner un vrai RED nominal ; ni un package absent ni une erreur de compilation ne sont un RED comportemental. Aucun comportement fonctionnel caché dans le socle.
- Propriétaire unique du workspace `Cargo.toml`/`Cargo.lock` : parent intégrateur. Intégrer, vérifier et versionner les types/API avant de créer les consommateurs isolés.

### A — configuration stricte, sans réseau (T1 déclaratif)

Ordre : `test-engineer` écrit les oracles du vrai loader → intégration et RED exécuté → `implementer` réalise le GREEN → régression ciblée et intégration.

Critères :
- Deux types de profil seulement ; plusieurs instances, dont deux du même type et un slug arbitraire compatible. Instance logique distincte du profil, sans branche vendor.
- Préfixe `OAUTH_CONNECT`, tableaux et overrides effectivement supportés prouvés par le loader SDK réel ; aucune substitution par un simple test de désérialisation.
- Catalogue vide/dupliqué, mauvais slug, clé/fichier/env inconnu dans les sources chargées, ancien préfixe chargé accidentellement, champs d'autre profil et enum inconnue : refus explicite. Respecter la politique SDK existante sur l'environnement ambiant, sans la modifier globalement.
- Scopes, URLs, paramètres OAuth réservés, transport, méthode d'auth token, S256 OIDC et formes direct/OpenBao : validation des obligations du contrat. Modes/références requis et exclusifs, Debug/erreurs redacted ; aucun secret réel ni lecture réseau.
- APIs référencées/exploitées et structure des mappings cohérentes. La validation AST des expressions et les contrôles runtime de projection appartiennent au lot B ; ne pas prétendre les couvrir par la seule désérialisation.

### B — JSONPath borné et projection pure (T2)

Ordre : tests indépendants sur les types intégrés → RED exécuté → GREEN de projection → intégration et vérification avec A.

Critères :
- AST de la bibliothèque maintenue : enfant/index/filtres scalaires-logiques admis ; récursion, regex, fonctions/extensions refusées. Témoins aux bornes 512 octets/16 niveaux, refus au-delà. Pas de DSL ni de parser de remplacement.
- `sub` : une chaîne non vide préservée ou entier JSON non négatif jusqu'à `u64::MAX`, conversion exacte ; absence/null, flottant, négatif, dépassement ou cardinalité multiple refusés.
- Autres claims : zéro/un résultat, null optionnel admis, mauvais type et plusieurs résultats refusés même égaux. Email absent implique verified=false ; aucun booléen fabriqué ni email synthétique.
- Email et verified : même définition API et même objet sélectionné, vérifié avec les chemins localisés ; objets différents de valeurs identiques ne rendent pas le couple valide. Incohérence structurelle rejetée à la validation, incohérence JSON rejetée à la projection.
- GitHub : primaire seul ; pas de primaire donne email absent même si `/user.email` est rempli ; deux primaires donnent une erreur. Username absent devient chaîne vide ; picture optionnel demeure descriptif et n'est jamais téléchargé.
- La projection reçoit les réponses JSON en mémoire. Le comptage « un GET par API et par complétion », les plafonds réseau et l'isolation de cache ne sont pas prouvés ici : lot transport suivant.

### Parallélisme retenu

Après intégration du socle et des types, A et B peuvent avoir des auteurs isolés sur des chemins disjoints : configuration d'un côté, projection/validation AST de l'autre. Ne pas faire dépendre B d'un fichier en cours d'écriture dans A : fournir des types et fixtures déjà intégrés. Les contrôles combinés attendent l'intégration des deux branches.

Chaque branche conserve son cycle RED avant GREEN. Le même auteur de tests poursuit les corrections de ses tests ; l'auteur production ne change pas leurs attentes. Aucun worker n'écrit le workspace/lock en concurrence. Les commandes Cargo restent sérialisées dans le checkout canonique Windows, même si les investigations/éditions isolées avancent en parallèle.

## Vérification proposée : proportionnée

- Réutiliser les preuves précédentes tant que leurs sources ne changent pas. En fin de lot, contrôler le raccord des nouveaux packages sans transformer une régression non exécutée en PASS.
- Archiver pour chaque sous-tranche : base et diff, cas découverts, commande, compteurs et assertion RED, puis mêmes oracles GREEN. Une caractérisation déjà verte reste une caractérisation.
- Tests de loader/env : utiliser les moyens d'isolation existants ; pas de mutations concurrentes de variables globales ni sleeps arbitraires.
- Revue indépendante bornée après intégration, sur les risques changés : sécurité de la configuration/validation AST et authenticité de la paire email/verified ; inclure la qualité des oracles si un doute subsiste, sans imposer une chaîne de quatre reviewers. Briefing persisté avant correction.
- `architecte` seulement si une frontière reste ouverte, pas pour refaire les décisions acceptées. Pas de PM ni nouveau QA planner systématique : le plan QA fournit déjà les oracles de ce lot.

Commandes candidates, **non exécutées**, après création et confirmation des packages/targets :

```powershell
cargo check --locked -p oauth-connect-configuration -p oauth-connect-domain -p oauth-connect-infra
cargo test --locked -p oauth-connect-configuration --lib
cargo test --locked -p oauth-connect-infra --lib
cargo fmt -p oauth-connect-configuration -p oauth-connect-domain -p oauth-connect-infra -- --check
cargo clippy --locked -p oauth-connect-configuration -p oauth-connect-domain -p oauth-connect-infra --all-targets -- -W clippy::future_not_send -W clippy::too_many_lines -D warnings
```

Adapter aux vrais targets du socle : ajouter les tests d'intégration du loader s'ils résident sous `tests/`, et tests domaine seulement si du comportement y est introduit. Ne pas laisser `--lib` masquer une suite créée ailleurs. Une commande/feature qui découvre zéro cas ne prouve rien. Actualiser minimalement Cargo.lock avant les commandes `--locked`, sans upgrades connexes.

Un Cargo à la fois, jobs=24 et target E: existants, sans override. Aucun Docker/Kind nécessaire pour la configuration et les réponses JSON en mémoire. Avant tout run réel, charger lifecycle/test-evidence et vérifier les effets des fixtures ; si une preuve requiert soudain un nouveau harness/runtime, arbitrer ce changement de périmètre avant lancement.

## Critère de sortie et suite

Le lot est terminé après tests RED/GREEN démontrés, validations pertinentes sur le contenu intégré, revue bornée traitée, docs/registre de gaps actualisés, commit/push vérifiés et retrait des arbres temporaires propres. La réalité ADR0413 reste **Partial**.

Suite ordonnée : transport OAuth et résolution secrets/setup avec collaborateurs HTTP WireMock → profil OIDC/Discovery/JWKS/UserInfo réels → migration additive nonce IAM et raccord LOGIN/LINK/RELINK → composition et préparation déploiement → bascule/retrait uniquement après preuves E2E externes et rollback prêt. L'allowlist exacte callback/relink doit être exercée à la composition service, jamais présumée fournie par le contrat partagé.

## Preuves de préparation et limites

### Point d'exécution intermédiaire — non clôturé

Configuration : 38 tests passés du socle refusant (23 réussites/15 échecs nominaux) à38/38. Projection :41oracles originaux passés de18/23 à41/41 après correction d'une permissivité du parser ;3caractérisations post-GREEN des chaînes quotées/échappements également vertes (44cas au total). Aucun ignoré/filtré dans les runs complets. La distinction entre TDD initial et caractérisations supplémentaires est conservée.

Le lot reste **ouvert** : qualitéClippy finale des tests et disposition du finding sécurité **S-1 MEDIUM** (coût amplifié par filtres absolus imbriqués sur JSON fournisseur hostile). Aucun HIGH/CRITICAL relevé, mais le parent cadre une borne pré-évaluation avant clôture. Aucune validation de boot complet, réseau, secret résolu, OIDC/IAM ou E2E n'en découle. Cette photographie n'est pas un bilan final.

État Git/inventaire vérifiés et contrat/QA relus pour cette préparation. Aucune nouvelle exploration de comportement Rust nécessaire : chemins et obligations réutilisés des artefacts intégrés. Les symboles du loader et la surface AST seront examinés par MCP avant les changements concernés, avec références et couverture des tests ; aucune conclusion nouvelle sur ces APIs n'est prétendue ici. Aucun test, build, fournisseur ou runtime exécuté pour préparer ce plan.
