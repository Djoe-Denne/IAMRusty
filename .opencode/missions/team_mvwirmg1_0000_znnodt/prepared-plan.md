# Plan proposé — authentification k6 Windows

Historique de préparation du 2026-10-31. **Révision1 approuvée : les corrections de la policy priment sur cet ancien aperçu.** OpenAI GPT-6.1 Sol uniquement, sans Astra ni long_run. Supprimer du plan exécuté le lot5 de tests du portage/framework et les suites systématiques ci-dessous ; pas de tests de nos tests ni campagne de robustesse. Le lot4 est conditionnel aux seules adaptations indispensables aux scénarios, non une refonte. Le lot6 devient une revue indépendante bornée de la parité/des oracles modifiés, pas deux profils automatiquement imposés. Commandes retenues : diff --check, inspections/exécution auth exactes à consigner avant run, IT0412 conditionnelles à un risque modifié concret. Le reste ci-dessous conserve les sources, invariants et dépendances utiles sans élargir ce contrat corrigé. Voir rapport02 et policy approuvée.

## Résultat et frontières

Porter les garanties de la campagne authentification locale qualifiée vers k6 et Chromium natifs Windows, pas seulement ses requêtes. Succès : parité exhaustive prouvée, parcours positifs et refus discriminants, observations parent, restauration et cleanup. Le PASS about:blank/42 ne qualifie ni HTTP ni OAuth.

Linux reste différé, avec métier JavaScript réutilisable, sans qualification ni réactivation WSL/jobs. Aucun frontend, laboratoire, déploiement, tunnel, serveur de remplacement, changement produit/identité ou retrait des anciens pilotes. GitHub externe reste opt-in distinct ; réutiliser ses preuves applicables.

## Base et sources observées

- Checkout master, HEAD `05941d163eb6632f26c89fc7057af03880148a08`. Commits mission1 `201ec05b` et archives `a24a7ce5` ancêtres vérifiés, sorties0. Un seul worktree Git recensé, branches historiques conservées.
- Gitlink rustycog `02e5db23aac6016aab062c3078696d5f7fd464ef`. Modifications étrangères : workspace Obsidian, rustycog/.serena/project.yml, candidates Context Mode et artefacts d'autres missions. Ne pas les intégrer, réinitialiser ou nettoyer.
- Mission1 et WINDOWS.md rapportent 34 tests Windows sans skip et cas minimal réel k6 2.3.0 / Chrome for Testing156.0.8078.73, fermeture et confidentialité validées. Preuves historiques, non rejouées en préparation.
- Lanceur actuel : validate/run/selftest fermés faute de cible admise. Ancienne campagne : laboratoire arrêté après qualification. Aucune cible actuellement autorisée et joignable établie ici ; aucune permission de réveil déduite.
- `ops/tests/oauth-browser-local-full.js` lu : inscription conditionnelle puis nouveau login, START Bearer, callback sans Bearer, relink réel et trois négatifs. Buffer/claims n'est qu'une observation, pas preuve de confiance ; adapter aux API k6 si conservé. Le pilote accepte401/403, mais la qualification rapporte403 : fixer les refus exacts au contrat, pas les élargir tacitement.

## Canon transmis aux auteurs

- ADR0504 : Accepted / Partial, séparation k6/Chromium/parent ; aucune preuve Linux.
- Contrat `k6-functional-linux.md` : Accepted, référence de structure et séparation, sans imposer sa qualification Linux à la livraison Windows.
- `k6-windows-functional.md` et amendement révision3 : Accepted, socle mission1 ; ne pas réintroduire ACL/verrous/attestations abandonnés.
- `k6-windows-browser-version.md` : Accepted / Implemented, version dans le même cas, pas nouveau lancement ou attestation sandbox.
- État qualifié `20261029-auth-security-qualified-state.md` et trois contrats `0412-oauth-*` : Accepted / Implemented localement ; autorité8080/iam, redirects navigateur18080 exacts, sans promotion k6.
- Contrat remote-signer-secure-transport, ADR0314 et preuves principal/remote : niveaux propres inchangés ; statuts et équivalence des sources à revalider au start avant consommation.
- Ledger source `.cursor/handoffs/20261028-auth-security-closure.md` ; ledger mission `.cursor/handoffs/20261029-k6-02-auth.md`.

## Lots proposés

Noms descriptifs, pas IDs natifs ni tâches admises. Pas de scores/classifieur requis. Consommateurs non publiés avant intégration réelle des entrées ; chaque auteur écrivain isolé, workers feuilles. Un seul Cargo Windows avec configuration24jobs/E: inchangée.

1. **Inventorier les garanties et preuves conservées** — `general`, recherche lecture seule, fresh, sans worktree écrivain. Livrer matrice exhaustive ancien oracle → k6 API / k6 Chromium / parent / UT-IT conservée et registre de gaps. Inclure inscription puis nouveau login, P1/P2/P3, N1/N2/N3, cookies/nonce/PKCE/redirections, effet réel relink, certificats et principal/remote. Localiser les anciens contrôles parent et leurs preuves, sans run. Le lead persiste/intègre avant consommateurs.
2. **Adapter le contrat Windows et l'admission de cible** — `architecte`, docs seules, worktree isolé. Recherche documentaire parallèle au lot1 possible ; finalisation dépend de sa matrice intégrée. Fixer cible existante, autorités/baux/identités, interfaces parent-enfant, ordre des phases, preuves de configuration et compteurs, stimulus/restauration et ACK faisant autorité, nouvelles commandes exactes avant run. Cible absente = prérequis non satisfait, pas autorité alternative. Contrat Proposed jusqu'à ratification compétente ; arbitrage utilisateur si nouvelle frontière non autorisée. Intégration ratifiée avant code consommateur.
3. **Porter les scénarios JavaScript** — `implementer`, propriétaire de `ops/tests/k6/scenarios/auth/` et helpers auth dédiés, worktree isolé. Dépend des lots1/2 intégrés. Vrai Chromium pour cookies/redirections, compte isolé/contexte vierge, pas de cookies forgés ni Bearer global, aucune commande système. Checks/complétude fermés. Aucun changement Rust ni run réel par défaut.
4. **Étendre le parent et lanceur Windows** — `hard-implementer`, propriétaire de run-windows.ps1, helpers Windows et résumé partagé, worktree isolé. Dépend des lots1/2 intégrés, parallèle au lot3 sur interfaces figées et fichiers distincts. Réutiliser supervision existante, admission/baux, phases certificat/observations et finally de restauration/cleanup même si setup échoue. Pas de nouveau superviseur ni privilège donné à k6. Aucun runtime avant gates.
5. **Tester le portage intégré** — `test-engineer`, tests seulement, fresh, worktree isolé, après intégration des lots3/4. Fixtures existantes pour refus avant I/O, portée credentials, callbacks/IdP sans fuite, complétude/phases absentes, source/config périmée, compteur SAN absent/non accru, restauration/cleanup sur échec précoce et interruption. Bug produit = preuve ROUGE remontée, pas correction produit par le testeur. Aucun nouveau serveur/infra de preuve. Les auteurs3/4 gardent leurs contrôles locaux.
6. **Revoir les garanties modifiées** — `test-reviewer` fresh pour matrice/oracles/couverture ; `security-reviewer` fresh pour credentials, cible, privilèges parent et cleanup. Lectures indépendantes et parallélisables après intégration, briefings persistés avant correction. Séparer défauts bloquants, suggestions et dettes hors périmètre. Pas de pipeline Rust/performance/infra automatique.
7. **Qualifier puis publier Windows** — lead après intégration/revues/gates utiles. Comparer les sources aux preuves0412 pour réutiliser ou sélectionner les IT, jamais tout rejouer par principe. Figer les commandes auth réelles avant run ; charger skills evidence/lifecycle avant runtime. Appels seulement sur cible existante autorisée et observée, sans déployer/réveiller Kind/WSL implicitement. Cible absente : poursuivre travail statique utile, aucune parité exécutée annoncée. Stimulus certificat au parent. Verdicts HTTP, auth Chromium, certificat, confidentialité et cleanup séparés. Infra-verifier seulement si changement infra significatif effectivement autorisé, pas extension implicite.

## QA proportionnée proposée

Commandes et critères persistés dans `model-policy.json/verification`, aucune commande de test exécutée ici :

- `git diff --check`.
- `python.exe -B -m unittest discover -s ops/tests/k6/tests -p 'test_*windows*.py'`.
- `python.exe -B -m unittest discover -s ops/tests -p 'test_oauth_browser_alignment_source.py'`.
- `python.exe -B -m unittest discover -s ops/tests -p 'test_oauth_idp_simulator.py'`.
- Si sources/parcours/matchers invalident les preuves correspondantes : `cargo test -p iam-service --test oauth_browser_transactions`, puis `cargo test -p iam-service --test auth_transactions_postgres`, puis `cargo test -p iam-service --test oauth_cleanup_lifecycle`, selon sélection justifiée. Fixtures testcontainers existantes et cleanup tracé ; les17IT restent IT Windows, pas E2E artificiels.
- Les nouvelles commandes auth n'existent pas encore : le contrat fixe leur forme, les auteurs leurs chemins effectifs ; liste exacte des inspections k6, tests ajoutés et lancement parent gardé obligatoire avant run. Aucune chaîne conditionnelle du draft n'est une commande à exécuter automatiquement.

Critères : chaque oracle obligatoire affecté et prouvé ou gap déclaré ; inscription réelle puis nouveau login ; réassociation effective avec mêmes identités et autorité configurée, sans parsing JWT comme trust. Tous les cas exécutés/achevés, checks/thresholds fermés, cardinalités exactes non nulles et résumé courant. Négatifs discriminants et observations Envoy/IAM/IdP du même run/incarnation/configuration. Mauvais nom de certificat signé même CA → hausse du compteur approprié, puis restauration positive ;503 seul insuffisant. Aucun PASS pour setup interrompu, zéro cas, résumé absent/périmé ou cible manquante. Pas credentials/URL sensible/traces/HAR publiés. Restauration et zéro ressource possédée active sans bail après succès ou échec.

Réutiliser principal/remote/GitHub applicable avec comparaison des sources. Pas suite Manifesto globale ni qualification HA/HSM/Linux. Revue indépendante des oracles modifiés obligatoire, sans chaîne QA supplémentaire imposée.

## Politique et décisions encore ouvertes

Version2, catalogue fourni inchangé, seuil de planification50, classification désactivée, aucune délégation long_run demandée. Pas d'override de modèle au spawn. QA proportionate recommandée ; light réduit les vérifications optionnelles mais pas les exigences de parité/revue, reinforced ajoute plan explicite et revue bornée sans nouveau labo. Corrections persistées et représentées par config avant start utilisateur.

La cible actuelle et son admission restent à établir après start par lectures autorisées des ancrages/baux existants. Absence de cible ou permission indispensable = arbitrage ciblé, pas permission déduite de start. Nouveau contrat non encore écrit/ratifié. Aucune modification source Rust explorée : charger workflow MCP avant toute investigation Rust ultérieure.

## Livraison future

Actualiser la photographie documentaire seulement après vérification. Garder0412 Implemented local, sans attestation k6 anticipée. Préserver anciens pilotes/travaux étrangers ; intégrer les livrables durables sur master, commit/push et référence distante vérifiés, retirer les seuls arbres/branches de mission selon permissions, archiver nativement. Si un prérequis manque, documenter livré/non exécuté/non clôturé. La préparation présente ne publie ni ne nettoie rien.
