---
title: RustyCog Outbox — durabilité et leases
category: references
tags: [reference, events, architecture, rust]
sources:
  - _sources/opencode-telegraph-outbox-20261027/inventory.json
  - docs/contracts/telegraph-notification-outbox.md
  - docs/platform/events-outbox.md
summary: "Outbox partagée : recorder dans la transaction métier, dispatcher réel et reprise après lease. Référence restaurée depuis la livraison Telegraph, sans garantie exactly-once."
provenance:
  extracted: 0.96
  inferred: 0.04
  ambiguous: 0.00
created: 2026-10-27T19:27:11Z
updated: 2026-10-27T19:27:11Z
---

# RustyCog Outbox

Cette page restaure une référence précédemment liée mais absente sur disque. Son périmètre vérifié est l'usage partagé livré par [[projects/telegraph/concepts/durable-notification-publication]], pas un audit exhaustif de tous les consommateurs du SDK.

## Contrat utile

Le recorder doit écrire l'intention dans **la transaction de l'agrégat** ; un second commit indépendant ne donne pas cette atomicité. Le dispatcher prend en charge les lignes éligibles et marque published après succès du publisher, ou conserve un état retryable après échec.

Une annulation locale peut laisser un claim publishing committé sous lease. La nouvelle instance attend l'éligibilité persistée puis reprend la même intention ; la jointure locale n'implique pas rollback de PostgreSQL ou d'un envoi accepté à distance.

La livraison Telegraph conserve les paramètres 30 s de lease, 2 s de poll, 5 s de backoff et 20 essais. Disabled/degraded NoOp doit suspendre le dispatch, pas consommer les lignes avec un succès fictif. La lease n'est pas la visibilité SQS du consommateur.

## Garantis et non garantis

Atomicité SQL locale et reprise durable ne rendent pas SQL+SQS atomiques ensemble. Les doublons restent possibles ; le consommateur doit converger. Des conditions de propriété SQL ne font pas à elles seules du fencing SQS. La mission n'ajoute ni unlock global, retry infini, CDC ni nouvel opérateur d'infrastructure.

Le défaut de fenêtre temporelle de la preuve est traité par [[projects/aiforall/skills/proving-leased-outbox-recovery]], pas en diminuant le verrou pour faire passer un test. Le dispatcher SDK n'a pas été modifié dans la série HTTP129→d60.

## Preuves et statut

La preuve composant contrôle avant/après expiration et vrai SQS ; la verticale ajoute les droits/replay/HTTP. Comptes 1/0/1 et 1/1/1 ne sont pas interchangeables. Résultats/SHA et limites : [[projects/aiforall/references/mission-telegraph-outbox-20261027]].

[[projects/aiforall/decisions/0300-events-authz]] distingue la photographie ADR0301 Partial et l'extension Telegraph livrée. Cette page ne résout pas l'écart IAM d'une transaction séparée.

## Related

- [[projects/telegraph/references/telegraph-service]] — composition root et sélection du vrai publisher.
- [[projects/telegraph/references/telegraph-entity-model]] — frontière atomique et comptes métier.
