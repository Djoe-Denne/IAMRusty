# Raccord IAM → OAuthConnect v2

## Mandat et périmètre

**Résultats finaux du périmètre local : [325 unités et80 IT passantes](20261108-iam-oauthconnect-v2-results.md).** Les états intermédiaires ci-dessous conservent la chronologie ; le rapport de résultats distingue les preuves finales, les deux incidents de préparation/environnement, les gates externes et le nettoyage restant sous autorisation.

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

## Progression vérifiée — raccord encore incomplet

### IT réelles — état intermédiaire

Sur le contenu intégré (`79105b8`, puis correction de préimage test seule `b28a922`) : **39 IT passent**, sans ignoré/filtré : transactions Postgres9, navigateur17, cleanup2 et START11. Les témoins de purge avant I/O, concurrence, upgrade réel, T-1 email et Q1 collision RELINK sont effectivement exécutés. Le premier test d'upgrade avait un INSERT de préimage sans UUID obligatoire ; correction de montage, mêmes oracles, puis9PASS. Aucun changement de production pour ce prérequis.

La première invocation callback a échoué avant assertions : démarrage Postgres opaque, puis collisions au port déclaré54070. Le lanceur a conservé le conteneur Engine `5f5d1b72938b8346ad5c234e4f1ddecab3ad58e04de0fcea817576fae36881e0`, sans l'adopter faute d'ID retourné par la fixture. Ces17échecs de setup **ne sont pas des rouges fonctionnels IAM**. Réessai unique demandé explicitement par l'utilisateur, sans modification du harness/SDK ou nettoyage du conteneur inconnu. Résultats restants à compléter ; aucun PASS global annoncé.

- **Transport v2** : tests compilables d'abord,3PASS/8RED sur les11nouveaux cas avant implémentation ; puis **22PASS** (11v2+11v1,124filtrés), dont HMAC sur PATH/corps réels, nonce/PKCE, borne1MiB inclusive, redirections et timeout10s avec une seule requête. Lecture bornée avant parsing, connexion3s ; aucune preuve RSS/gzip/streaming sans longueur revendiquée.
- **Domaine et capacité** :22nouveaux cas donnaient9PASS/13RED (BEGIN sans nonce et capacité legacy indûment accordée), puis passent avec tirageOsRng indépendant. Huit preuves supplémentaires couvrent forwarding des secrets originaux après consommation, erreur v2 sans fallback/retry et zéro appel sans capacité. **83tests domaine passent**, sans ignorés/filtrés ; Clippy all-targets strict passe.
- **Writer et migration** : unité SQL3PASS/1RED avant changement, puis **4PASS** (142filtrés). Colonne nullable additive, chaîne Migrator enregistrée, INSERT du nonce et double purge commitée implémentés ; `down` préserve la colonne et `up` est réapplicable. Tests Postgres/upgrade compilés, **non encore exécutés**. Clippy strict domaine+migration passe. Aucun SQL appliqué à un environnement de déploiement.
- **HTTPS** : fixture existante enrichie, nominal v1 conservé, nominal v2 et refus CA/SAN/redirection compilés ; exécution finale à venir.

Les orchestrations LOGIN/LINK/RELINK utilisent désormais authorize/complete v2, sans helper d'échange v1 ni appel de profil autonome dans leurs parcours. Leurs témoins ont atteint le RED avant correction (4LOGIN ;6LINK/RELINK), puis **87tests domaine et30application passent**, sans ignorés/filtrés. Les gardes de contexte et les blocs métier sont conservés. Clippy strict domaine+application+migration passe ; onze binaires IT compilent. Leurs exécutions réelles restent à faire.

Clippy strict `iam-infra --all-targets` reste rouge sur neuf diagnostics `dead_code` préexistants/non modifiés ; aucun allow ou masquage. Les preuves ci-dessus restent **partielles** : IT réelles et revues finales non terminées, aucun PASS global annoncé. La revue sécurité producteurs/LOGIN n'a trouvé aucun finding ; son verdict statique doit encore être complété pour le delta final LINK/retrait helper.

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

L'utilisateur a ensuite explicitement approuvé l'ajout de **`request_trace_redaction`**, onzième binaire existant directement adapté aux endpoints v2, pour vérifier la non-fuite dans les traces. Même lanceur gardé, aucun nouveau harness/changement de nettoyage ; sélectionPS d'abordREDavantajout, puisGREENetthreadsregressionGREEN. Ce test compile ; son exécution reste à faire avec les autres IT.
