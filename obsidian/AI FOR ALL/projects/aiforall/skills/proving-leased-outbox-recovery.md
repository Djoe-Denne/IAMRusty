---
title: Prouver une reprise outbox après lease sans affaiblir l'oracle
category: skills
tags: [testing, events, rust, architecture]
sources:
  - _sources/opencode-telegraph-outbox-20261027/inventory.json
  - .cursor/qa-plans/20261027-vertical-post-expiry134.md
summary: "Séparer lease SQL, visibilité SQS et arrêt local ; fixer une deadline unique depuis l'éligibilité stockée, garder les owners et vérifier le vrai transport."
provenance:
  extracted: 0.88
  inferred: 0.12
  ambiguous: 0.00
created: 2026-10-27T19:27:11Z
updated: 2026-10-27T19:27:11Z
---

# Prouver une reprise outbox après lease

Recette distillée de la mission Telegraph, pas un nouveau SLA produit. Voir [[projects/aiforall/references/mission-telegraph-outbox-20261027]] et [[projects/rustycog/references/rustycog-outbox]].

## 1. Séparer les horloges et les frontières

- Lease SQL : autorise ou interdit une nouvelle prise en charge durable.
- Visibilité SQS : masque un message déjà reçu par un consommateur.
- Grâce/jointure locale : borne les tâches que le caller possède ; ne retire pas un effet distant déjà envoyé.
- Dans Telegraph, 25 s après reconstruction ne couvraient pas toute lease légale de 30 s. Corriger cette incompatibilité de preuve ne nécessite pas de raccourcir la lease.

## 2. Fixer la borne une seule fois

1. Après arrêt/restauration, capturer `t0` monotone et UTC **avant** lecture/factory/reconstruction.
2. Lire la même intention sous bootstrap borné : publishing utilise `locked_until` avec propriétaire ; failed/pending utilise `next_attempt_at` sans lock incompatible. Déjà published : résiduel nul, mais message réel toujours requis.
3. Vérifier identité/payload/version, essais restants et cohérence des horloges. Un état manquant/incompatible ou un résiduel hors plafond est une précondition en échec, pas une invitation à élargir le délai.
4. Pour le protocole autorisé : `R=max(0,E−UTC(t0))`, `R<=30s`, **D=t0+R+25s<=t0+55s**. Ne jamais refaire partir le chronomètre après un retry ou la factory.
5. Reconstruction, attente published, vraie réception SQS, validation des bytes et comptes SQL partagent D. Contrôler strictement `completed_at < D` **après le dernier await** : `timeout_at` seul peut rendre un futur Ready repollé trop tard.

La tolérance UTC/monotone de 100 ms est une précondition de mesure, pas du temps ajouté. IAM garde sa limite propre de 25 s ; la chaîne de droits ultérieure n'est pas faussement revendiquée dans D.

## 3. Garder les moyens de nettoyer

Un timeout autour d'une fixture qui cache ses JoinHandle peut ordonner abort tout en empêchant la jointure observée. Le test garde parents et [[projects/rustycog/references/rustycog-http|propriétaire HTTP]] hors des futures annulables ; erreur/timeout nettoie via les helpers existants sous les mêmes 6 s, avant retour/panic. Pas de backend ou fixture partagée nouvelle pour contourner le problème.

## 4. Choisir des preuves discriminantes

- Avant expiration : cycles réellement commencés/terminés, absence de reclaim précoce, propriétaire/verrou et identités suivis.
- Après expiration : nouveau claim/tentative observés, succès du vrai transport, statut published et message réellement reçu corrélé, comptes métier inchangés.
- Verticale : handler/OpenFGA réel sans grant préalable, replay mêmes bytes à ledger neuf, tuple unique, 403 sans mutation et 200 destinataire.
- Vérifier la date future dès la première observation FAILED, pas après un sampler indépendant de 12 s.
- Ne pas confondre ID de campagne trop long, panne d'environnement, rouge fonctionnel et preuve simplement absente.

Les tests avant et après expiration se recouvrent partiellement mais ne sont pas supprimables sur leur seule taille : comparer leurs oracles, notamment ancien propriétaire/verrou, avant consolidation. ^[inferred]

## 5. Réutiliser et rendre précisément

Windows natif, un seul Cargo, 24 jobs configurés ; fixtures testcontainers existantes, aucun Kind pour cette IT. Registre contemporain par IDs, baux et inventaires avant/après ; zéro fixture possédée active à la fin, SMTP nonzero/volumes opaques ASK conservés. Un PASS de source/compilation n'est pas un PASS de run. Réutiliser les preuves identiques au lieu d'une nouvelle campagne pour chaque rapport.

Pour une équipe, séparer remise auteur et livraison intégrée, publier le consommateur de revue après intégration, transmettre la correction de contrat par écrit et ne pas inventer l'héritage d'une session. C'est un moyen de réduire la redécouverte sans masquer les limites d'outil. ^[inferred]

## Related

- [[projects/telegraph/references/telegraph-testing-and-smtp-fixtures]] — application concrète et fidélité des protocoles.
- [[projects/aiforall/concepts/local-runtime-lifecycle]] — baux, propriété et arrêt du runtime local.
