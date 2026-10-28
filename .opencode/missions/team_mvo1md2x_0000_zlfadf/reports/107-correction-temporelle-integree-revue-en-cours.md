# Correction du test intégrée, examen indépendant en cours

## Étape et garantie conservée

Le correctif demandé en106 est préparé et intégré dans l'arbre du lead. Il vérifie une date future dès la première observation de l'échec, avant la fin de l'attente indépendante de douze secondes. Une ligne absente, une erreur SQL ou la limite de temps atteinte font échouer le test. La production, les délais et les contrôles de reprise et d'accès ne changent pas.

Le même auteur a formaté puis compilé la cible avec `--no-run` : succès, aucun test exécuté. Le lead a appliqué le patch borné livré sans commit d'auteur, vérifié l'empreinte exacte du fichier intégré et `git diff --check`, puis archivé les huit artefacts fournis. Aucun merge de branche non committée ni contournement de la fusion native défectueuse n'est revendiqué.

## Examen et barrière suivante

Une revue indépendante fresh de la préservation des garanties est admise dans la politique existante, sans nouveau modèle ou nouvelle classification. Son briefing est persisté, avec observations vérifiées et sources/diff, sans conclusions du développeur. Des copies locales vérifiées sont disponibles pour éviter un refus de lecture hors répertoire ; le reviewer n'écrit pas et ne lance aucun Cargo/runtime.

L'unique validation106 reste **non consommée**. La revue doit terminer avant préflight runtime et exécution. Le diagnostic105 reste rouge et archivé. La documentation a une proposition d'amendement105/106, non appliquée, qui ne prétend aucune validation ou publication acquise.

### Ajustement des entrées de revue

Le sandbox fichiers du reviewer autorise TI, contrairement au répertoire de worktree affiché par le routage d'équipe. La lecture du briefing externe puis celle de la copie pool15 ont été refusées ; l'auteur n'a pas contourné la permission et n'avait pas commencé l'analyse. Le lead copie donc les neuf entrées sous **TI/.tmp/review106-inputs/**, vérifie l'égalité des empreintes et confirme la reprise de la même session de revue. Aucun nouveau reviewer, source modifiée, test ou autorisation humaine supplémentaire.

## Annexe

- Source intégrée : TI/services/Telegraph/tests/notification_created_routing_test.rs, SHA256 `6ac7be0e9e220a501241a0b48bf261c6fd7e14aa921ab098dbbfa29e7df071e1`, +34/−1 contre AI1f2.
- Patch : `ceb2b730fea2f8524b56243dd263daacc8441a0204b3d6e12b1362db8bd5dad0`. Archive TI/.tmp/vertical-temporal-correction106-integrated/. Huit fichiers réellement présents archivés ; aucun fmt-check.json séparé n'a été fourni. Le premier essai de copie attendait ce nom absent et s'est arrêté ; la reprise a conservé et vérifié les copies déjà faites.
- Compilation auteur : `cargo test --locked -p telegraph-service --test notification_created_routing_test --no-run`, exit0, 12,42s. Formatage et diffcheck0 ; aucune compilation redondante du même fichier/hash par le lead.
- Brief : `.cursor/review-briefings/20261027-temporal-oracle106.md` à la racine principale. Reviewer pool15/test-reviewer, tâche `task_mvqfk78y_000c_0khmvf`, sources intégrées et copies `.tmp/review106-inputs/` dans son arbre isolé.
- SDKd60 propre, pin129 inchangé ; seul test et différence SDK préexistante dans le diff du lead. Limite : compilation et inspection ne sont pas une preuve de réussite comportementale.
