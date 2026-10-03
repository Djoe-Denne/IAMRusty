# ADR-0304 — AuthN JWT / arbitrage 2026-10-03

Statut : Accepted. Réalité : Partial. Canon : docs/adr/0304-jwt-acces-plateforme-rs256-jwks.md ; voir le fichier ADR.

- Principal canonique (iss, sub) ; une clé org ne confère aucune identité de compte plateforme.
- Arbitrage humain explicite : confiance JWKS expire à 60 s depuis le dernier snapshot autoritatif validé, mesh et in-process ; outage après la borne = fail-closed. Poll 60 s / 2 s ne prouve pas cette borne.
- JWKS valide vide retire les clés ; registry initialisé vide ne ressuscite pas le bootstrap ; émission par clé révoquée interdite.
- Remote signer HTTP Partial, pas HSM/KMIP livré ni adapters cloud BYOKMS.
- Baseline f060d47 / rustycog ba69c9e : fixes et preuves finales encore requis. IT testcontainers et E2E Kind seulement après intégration complète ; aucune promotion Implemented dans ce tour.