# IAM-IdP — ADR-0413

Statut : Accepted (acceptation humaine explicite le 2026-11-06). Réalité : Unimplemented. Voir le fichier ADR `docs/adr/0413-oauthconnect-multi-instances-profils.md` et le contrat `docs/contracts/oauthconnect-multi-instances.md`.

- Proposition rédigée le 2026-11-06 dans le worktree isolé `oauthconnect-architecture-20261106`, base b39c6c4, puis intégrée par le parent au checkout canonique. Cette intégration documentaire ne vaut ni acceptation de l'ADR ni implémentation de la refonte ; se référer au ledger `.cursor/handoffs/20261106-oauthconnect-tdd.md` pour la publication et les gates.
- Un service RustyCog OAuthConnect, instances logiques au boot et deux profils OAuth mappé API+JSONPath / OIDC standard ; slugs github/gitlab et UUID/liens IAM préservés.
- Supersession partielle de 0408/0411 et interface 0407 ratifiée, périmètre annoté dans les ADR. API v2 authorize/complete renvoie tokens actuels + profil validé, jamais claims riches ni ID token ; nonce durable IAM accepté, pas de cache de corrélation Connect.
- Texte, regroupement v2 et delta email GitHub primaire seul sans fallback public acceptés explicitement. Bascule GitLab/retrait legacy restent conditionnés à preuve externe sub OIDC = ancien id string et aux gates de cutover ; unités/IT TDD autorisées après intégration. Claims riches/actions toujours différés.
- HMAC PATH complet par instance, HTTPS/PKCE/secrets stricts conservés ; 0505 reste Proposed/Implemented. Aucun code, test, Cargo ou runtime exécuté dans ce cadrage.
