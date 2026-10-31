# Plan natif retenu — sécurité de l'authentification

## État et provenance

**Actualisation de délégation :** l'utilisateur a demandé de donner les autorisations à l'avance. Le [rapport de préautorisation](reports/06-preautorisations-long-run.md) et le brouillon corrigé priment sur les mentions d'autorisations à redemander ci-dessous. La ratification reste une opération explicite, désormais déléguée au lead après le travail d'architecte, dans les trois frontières déjà planifiées et sans contradiction d'ADR Accepted. Publication SDK bornée et opérations du laboratoire full sont préautorisées sous leurs conditions de sûreté ; ni garde-fous ni revue/intégration ne disparaissent. Cette révision attend sa présentation et son approbation native par start ; aucun worker n'est démarré par cette correction.

Préparation revue et import natif contrôlé : quatre racines pending, dix lots consommateurs conservés dans l'annexe documentaire et non importés. Aucun worker, test ou runtime démarré. Aucune approbation d'exécution déduite de l'import ; le start utilisateur reste nécessaire après présentation de la politique courante.

Source complète : session PM `ses_e7c953b4dffeSr2aLcciLsCZxQ`, document ready `msg_183a6f62b001DcW1GQl6qNN467`, version 2, aucune raison bloquante ou information manquante, catalogue vérifié, aucune question ouverte. Le document conserve les fiches détaillées des dix consommateurs dans son contexte/annexe. Ce fichier en donne le graphe opératoire et les invariants ; les fiches importées portent leurs critères et sources complets sur le tableau natif.

Périmètre : corriger et qualifier l'authentification existante ; Ensemble reste l'outil de coordination, pas un développement de plugin. Base de découverte : application `c72ae362b950c63d6736a6818c68babc5a8d6f40`, SDK `d60a88b2f2f499768a807941a1b4c80c9a366b3f`. Chaque auteur recevra le HEAD/diff intégré réel, pas l'hypothèse que cette base est encore courante.

Politique proposée inchangée depuis la clarification : OpenAI uniquement, Sol et ses quatre variantes, seuil de départ 50, classification désactivée ; long_run non actif avant start. QA proportionate, quatorze commandes de référence et douze critères, à sélectionner selon diff et provenance sans campagne exhaustive automatique.

## Règles d'attribution et d'intégration

- Contextes fresh et briefs ciblés avec sources, contrats et Statut ADR. Auteurs isolés ; un propriétaire par fichier mutable. Reviewers fresh, lecture seule, sans verdict d'auteur dans le briefing.
- Le lead réserve les numéros ADR et détient index ADR, gitlink, intégrations, ledger et briefings. Les contrats nouveaux restent Proposed jusqu'à ratification, puis sont intégrés avant publication des consommateurs.
- Seules les quatre racines ci-dessous sont publiées. Résolution de dépendance et fin d'auteur ne publient pas un consommateur : le lead inspecte/intègre, vérifie les entrées disponibles et publie ensuite avec les vrais IDs et le HEAD intégré.
- Les auteurs des lots de développement gardent régression ROUGE, correction et refactoring dans la même tranche. Un défaut produit trouvé par la QA est renvoyé au propriétaire ; test-engineer n'écrit pas de correction de production.
- Un seul Cargo Windows, 24 jobs sans override, cible E:, arbre compilé gelé. Fixtures IT réelles existantes, mocks pour HTTP sortant et queues opt-in selon ADR 0200–0202. Ne pas confondre erreur d'environnement et ROUGE fonctionnel.
- SDK publié seulement après autorisation distincte, commit exact avant gitlink. Kind seulement après corrections/IT finales et autorisation ciblée ; inventaire, baux et registre contemporains. Aucun runtime pour les trois décisions ou les contrôles statiques TLS.
- Les estimations du PM restent indicatives et non mesurées ; ni devis ni engagement de délai. Pas de scores ou modèles injectés aux tâches pour contourner le catalogue.

## Racines importées — exécution encore non approuvée

### 1. Séparation des autorités jusqu'aux permissions — arch-identites

**ID réel :** `task_mvsf2g5q_0003_qqrmnr`. **Profil :** architecte. **Dépendance :** aucune. **Type :** architecture, planning requis, worktree isolé.

**Entrées :** ADR 0305 Accepted / Implemented sur son périmètre, schéma FGA multi-émetteur non décidé ; `with_principal`, `permission_middleware`, `Subject`, route signer Hive et projection `workers/sentinel-sync/src/translator/hive.rs`.

**Livrable / ownership :** contrat proposé `docs/contracts/authz-trust-domain-boundary.md`, ADR forward si nécessaire et numéro réservé par le lead. Pas de production, modèle FGA ou migration dans ce lot.

**Acceptation :** comparer conservation de l'autorité et refus explicite des identités incompatibles sans bannir globalement les identités d'organisation ; définir représentation, membres/tuples/projections, droits existants, erreurs, éventuelle migration et compatibilité standalone/monolithe. Allowlist du consommateur et régression avec deux JWT reconnus, même sub/iss distincts, témoin légitime et refus avant RPC/mutation. Ratification puis intégration avant correction.

**Vérification :** références, projections et tests aux frontières utiles ; aucun zéro de graphe pris comme preuve d'absence. Vérification documentaire sans Cargo/runtime.

### 2. Exigences sécurisées du signataire distant — arch-signataire

**ID réel :** `task_mvsf2g5q_0004_ft9ns7`. **Profil :** architecte. **Dépendance :** aucune. **Type :** architecture, planning requis, worktree isolé.

**Entrées :** ADR 0309 Accepted / Partial et 0311 Accepted / Implemented ; configuration remote, constructeur et tests existants. Le timeout de dix secondes existe déjà.

**Livrable / ownership :** contrat proposé `docs/contracts/remote-signer-secure-transport.md`, ADR si nécessaire. Aucun code produit, fournisseur HSM/KMIP ou nouveau cloud.

**Acceptation :** durcir le remote supporté ou interdire explicitement son activation sécurisée tant que non qualifié, sans désactivation silencieuse ; fixer URL, exceptions locales, redirections, bornes succès/erreur, délais/confidentialité. Préserver Sign/GetPublicKey digest-only ; refus sans repli et configurations/algorithmes admis explicites. Ratification/intégration avant consommateur.

**Vérification :** comparer validation remote/Transit, lectures et tests ; documentation officielle des versions reqwest/TLS résolues si nécessaire. Aucun changement de dépendance par principe ni runtime.

### 3. Contrat de preuve navigateur — arch-navigateur

**ID réel :** `task_mvsf2g5q_0005_ximvvi`. **Profil :** architecte. **Dépendance :** aucune. **Type :** architecture, planning requis, worktree isolé.

**Entrées :** ADR 0412 Accepted / Partial, contrat exact-route et rendu historique de 112 objets, non assimilé à une preuve réseau. 0606 reste Proposed.

**Livrable / ownership :** proposition ciblée d'amendement de `docs/contracts/0412-oauth-exact-route-envoy-proof.md`, contrat complémentaire si nécessaire. Aucun code du laboratoire.

**Acceptation :** résoudre la contradiction HTTP/TLS, fixer entrée, origines, cookies, certificats et redirects, en distinguant les hops internes. Navigateur réel, connecteur supporté activé et fournisseur reproductible sans credentials personnels privilégié. Matchers exacts préservés ; pas de preuve avec connectors=[] ni qualification production implicite. Amendements à ratifier et allowlist avant consommateur.

**Vérification :** contrat/rendu/Envoy recoupés ; `browser_flow.rs` est reqwest, pas navigateur. Aucun main écrivain du renderer, cluster ou navigateur pendant la décision.

### 4. Identité des serveurs contactés par Envoy — corriger-envoy

**ID réel :** `task_mvsf2g5q_0006_29odow`. **Profil :** implementer. **Dépendance :** aucune. **Type :** implémentation, planning requis, worktree isolé.

**Entrées :** ADR 0308 Accepted / Partial ; CA et SNI sans matcher de nom observé. Identités sourcées dans `ops/scripts/generate-platform-mesh-certs.sh` et certificats publics prévus, pas un Secret vivant.

**Ownership :** `ops/deploy/mesh/envoy.yaml`, `ops/deploy/apps/overlays/kind-mesh/envoy-mesh.yaml`, tests amont ciblés sous `ops/tests/`. Pas de listener navigateur ; renderer modifié seulement si nécessaire après attribution lead.

**Acceptation :** matcher précis pour ext-authz, IAM, Hive, Telegraph et Manifesto ; cohérence sources/rendu sans élargir le trust. Filtres, exemptions et certificat client passerelle préservés. Régression statique ROUGE puis correction et scénario même CA/bon nom contre mauvais nom. Identité indécidable renvoyée à architecte, non inventée. Intégration avant transfert de ces fichiers à l'alignement navigateur.

**Vérification :** rendu en mémoire sans main écrivain ni Secrets imprimés, cinq identités contrôlées ; tests statiques nommés/exécutés avec commandes réelles. Scénario vrai Envoy livré mais run différé : rendu ou HTTP 5xx seul n'est pas une preuve de refus TLS.

## Dix lots conservés au dossier — non importés

### 5. Correction identité/permissions — corriger-identites

**Profil :** hard-implementer, planning requis, fresh, worktree isolé. **Entrée bloquante :** arch-identites ratifié et intégré.

**Ownership :** allowlist réduite par contrat : SDK auth/permission/tests, gardes et tests Hive, projections et modèles FGA si nécessaires ; consommateurs Manifesto/Telegraph seulement après références et attribution. Exclure remote/laboratoire. Aucun schéma inventé.

**Acceptation :** deux JWT reconnus atteignent le contrôle, témoin légitime autorisé et collision refusée avant RPC/mutation. Préserver refus fail-closed et voies standalone/mesh ; lecteurs et producteurs cohérents. Publication SDK distinctement autorisée si nécessaire.

**Vérifications :** suites `organization_signer_permissions`, `organization_account_guard` ; si SDK modifié, tests `permission_middleware`, `mesh_gateway_auth` et `http::` avec `--manifest-path rustycog/Cargo.toml --locked --features http`. Ajouter commandes des projections/migrations réellement touchées avant run. Hive/OpenFGA réels ; mock seulement RPC IAM sortant. Slot Cargo/fixtures et pin SDK détenus par lead.

### 6. Correction remote — corriger-signataire

**Profil :** implementer, planning requis, fresh, worktree isolé. **Entrée :** arch-signataire ratifié/intégré.

**Ownership :** configuration IAM, `infra/src/signing/remote.rs`, `infra/tests/remote_signer.rs`, setup strictement nécessaire. Pas de refonte Transit/setup.

**Acceptation :** refus avant réseau des configurations interdites, exceptions locales explicites ; zéro appel/secret vers redirection ; bornes succès/erreur même sans Content-Length ou segmentées ; délai borné. Digest-only, version, WorkloadIdentity, vérification cryptographique et absence de clé privée préservés. Pas de repli.

**Vérifications :** `cargo test --locked -p iam-configuration --lib`, `cargo test --locked -p iam-infra --test remote_signer`, Clippy ciblé IAM. Témoins positifs/TLS réel selon propriété, régressions rouges puis correction. Cargo et fixtures HTTP/TLS exclusifs, pas Kind.

### 7. Alignement laboratoire/scénario — aligner-navigateur

**Profil :** implementer, planning requis, fresh, worktree isolé. **Entrées :** arch-navigateur ratifié et corriger-envoy intégré ; transfert de propriété Envoy seulement après intégration.

**Ownership :** renderer, configuration IAM du profil full, overlays attribués et scénarios/tests `ops/tests/` ; pas de réimplémentation des transactions OAuth.

**Acceptation :** entrée, redirects, cookies/certificats conformes ; ni tolérance TLS globale ni exemption élargie. Connecteur supporté actif, fournisseur sans secret personnel non autorisé. Scénario exact et témoins livrés ; reqwest reste IT. Pas de nouveau framework/Kind implicite.

**Vérifications :** `python -B -m unittest discover -s ops/tests -p test_local_full_target.py`, puis `test_local_full_acceptance_source.py` ; rendu en mémoire sans main/Secrets imprimés. Nouveaux tests et actions réseau nommés avant acceptation, campagne réelle différée.

### 8. Qualification native — verification-native

**Profil :** test-engineer, planning requis, fresh, worktree isolé. **Entrées :** les quatre corrections intégrées, snapshot figé et pin résolu.

**Acceptation :** preuves exécutées/réutilisées, défauts préexistants, erreurs d'environnement et non-exécutés distingués. Nouvelles régressions et garanties affectées couvertes : émission autoritative, JWKS vide/fraîcheur 60 s, sessions atomiques, OAuth et confidentialité. Compilation seule insuffisante. Aucun correctif production par QA. Zéro conteneur IT possédé running en fin de batch et sur échec.

**Vérifications :** sélectionner les fmt/check/Clippy/tests du brouillon selon diff/provenance, pas toute la liste en bloc. Les suites OAuth sont rejouées lorsque le contrat l'exige après changement du parcours/matchers. Skills lifecycle/test-evidence, inventaires et cleanup selon canon. Windows/Cargo unique/24 jobs/E: ; jamais Kind pour les IT.

### 9. Revue sécurité — revue-securite

**Profil :** security-reviewer, lecture seule, fresh, worktree:false, final_response. **Entrée :** verification-native et snapshot figé, briefing factuel sans conclusions développeur.

**Acceptation / contrôle :** examiner collision, compatibilité des droits, contournements mesh, remote/redirections/corps et exceptions ; chemins/refus/effets sourcés. Distinguer blocages, suggestions et hors-périmètre ; aucune attaque exécutée revendiquée. Aucun défaut critique/élevé ouvert avant campagne finale. Aucun code/tests écrit ; corrections renvoyées aux propriétaires, sans Cargo/runtime supplémentaire par principe.

### 10. Revue des oracles — revue-oracles

**Profil :** test-reviewer, lecture seule, fresh, worktree:false, final_response. **Entrée :** verification-native ; lecture parallèle possible avec revue-securite.

**Acceptation / contrôle :** deux JWT reconnus, positif légitime, refus avant RPC/mutation ; zéro requête redirigée et corps sans taille annoncée ; Envoy mauvais nom distingué de panne par bon nom/même CA ; moteur navigateur et connecteur actif. Fixture absente/capture vide non assimilées à succès. Registre borné des gaps, sans réécriture ni pipeline général.

### 11. Préflight runtime — qualifier-runtime

**Profil :** test-engineer, planning requis, fresh, worktree isolé. **Entrées :** qualification native et revues sans blocage, corrections/IT finies, contrat navigateur, autorisation ciblée obtenue par lead.

**Acceptation :** capacités réelles du moteur, certificats et origine ; endpoint/mapping avant API puis identité et bail. Contexte nommé insuffisant. Inventaire/IDs/bail parent ; stop si propriété inconnue, mauvaise cible, port occupé ou capacité absente. Pas d'installation, stack, reconstruction/remplacement ou réparation implicitement autorisés.

**Vérifications :** gardes `ops/deploy/local_full_target.py`, test `test_local_full_target.py` ; action navigateur bornée autorisée, distinction catalogue absent/permission refusée/backend déconnecté. Verdict PASS/FAIL/INCONCLUSIVE et actions exactes de campagne. Baux Docker/Kind/ports/navigateur détenus par lead, skills lifecycle/test-evidence avant préflight.

### 12. Campagne intégrée — preuve-integree

**Profil :** test-engineer, planning requis, fresh, worktree isolé. **Entrée :** qualifier-runtime accepté, mêmes commits/images/config ; divergence déclenche contrôle ciblé.

**Acceptation :** vrai navigateur, connecteur actif et routes exactes via Envoy ; matrice ratifiée et seules garanties supplémentaires convenues. Envoy bon nom accepté/mauvais nom de même CA refusé, échec TLS corrélé à requête et témoin positif. Preuves liées au snapshot, sans secrets/jetons bruts. Skip/indisponibilité non convertis en PASS. Pas de réexécution des suites historiques par principe ou correction production par QA.

**Vérifications :** commandes exactes livrées/validées aux lots précédents, matrice `ops/tests/mesh-public-route-cases-local-full.json` adaptée et ratifiée ; pas d'ancien script mesh lancé tel quel. Handoff preuves/limites et inventaires ; bail jusqu'à vérification infra puis cleanup lead. Builds Linux de livraison distincts des tests natifs.

### 13. Contrôle infra indépendant — verification-infra

**Profil :** infra-verifier, lecture seule, fresh, worktree:false, final_response. **Entrée :** preuve-integree et runtime sous bail parent.

**Acceptation / contrôle :** cible autorisée seulement, configuration chargée/identités/observations/témoins recoupés. Admin Envoy si déjà accessible sans mutation ; pas de port publié, cert modifié, redéploiement, réparation ou nouvelle campagne. Verdict sourcé PASS/FAIL/INCONCLUSIVE ; absence d'observation n'autorise pas un redémarrage automatique. Le lead clôture ensuite ses seules ressources.

### 14. Réconciliation documentaire — reconcilier-documentation

**Profil :** architecte, documentation, fresh, worktree isolé. **Entrées :** revues, preuves et contrôle infra ; décisions ratifiées et commits intégrés.

**Ownership :** contrats de mission/synthèse dans les limites du profil. Index ADR, ledger et rapports restent au lead. Pas de nouvelle décision ou réécriture historique implicitement attribuée.

**Acceptation :** repli bootstrap obsolète après JWKS vide corrigé avec sources/portée ; révocation refresh et validité résiduelle d'accès distinguées. Décisions, implémentation et résultats séparés. Pas de qualification cloud/HSM/production ou promotion Proposed implicite ; limites INCONCLUSIVE conservées. Pas de clôture avec blocage sécurité ; accord compétent pour évolution de Statut/Réalité.

**Vérifications :** assertions contre symboles/tests/artefacts prouvés, liens/dates/statuts, `git diff --check`. Test présent seul insuffisant ; aucun Cargo/runtime requis.

## Parallélisme et barrières restantes

Les quatre racines importées sont indépendantes, dans des worktrees séparés ; contrats/numéros/index sous coordination lead. Après intégration/ratification, corrections identité et remote sont parallélisables sur allowlists disjointes ; alignement navigateur les rejoint après ses entrées. Un chevauchement découvert impose ownership unique/séquencement, jamais deux auteurs du même fichier.

Chemin critique probable : contrat identité → correction et publication SDK autorisée si nécessaire → qualification native → revues → préflight → preuve intégrée → contrôle infra → documentation. La branche navigateur peut devenir critique. Cargo/fixtures restent sérialisés ; les reviewers peuvent lire en parallèle sans créer de fausses branches d'écriture.

L'import initial ne ratifie aucun contrat, n'autorise ni publication SDK ni runtime, et ne lance aucun worker. L'approbation native par start reste réservée à l'utilisateur. Les décisions futures planifiées ne sont plus des blocages de préparation.
