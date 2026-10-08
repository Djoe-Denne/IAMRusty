# IAM signing — ADR-0311

Statut : Accepted. Réalité : Implemented. Canon : docs/adr/0311-crypto-privee-deleguee-aux-providers.md ; voir le fichier ADR.

- Accept explicite de Djoé Denne le 2026-10-08 ; rédaction Proposed du 2026-10-07 conservée comme historique.
- Crypto privée production déléguée ; rotation via rotate_provider_material() (services/IAMRusty/infra/src/signing/rotate.rs:153), Transit key_version immuable exigée dans transit/probe/scoped/registry.
- PEM dev/test explicite seulement, fixtures IsolatedTest ; plateforme OpenBaoTransit sans fallback (services/IAMRusty/setup/src/app.rs:340–350,1017,1036,1196–1208).
- Amendement ciblé de 0304 §4 effectif, pas supersession entière ; autorité entreprise, guards, fences et workloads Lazaret conservés.
- Implémentation livrée, preuves vérifiées par le contrôleur au HEAD e8fcaea ; units + IT IAM vertes sur master confirmées par l’utilisateur le 2026-10-08, pas artefact CI archivé. Aucun PASS sécurité global ni E2E 0412 implicite.
