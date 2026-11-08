# Documentation OAuthConnect / IAM et ingestion de session

## Résultat demandé

Transformer l'intégralité de la session et de ses sous-agents en connaissances durables du wiki AIForAll ; intégrer les schémas d'architecture appréciés au dépôt et approfondir JWKS, rotation et propriété des clés, TLS et liens avec JWT/HMAC/OAuth/OIDC.

Base `b732b60bc76eda59110f3cd1b6924696bfbd0f35`. Mission documentaire, pas de nouvelle décision d'architecture ou de déploiement. Le wiki est une synthèse sourcée, ADR/contrats et code restent le canon. Les anciens schémas datés restent des photographies historiques, pas des états live.

## Livrables et vérification proportionnée

- Inventaire de couverture de la session racine et de ses descendants, sources privées conservées localement ; publication de synthèses expurgées, pas de secrets ni transcripts bruts dans Git.
- Guide technique pédagogique français, des notions cryptographiques aux responsabilités et chemins de code, distinguant implémenté/configuré/inconnu ; standards primaires cités.
- Schémas actuels détaillés, sources versionnées et rendus HTML/SVG autonomes consultables horsligne, index/liens portables ; un Canvas seul ne constitue pas la documentation.
- Pages wiki dédupliquées/reliées, provenance/frontmatter/manifest/index/log cohérents ; index QMD AIForAll uniquement, sans reconfiguration globale.
- Contrôles de liens/frontmatter/structure HTML, rendu navigateur si disponible et revue indépendante bornée des explications de confiance/rotation/TLS. Les validations non exécutées restent explicitement non exécutées.

Exécution directe sans Ensemble/PM, branches documentaires isolées si utiles. Aucun Cargo, Docker/Kind/WSL, secret ou rotation réelle, code applicatif, pin SDK, configuration de déploiement ou mécanisme de test nouveau. Publication et nettoyage de mission sont vérifiés séparément.
