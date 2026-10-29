---
title: "k6 — validation fonctionnelle Linux proposée"
category: concepts
tags: [architecture, security, visibility/internal]
sources:
  - docs/adr/0504-validation-fonctionnelle-linux-k6.md
  - docs/contracts/k6-functional-linux.md
  - docs/missions/20261029-k6-recherche-et-enchainement.md
summary: "Trois missions k6 proposées : socle, portage authentification, puis droits Linux ; Chromium réel pour OAuth et verdicts fonctionnels non vides."
provenance:
  extracted: 1.0
  inferred: 0.0
  ambiguous: 0.0
created: 2026-10-29
updated: 2026-10-29
---

# k6 : une proposition, pas une campagne exécutée

ADR 0504 et son contrat sont **Proposed / Unimplemented**. Trois ordres sont rédigés : socle et smoke ; portage des preuves d'authentification ; propagation et reprise des droits sous Linux. Chaque étape dépend de l'intégration et des preuves de la précédente. Aucun k6 ni Chromium installé, aucune de ces missions lancée.

## Frontières à préserver

`k6/http` porte les scénarios API ; `k6/browser` utilise Chromium réel pour cookies et redirections OAuth. Une succession de requêtes HTTP n'est pas une preuve navigateur. La campagne réutilise les oracles de [[projects/aiforall/references/auth-security-closure-20261029]] et les routes exactes de [[projects/iamrusty/decisions/0412-oauth-transaction-browser-binding]], sans changer l'autorité pour faciliter le runner.

Le parent opérations conserve le contrôle du laboratoire, des perturbations, observations et du nettoyage. Pas d'API administrative générique ajoutée pour k6. Setup/teardown ne garantissent pas le nettoyage après interruption. Les contrôles fonctionnels doivent avoir des thresholds qui rendent l'exécution fautive en cas d'échec, avec preuve de complétude et de non-vacuité ; les checks seuls ne suffisent pas.

## Périmètre et recherche

Les E2E et services de campagne sont Linux ; les tests Rust natifs Windows gardent leur stratégie actuelle. OHA, performance, nouvelle UI, qualification HA exhaustive, Grafana Cloud et nouvelles plateformes ne sont pas inclus. Les preuves GitHub applicables sont réutilisables ; GitLab réel n'est pas imposé.

La release officielle k6 v2.3.0, publiée le 21 septembre 2026, est une base candidate identifiée pendant la recherche, pas une installation constatée. Les pages Grafana `latest` étant mobiles, les futures missions doivent vérifier les API contre la version effectivement épinglée. Ne pas recopier un exemple navigateur `no-sandbox` dans une preuve de sécurité.

La dette matérielle HSM reste acceptée et non bloquante pour ces campagnes, mais n'est pas qualifiée ; ADR 0309 reste Partial. Les résultats attendus ne deviennent pas des résultats observés par leur inscription dans le wiki.

Références officielles et trois ordres : `docs/missions/20261029-k6-recherche-et-enchainement.md`. Projet : [[projects/aiforall/aiforall]].
