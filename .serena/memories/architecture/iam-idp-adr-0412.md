# IAM-IdP — ADR 0412

Canon : docs/adr/0412-oauth-transaction-persistante-liee-navigateur.md. Statut : Accepted. Réalité : Implemented (qualification locale ratifiée par le lead sous délégation utilisateur, 2026-10-29). Voir le fichier ADR.

- Transaction OAuth persistante writer, binding state/cookie, consommation atomique avant échange, PKCE et refus replay ; START JWT plateforme, callback relink exact autorisé par transaction sans Bearer.
- Chrome149 neuf via Envoy kind-aiforall-local-full/18080 : P1/P2login200/P3callback exact et N1–N3=403 même run ; connecteur GitLab réel avec IdP synthétique.
- Source image/navigateur7b8c1f95, SDK02e5 ; IT complémentaires8606173 code exécutable identique. Trois suites :17PASS,0ignored/filtered. Revue oracles et infra bornées, SAN réel discriminant séparé.
- Historiques Partial2026-10-04/08 et échec401 avant correctif conservés dans le canon ; pas de réécriture de leurs preuves.
- Pas qualification production/edgeTLS/HA/deuxrépliques déployées/toutes projections. Pas mesure SQL/config_dump directe ; limites MISSING explicites. Nœuds possédés arrêtés sans suppression, ressources étrangères préservées.
