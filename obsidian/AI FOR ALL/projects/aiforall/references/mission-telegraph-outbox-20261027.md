---
title: "Telegraph : outbox durable, arrêt HTTP et livraison du 27 octobre 2026"
category: references
tags: [reference, events, testing, architecture, visibility/internal]
sources:
  - _sources/opencode-telegraph-outbox-20261027/inventory.json
  - docs/contracts/telegraph-notification-outbox.md
  - docs/contracts/http-runtime-owned-shutdown.md
summary: "Livraison Telegraph : transaction commune, reprise après lease, HTTP possédé, preuves SQS/OpenFGA/403/200 et publication du SDK/pin, avec réserves explicites."
provenance:
  extracted: 0.96
  inferred: 0.04
  ambiguous: 0.00
created: 2026-10-27T19:27:11Z
updated: 2026-10-27T19:27:11Z
---

# Telegraph : outbox durable et livraison vérifiée

Le wiki est une synthèse secondaire : `docs/adr/` porte les décisions voulues avec leur statut, `docs/contracts/` les contrats et le code la réalité. Les ADR 0313 et 0503 restent **Proposed**, 0301 **Partial** ; cette livraison ne les promeut pas.

## Résultat livré

- Notification, livraison éventuelle et une intention `NotificationCreated` partagent la transaction writer ; publication directe post-commit supprimée. Voir [[projects/telegraph/concepts/durable-notification-publication]].
- Disabled/degraded NoOp suspend le dispatcher sans faux `published`, HTTP restant disponible.
- Propriétaire HTTP additif adopté par Telegraph standalone et le listener du monolithe ; port 0 et Extended CONNECT HTTP/2 corrigés. Voir [[projects/rustycog/references/rustycog-http]].
- Produit conservé : lease SQL 30 s, poll 2 s, backoff 5 s, maximum 20 essais. Aucun unlock aveugle, retry réinitialisé ou drain nouveau pour faire passer les tests.

## Preuves qui se complètent, sans se remplacer

- **Composant après expiration** : `post-expiry122-6412a9c40ae64bf4`, 1 PASS, 0 échec, 41,02 s libtest. Quinze cycles avant éligibilité, un après ; nouveau claim puis vrai message SQS. Observations terminées à 30,244 s avant l'échéance fixe 54,990 s. Comptes notification/livraison/outbox : **1/0/1**.
- **Verticale finale** : `vertical134-06bd9d7e772c4370`, 1 PASS, 0 échec, 0 ignoré, 6 filtrés, 25,39 s. Nominal et panne/reconstruction, vrais bytes SQS, handler Sentinel/OpenFGA sans grant préchargé, replay à ledger neuf, tuple unique, HTTP 403 sans mutation puis 200 destinataire. Comptes **1/1/1**.
- Dans cette verticale, E était déjà passée : R=0, D=25 s, observations finies en 266736 µs. Elle ne prétend pas avoir rencontré le verrou encore actif du composant.
- Revue indépendante du delta temporel/ownership : PASS **statique borné**, pas exécution de toutes les branches de timeout. Les rouges historiques restent conservés ; la cause exacte du timeout109 reste inconnue.

## Contributions des sous-agents

- L'architecte a distingué reprise durable après expiration et jointure locale en 6 s : aucune obligation de libérer immédiatement le claim SQL.
- La recherche documentaire a confronté le dépôt aux sources AWS, Tokio et PostgreSQL : annuler un futur n'atteste pas l'annulation d'un effet distant ; visibilité SQS et lease SQL ne sont pas le même mécanisme.
- L'auteur de tests a signalé qu'un timeout autour du démarrage de fixture pouvait perdre les handles nécessaires à la jointure. Le raccordement privé du test conserve explicitement parents et propriétaire HTTP, sans nouvelle API de production.
- Le reviewer indépendant a vérifié D non renouvelée, contrôle après dernier await, réception réelle, cleanup et conservation des droits/replay. Ses conclusions restent distinctes du résultat du run.
- Le lead a remplacé les livrables PM/inventaire non récupérables par un travail direct annoncé, sans inventer de résultat d'agent ou changer les modèles approuvés.

## Livraison et limites

Commit application **39024b2b445ae9e35f9ad148ea754995f8baab18**, poussé sur `Djoe-Denne/IAMRusty`, branche `mission-telegraph-outbox-integration`. Gitlink mode 160000 vers **d60a88b2f2f499768a807941a1b4c80c9a366b3f**, SDK publié sur `Djoe-Denne/rustycog`, branche `mission-telegraph-ac7-zlfadf`. SHA distants vérifiés ; ni force-push ni merge de branche principale.

Clippy strict antérieur non vert, Sonar/CI distante non exécutés, pas de déploiement. Aucun exactly-once, fencing multi-worker, backfill ou déduplication IAM entrante promis. Drop/abort ne prouvent pas une async join générale.

Campagne finale : quatre créations, trois retirées, un SMTP Exited(2) et deux volumes opaques conservés ASK, zéro fixture possédée active ; préexistants intacts. Desktop/WSL non arrêtés car workload étranger actif, RAM non mesurée.

## Provenance et portée de l'ingestion

Le bundle immuable contient **14 sources écrites**, empreintes et copies exactes : ledger de session, contrats, avis architecte, recherche, remise auteur, revue et rapports de preuves/livraison. **Aucun export exhaustif des sessions natives**, raisonnement caché ou log outil brut n'est revendiqué. Rapports 141 puis 142 : le premier décrit le commit encore local ; le second lève explicitement cette réserve par le push vérifié.

## Related

- [[projects/aiforall/skills/proving-leased-outbox-recovery]] — recette de preuve bornée.
- [[projects/telegraph/references/telegraph-http-and-notification-api]] — droits réellement observés.
- [[projects/aiforall/decisions/0300-events-authz]] — photographie canonique et évolution du code.
- [[projects/aiforall/aiforall]] — hub de plateforme.
