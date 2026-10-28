---
title: "ADR 0314 — Autorité du principal dans OpenFGA"
status: accepted
feature_status: unimplemented
---

# Autorité du principal dans OpenFGA

Canon : `docs/adr/0314-principal-authority-through-openfga.md`.
Contrat : `docs/contracts/authz-trust-domain-boundary.md`.

Décision ratifiée explicitement par le lead le 2026-10-28 sous délégation utilisateur, Réalité Unimplemented : conserver `(iss, sub)` jusque dans Subject, cache et projections ; droits plateforme historiques uniquement avec provenance établie, autres autorités encodées sans collision. Refus local des interfaces UUID-only, pas d’interdiction globale des identités organisation. L'Accept ne prouve aucune correction ou campagne exécutée.

Related : [[projects/aiforall/decisions/0304-access-jwt-trust]]. Le canon et son Statut/Réalité priment ; aucune preuve de livraison dans cette note.
