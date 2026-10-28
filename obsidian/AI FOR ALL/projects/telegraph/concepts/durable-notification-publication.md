---
title: Publication durable des notifications Telegraph
category: concepts
tags: [communication, events, notifications, architecture]
sources:
  - _sources/opencode-telegraph-outbox-20261027/inventory.json
  - docs/contracts/telegraph-notification-outbox.md
  - docs/platform/events-outbox.md
summary: "Telegraph enregistre notification, livraison éventuelle et événement ensemble, suspend les transports NoOp et reprend l'envoi après l'éligibilité SQL."
provenance:
  extracted: 0.95
  inferred: 0.05
  ambiguous: 0.00
created: 2026-10-27T19:27:11Z
updated: 2026-10-27T19:27:11Z
---

# Publication durable des notifications Telegraph

L'intention durable ferme la fenêtre « persistance métier réussie, publish réseau perdu ». `create_notification` et `create_notification_with_delivery` utilisent le writer commun : notification + éventuelle livraison + une ligne outbox dans **la même transaction**, sans publish post-commit conditionnant le succès métier.

## Frontières et états

- `NotificationCreated` garde les identités notification/destinataire/événement et le payload validés avant commit. Le domaine n'a pas à exposer SeaORM pour obtenir cette atomicité.
- Une migration additionnelle introduit l'outbox ; déploiement avant le binaire, sans backfill historique implicite.
- Seul un publisher Live concret non-NoOp déclenche le dispatcher. Disabled/degraded conserve les intentions et l'accès HTTP sans fabriquer de statut published.
- Après échec du vrai transport, l'intention reste durable. Reconstruire utilise **la même DB et le même event_id**, pas une nouvelle notification pour cacher l'échec.
- Une lease SQL encore active peut rester jusqu'à expiration après arrêt local. La jointure locale n'est ni un rollback distant ni un déverrouillage durable.

## Jusqu'au droit HTTP

Le message SQS réellement reçu passe par le handler public de [[projects/sentinel-sync/sentinel-sync]], qui écrit le tuple `notification:<id>#recipient@user:<id>` dans OpenFGA. Le destinataire obtient 200 au marquage comme lu ; un autre utilisateur obtient 403 et ne modifie pas la ligne. Pas de fallback qui contournerait un refus du middleware OpenFGA.

La preuve finale rejoue les mêmes bytes avec un ledger neuf et contrôle un tuple unique ; elle ne démontre pas une déduplication de tous les événements IAM entrants. Voir [[projects/telegraph/references/telegraph-http-and-notification-api]].

## Ce qui n'est pas promis

Essais finis, pas exactly-once ni fencing multi-worker ; un dispatcher actif par DB dans le périmètre livré. Lease 30 s ≠ visibilité SQS 5 s du scénario IAM. Une absence de message n'est pas une absence d'état métier. Un PASS composant n'établit pas le parcours de droits complet.

## Related

- [[projects/rustycog/references/rustycog-outbox]] — primitive partagée.
- [[projects/aiforall/skills/proving-leased-outbox-recovery]] — échéance liée à l'état persisté.
- [[projects/aiforall/references/mission-telegraph-outbox-20261027]] — preuves et SHA livrés.
- [[projects/telegraph/references/telegraph-entity-model]] — lignes métier et intention d'infrastructure.
