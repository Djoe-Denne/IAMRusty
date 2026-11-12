---
title: >-
  QA proportionnée — la preuve suffisante au coût justifié
category: concepts
tags: [testing, architecture]
sources:
  - .opencode/agents/qa-planner.md
  - .opencode/agents/test-engineer.md
  - .opencode/agents/test-reviewer.md
  - docs/history/iac-k6-qa-20261112/README.md
summary: >-
  Le planner définit quoi tester et comment, sans imposer un framework. Un setup substantiel exige obligation, alternative, coût de maintenance et arbitrage, sans réduire la sécurité.
provenance:
  extracted: 0.96
  inferred: 0.04
  ambiguous: 0.0
created: 2026-11-12
updated: 2026-11-12
---

# QA proportionnée : prouver le besoin, challenger les moyens

Le 12 novembre, l'utilisateur décrit un biais récurrent du QA planner vers les
détours et les setups trop importants. Le profil AIForAll est recentré dans
**24d2a52** : proposer les tests et leur méthode pour la fonctionnalité demandée,
pas optimiser un dispositif QA complet. Cette méthode est portable ; le pin
ADR/fixtures AIForAll et la modification locale ne migrent pas automatiquement
vers les autres projets.

## Ancrer la preuve dans le lot

Résultat attendu, critères convenus, frontières touchées et hors-périmètre
précèdent les moyens. Chaque test vise un critère ou un défaut plausible précis :
scénario, niveau, fixture/données, oracle observable, arrangement→stimulus→assertions,
attente déterministe et fermeture des effets possédés.

Une liste courte suffit. Matrices chiffrées, analyses de défaillance, quotas et
pourcentages de budget ne sont plus systématiques. Une hypothèse n'est pas une
spécification ; la présence d'un test n'est pas une exécution. Les obligations
de sécurité et ADR applicables ne disparaissent pas pour autant.

## Un point de décision, pas une usine à tests

Réutiliser les moyens existants, puis une petite adaptation locale. Si le scénario
demande beaucoup de code spécifique, un nouveau harness/processus, des secrets,
une orchestration ou de l'instrumentation produit, expliciter :

1. **Obligation** : que veut-on prouver et pourquoi l'existant ne suffit-il pas ?
2. **Alternative minimale** : que prouve-t-elle, et que ne prouve-t-elle pas ?
3. **Coût** : test + setup + maintenance + fragilité + effets/cleanup, avec incertitudes.
4. **Décision** : réutiliser, simplifier, accepter après arbitrage ou différer.

Le nombre de lignes est un signal, pas un seuil de refus. Une petite fixture locale
n'exige pas un accord à chaque ligne ; un plan QA n'autorise pas non plus à
construire aveuglément l'outillage qu'il mentionne. Le test-engineer doit remonter
un coût substantiel découvert en implémentation plutôt que suivre un ID `G-n`.

## Détour utile ou dérive ?

- **Nécessaire** : critère convenu/canon ou risque concret de sécurité/corruption
  introduit ou exposé. Justifier scénario d'échec, ancrage, observation manquante
  et action minimale. Un risque grave plausible peut bloquer la frontière avant run.
- **Optionnel** : diagnostic/confort/couverture supplémentaires sans nécessité
  démontrée pour la sortie actuelle ; pas de gate implicite.
- **Hors périmètre** : dette adjacente, refonte générique ou audit global ; pas de
  nouveau chantier détaillé sous le prétexte de « qualité ».

Si aucune méthode admissible ne prouve un critère obligatoire, ce critère reste
**blocked** et demande arbitrage. Ne pas le déclasser en option, réduire l'assertion
ou mocker la transaction/ACL/course à prouver. Continuer les checks indépendants.
La proportionnalité réduit la dérive, pas le niveau de confiance promis. ^[inferred]

## Application et limites

La stratégie [[projects/aiforall/skills/testing-k6-fixture-first]] illustre la
frontière : chart déclaratif, parent mince, vrais parcours API, isolation et
inverse causal définis avant effets. ADR0200–0202 restent applicables aux IT,
mocks sortants seulement et queues opt-in selon le comportement.

Le planner reste feuille, sans code/runtime/délégation ; les profils d'écriture
et de revue restent optionnels. Les matrices/IDs ne sont pas réintroduits en aval
si une liste de checks suffit. Les modèles et permissions n'ont pas changé.

Validation livrée : YAML, conservation des frontmatters et contrôles textuels,
**pas de mesure comportementale** du nouveau profil. Source de discussion et
sous-agents : [[projects/aiforall/references/session-iac-k6-qa-20261112]].
Contexte de coordination : [[projects/aiforall/concepts/orchestrator-agent-harness]].
