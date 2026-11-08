# Raccord IAM → OAuthConnect v2

## Mandat et périmètre

Raccorder LOGIN, LINK et RELINK aux opérations v2 du service OAuthConnect déjà livré, sans bascule de déploiement. Conserver UUID, comptes, identités externes et liens existants. Réutiliser RustyCog et son pin ; aucun second harness E2E fournisseur.

Base : `e8952d805e81e49a940ef6d999bb74776f1d3166`. Cible : ADR0413 Accepted/Partial et [contrat §§5–7](../contracts/oauthconnect-multi-instances.md#5-adaptation-iam-et-contexte-oidc). **Travail en cours, aucune preuve IAM nouvelle de compilation/test à ce stade.**

## Vérification convenue

Exécution directe, sans mission Ensemble ni PM. TDD RED→GREEN et QA proportionnée : large couverture des parcours, vrais tests Postgres par fixtures RustyCog, HTTP sortant simulé, revues ciblées. Compilation et tests natifs Windows, un seul Cargo à la fois, 24 jobs.

Les quelques cas rares/ambigus encore rouges seront présentés **en un bilan fonctionnel groupé** avant toute décision de suppression ou réparation élargissant le comportement. Échec d'environnement, test absent ou compilation impossible ne valent pas RED comportemental ni PASS.

## Invariants et limites

Nonce IAM indépendant et durable avant START ; writer consomme puis efface nonce/verifier atomiquement avant tout complete. Migration additive nullable, refus des anciennes transactions sans nonce. Pas de fallback v1, profil dérivé du seul access token ou effet IAM lors d'un échec OIDC.

E2E GitHub/GitLab réels et déploiement restent à l'autre propriétaire. L'équivalence exacte `sub` GitLab/ancien ID reste une condition externe de bascule ; aucune migration de comptes ou réassociation implicite n'est autorisée.

## État vérifié de départ

`cargo test --locked -p iam-domain --lib` : **53 PASS, 0 échec, 0 ignoré, 0 filtré** sur la base du mandat, avant changement applicatif. Ce résultat caractérise l'existant ; il ne prouve pas le raccord v2. Aucun runtime Docker n'a été démarré.

## Livraison par dépendances

1. Intégrer le cadrage des coutures IAM et le [plan QA](../../.cursor/qa-plans/20261108-iam-oauthconnect-v2.md) avant leurs auteurs.
2. Préparer des témoins RED compilables : adapter HTTP v2 et capacité transactionnelle indépendants. Une nouvelle API manquante nécessite un socle compilable, pas un faux RED de compilation.
3. Intégrer nonce/migration/writer et transport avant leurs consommateurs LOGIN/LINK/RELINK, navigateur et fixtures ; vérifier les compteurs v2 et l'absence de fallback.
4. Exécuter les IT réelles sur contenu intégré avec les fixtures et leur garde de ressources existantes. Vérifier les régressions IAM et compilation du monolithe, sans l'exécuter.
5. Revues indépendantes ciblées selon le diff, bilan groupé des rouges rares éventuels, publication et nettoyage. Pas de verdict global à partir des seuls PASS partiels.

## Choix de campagne IT à approuver

Le lanceur gardé existant `ops/tests/it-native.ps1` sélectionne seulement trois binaires IAM : transactions Postgres, navigateur et cleanup. Les autres régressions utiles au raccord (start/callback, refus fournisseur, relink, HTTPS vérifié, garde de comptes et supervision cleanup) ne sont pas sélectionnables individuellement par ce lanceur.

La suite IAM entière n'est pas une alternative neutre : `tests/sqs_event_routing_tests.rs` active explicitement SQS, n'est pas ignorée et sort du périmètre OAuth. L'utilisateur a **approuvé l'extension bornée de l'allowlist existante** à `auth_oauth_start`, `auth_oauth_callback`, `auth_oauth_provider_failures`, `relink_provider`, `verified_idp_https`, `organization_account_guard` et `app_cleanup_supervision`. Les gardes de propriété/registre/cleanup restent inchangées, sans nouveau harness ni activation des queues.

Les tests PowerShell/AST de sélection ont échoué avant l'extension puis passent après ; la régression de sélection des threads passe aussi. Aucun Cargo, conteneur ou runtime n'est lancé par ces deux vérifications. Les IT IAM restent à exécuter sur le contenu intégré.
