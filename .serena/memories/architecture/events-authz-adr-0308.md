# ADR-0308 — AuthN mesh / arbitrage 2026-10-03

Statut : Accepted. Réalité : Partial. Canon : docs/adr/0308-mesh-authn-jwt.md ; voir le fichier ADR.

- Utilisateur a rouvert explicitement « pas de TTL pour Implemented » : confiance JWKS bornée à 60 s, y compris pendant un outage.
- Temps monotone depuis dernier snapshot autoritatif validé ; hit ou refresh échoué ne prolonge pas la confiance ; sans snapshot initial, aucun principal accordé.
- Snapshot valide keys: [] accepté et cache vidé ; fallback bootstrap après révocation de la dernière clé à corriger.
- Produit reste ext_authz HTTP + principal (iss, sub), mTLS gateway ; ni filtre JWT Envoy, ni SPIFFE obligatoire.
- Baseline f060d47 / gitlink ba69c9e propre ; poll binaire 60 s / Kind 2 s déjà présents, S2S présent mais preuves actuelles à rejouer. Expiration caches pas démontrée : Partial maintenu.
- Nouveaux tests à écrire pendant les lots ; IT et E2E seulement à la fin. Publication SDK puis gitlink exige décision parent explicite, pas commit automatique.