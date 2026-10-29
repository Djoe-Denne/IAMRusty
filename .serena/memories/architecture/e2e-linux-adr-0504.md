# Validation fonctionnelle Linux — ADR0504

Canon : docs/adr/0504-validation-fonctionnelle-linux-k6.md. Statut : Proposed. Réalité : Unimplemented. Voir le fichier ADR.

- Proposition structurelle architecte demandée pour trois missions futures : socle/smoke, portage auth qualifiée, propagation/reprise droits Linux. Aucun test k6 exécuté.
- k6/http pour API ; k6/browser avec vrai Chromium pour OAuth. Un client HTTP seul ne vaut pas navigateur. Versions épinglées,1VU/1itération, checks+thresholds+complétude.
- Parent ops possède cible/baux/contrôles privilégiés/observations/cleanup ; k6 sans API admin générique. RunnerLinuxhorscluster et origines ratifiées préservées.
- Pas OHA/performance/GrafanaCloud/stackobservabilité obligatoire. UT/IT Rust de développement Windows et ADR0200–0202 conservées.
- ADR0412 reste Implemented localement ; portagek6 nonprouvé. DetteHSM acceptée pour campagne, ADR0309Partial inchangée. Préparation seule, config/start futurs avant implémentation.
