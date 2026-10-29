# Validation fonctionnelle Linux — ADR0504

Canon : docs/adr/0504-validation-fonctionnelle-linux-k6.md et docs/contracts/k6-functional-linux.md. Statut : Accepted pour la V1 mission1, ratifiée le 2026-10-29 ; qualification incomplète. Cette mémoire est un digest non canonique. Lire les documents Git et le ledger courant avant toute exécution.

- Proposition initiale affinée par architecte/pool-3, base 9896140 ; V1 ratifiée et intégrée par le lead dans c9b537c, addendum du trajet protégé dans 3f2e8fb. Les anciennes candidates Proposed ne remplacent pas ces décisions.
- Un GET public /iam/health,200/corps exact OK ; per-vu-iterations1VU/1itération et une requête. Résumé courant strict, cardinalités/checks/confidentialité et cleanup obligatoires ; HTTP/Chromium/cleanup distincts.
- Parent ops seul propriétaire de cible/bail/preuves/cleanup ; lanceur mince sans création/réparation de bail ni override root/privacy. Garde POSIX présente mais non qualifiée : chemin privé, kubeconfig fixe, roster Docker, endpoint et UID revalidés ; aucun bail Windows copié/adopté.
- Blocage source découvert : voie local-full18080→CP30080→Envoy10000→IAM non publique ; préfixe /iam/ garde ext_authz, check.rs refuse sans Bearer. Aucun token/exemption/tunnel ajouté. Origine publique existante à identifier/figer sous autorité parent ; aucun bail POSIX/accès actuel prouvé, HTTP INCONCLUSIVE/refus sans requête sinon. Ancien lab arrêté.
- CI dédiée ubuntu-24.04/actionsSHA/contents:read : outils et Chromium sandboxé about:blank sans cible/secret applicatif de PR. k6v2.3.0 Linuxamd64/checksums officiels ; CfT156.0.8078.73 candidat officiel actuel, hashes à acquérir/revoir avant exécution, non qualifié ; pas Chrome149 historique.
- Mission2/3 non lancées ; UT/IT Rust Windows0200–0202 et preuves0412/17IT à leur portée. Lectures/recherche documentaire seulement, aucun runtime démarré ni preuve k6 acquise.
