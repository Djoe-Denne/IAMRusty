---
title: Session OAuthConnect et IAM — conception, preuves et schémas
category: references
tags: [architecture, oauth, iam, testing, visibility/internal]
sources:
  - docs/history/oauthconnect-iam-20261108/README.md
  - docs/history/oauthconnect-iam-20261108/sessions.md
  - docs/history/oauthconnect-iam-20261108/coverage.json
  - docs/guides/iam-oauthconnect-securite-clefs-jwks-tls.md
  - docs/missions/20261108-iam-oauthconnect-v2-results.md
summary: >-
  Session du6–8novembre et36feuilles distillées : contrat/config/mapping, HTTP/secrets, OIDC/service, IAM v2 et schémas. Couverture bornée, sources expurgées, limites explicites.
provenance:
  extracted: 0.98
  inferred: 0.02
  ambiguous: 0.0
created: 2026-11-08
updated: 2026-11-08
---

# Session OAuthConnect et IAM

## Portée de l'ingestion

Cette référence couvre l'état natif accessible de la session racine et de ses36anciennes feuilles, du6au8novembre, jusqu'à la demande de documentation du **8novembre2026 à12:25:19.929UTC**. L'inventaire retrouve39sessions liées, dont37anciennes capturées,4458messages avant frontière et234pages de messages épuisées. Aucun doublon ou ancienne session absente n'est observé dans cet inventaire.

La couverture n'est pas un export brut intégral :5542parties de raisonnement interne et les contenus système/compaction sont exclus ;512messages privés/runtime restent limités aux métadonnées. Trente messages racine après frontière et les deux nouvelles feuilles documentaires sont exclus pour éviter une ingestion autoréférentielle. Sessions supprimées/forks sans lien déclaré et conversations hors projet ne sont pas prouvés absents. Les résumés de compaction ne servent pas à inventer des échanges manquants.

Les projections expurgées d'analyse et l'index de témoins restent locaux ; le dépôt conserve une **distillation publiable**, un inventaire et les identifiants de provenance, pas les tool inputs privés, secrets, instructions internes ou clés. Les37tailles et empreintes des projections ont été recoupées avant rédaction. Un hash atteste des octets capturés, pas à lui seul de la vérité d'une déclaration d'agent.

Les fichiers du wiki constituent l'ingestion versionnée. Le refresh QMD n'est pas exécuté : les chemins de mise à jour CLI/SDK installés effacent un cache, contraire au contrat local. La recherche indexée peut donc rester en retard ; aucune purge ou reconfiguration globale n'est faite pour la forcer.

## Lire toute l'histoire, pas seulement le dernier lot

1. **Cible et contrat v2** : instances configurables, profils OAuth/OIDC, résultat tokens+profil groupé, responsabilités IAM et nonce indépendant. Claims riches et délégation d'actions restent différés.
2. **Configuration et projection** : catalogue validé au boot, profil typé, JSONPath de projection borné, budgets, sélection déterministe d'email ; aucun langage métier ou fallback vendor implicite.
3. **Transport et secrets** : frontières URL/origine, HMAC par instance/PATH exact, résolutions KV/direct explicites, TLS et bornes ; catalogues partiels refusés.
4. **OIDC et service** : Discovery/JWKS, snapshots publics effectifs, contrôles cryptographiques/contextuels, cache et cooldown, registry/routing HMAC, composition et vrai binaire. La couture AppState reste JWT-refusante, pas une nouvelle autorité.
5. **Raccord IAM** : nonce durable, capacité après commit et double-purge, LOGIN/LINK/RELINK sans v1, migration additive et politiques email conservées.
6. **Schémas et pédagogie** : vues Canvas appréciées, export HTML horsligne, puis sources versionnées et explications approfondies des clés/JWKS/rotation/TLS.

Le [récit durable complet](../../../../../docs/history/oauthconnect-iam-20261108/README.md) conserve décisions, tentatives, incidents, contributions des auteurs/reviewers et leurs précédences. Le [répertoire des sessions](../../../../../docs/history/oauthconnect-iam-20261108/sessions.md) couvre chaque ancienne feuille, y compris celles réutilisées sur plusieurs tranches ; le titre natif ne décrit pas toujours son dernier rôle.

## Résultats historiques, sans faux cumul

- Connect :295tests dans le bilan du lot service, publication `e8952d8` ; les états intermédiaires114/189 sont des périmètres antérieurs, pas des tests à additionner.
- IAM :325unités (30+87+62+146) et80IT sur11binaires (9+17+2+11+17+4+11+5+2+1+1), publication `b732b60` du raccord et de sa clôture.
- Revues indépendantes : verdicts statiques bornés, pas exécutions. Les oracles email manquants ont été ajoutés puis exécutés ; pas de transformation d'une compilation ou d'un setup raté en RED métier.
- Clippy strict infra IAM reste rouge sur dette préexistante. Fournisseurs réels, comparaison `sub` GitLab, chaîne G5, cutover et retrait legacy restent non qualifiés par ces campagnes.

Ces résultats sont des preuves **antérieures réutilisées**, pas des tests réexécutés pendant l'ingestion documentaire. Connect et IAM ont des frontières de test différentes ; leurs totaux ne fabriquent pas une preuve E2E intégrée.

## Incidents et corrections à ne pas perdre

UserInfo signé : fixture au mauvais MIME (`text/plain`), corrigée vers `application/jwt`, sans assouplir la production. Blocages Send/TLS : relation de lifetime redondante et DN synthétiques CA/feuille ; pas de désactivation de CA/SAN. La composition JWT inutilisée a été remplacée par la couture HMAC-only explicitement refusante.

Upgrade IAM : UUID obligatoire absent du montage avant l'upgrade, corrigé dans le test seulement. Callback : premier démarrage Postgres opaque, collisions de port déclarées ensuite ; retry unique demandé par l'utilisateur,17/17sans code modifié, **cause initiale inconnue**. Le port sondé puis relâché est un risque général, pas une cause démontrée de cet incident.

Onze fixtures identifiées ont été retirées. Un conteneur `created` et douze volumes opaques sont conservés **sur demande explicite de l'utilisateur** ; cette mission documentaire ne rouvre ni diagnostic ni nettoyage. Voir [[projects/aiforall/concepts/testcontainers-resource-churn]].

## Navigation et canon

- [[projects/oauthconnect/oauthconnect]] et [[projects/iamrusty/decisions/0413-oauthconnect-multi-instances]] — ADR Accepted/Partial, code local qualifié mais pas de bascule live.
- [[projects/iamrusty/concepts/oauthconnect-v2-durable-context]] et [[projects/iamrusty/concepts/oauth-provider-linking]] — mécanismes et métier.
- [[projects/oauthconnect/concepts/oidc-jwks-and-key-rotation]] et [[projects/oauthconnect/concepts/tls-hmac-and-signatures]] — distillation pédagogique.
- [Guide approfondi](../../../../../docs/guides/iam-oauthconnect-securite-clefs-jwks-tls.md), [sources code/normes](../../../../../docs/guides/iam-oauthconnect-securite-sources.md), [atlas versionné](../../../../../docs/diagrams/README.md).
- [Inventaire de couverture publiable](../../../../../docs/history/oauthconnect-iam-20261108/coverage.json).
