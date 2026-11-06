# IAM-IdP — ADR-0413

Statut : Proposed. Réalité : Unimplemented. Voir le fichier ADR `docs/adr/0413-oauthconnect-multi-instances-profils.md` et le contrat `docs/contracts/oauthconnect-multi-instances.md`.

- Proposition rédigée le 2026-11-06 dans le worktree isolé `oauthconnect-architecture-20261106`, base b39c6c4, puis intégrée par le parent au checkout canonique. Cette intégration documentaire ne vaut ni acceptation de l'ADR ni implémentation de la refonte ; se référer au ledger `.cursor/handoffs/20261106-oauthconnect-tdd.md` pour la publication et les gates.
- Un service RustyCog OAuthConnect, instances logiques au boot et deux profils OAuth mappé API+JSONPath / OIDC standard ; slugs github/gitlab et UUID/liens IAM préservés.
- Supersession partielle de 0408/0411 seulement proposée, pas effective. API v2 authorize/complete renvoie tokens actuels + profil validé, jamais claims riches ni ID token ; nonce durable IAM proposé, pas de cache de corrélation Connect.
- Accept humain requis sur le texte, regroupement v2 et delta email GitHub sans fallback public ; bascule GitLab conditionnée à preuve externe sub OIDC = ancien id string. Claims riches/actions toujours différés.
- HMAC PATH complet par instance, HTTPS/PKCE/secrets stricts conservés ; 0505 reste Proposed/Implemented. Aucun code, test, Cargo ou runtime exécuté dans ce cadrage.
