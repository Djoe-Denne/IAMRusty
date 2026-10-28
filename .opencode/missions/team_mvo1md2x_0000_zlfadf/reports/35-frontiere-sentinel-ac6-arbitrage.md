# Rapport 35 — AC6 : défaut Sentinel reproduit, extension ciblée à décider

## Objectif et étape

Préserver AC1–AC7 sans affaiblir les oracles. T2 intégrée ; T3 a prouvé les scénarios runtime ciblés mais le parcours intégré reste non vert. L'autorisation long_run couvre les choix techniques locaux, pas une extension explicitement exclue par contrat.

## Changement et preuves vérifiées

Après le nom SMTP unique, la collision ne bloque plus la création. Run `t3-routing-2ebb73f602484bdf` : summary.json lu, routage exit101,1PASS(témoin)/1FAILED,0ignored ; HTTP/résilience non exécutés. Le lead a lu l'échec réel dans notification_created_routing_test.log789–790 : rejeu via handler, ligne212, OpenFGA400 `write_failed_due_to_invalid_input`, message `cannot write a tuple which already exists ... invalid write input`.

Source vérifiée dans le worktree auteur : workers/sentinel-sync/src/fga_client.rs239–269 délègue les erreurs à is_idempotent_tuple_error ;335–339 ne reconnaît que des sous-chaînes underscored, pas le message réellement retourné par OpenFGA1.5.0. L'auteur rapporte que la première écriture a réussi et que l'échec apparaît au rejeu du même événement avec ledger neuf. Le replay ne doit pas être supprimé ou réduit à un simple hit du ledger mémoire pour masquer le défaut.

Contrat vérifié §6/ligne125 : **aucun changement production Sentinel**. Corriger ce classifier dépasse donc une frontière explicite, contrairement aux modifications locales de fixtures déjà autorisées. Pas d'édition Sentinel faite, aucun changement de statut ADR proposé implicitement.

## Résultats et ressources

Quatre succès runtime antérieurs restent valides dans leur portée ; ce n'est pas un PASS global. Ce run crée4IDs, owned_running[], unknown_new_ids[], avant2/après3 : node40201...running intact et deux SMTP arrêtés, mission-owned historiques retenus ask (26b3... et eeb835...). Deux nouveaux volumes opaques dans ce run, six au total T3, conservés ask. Pas de suppression forcée, volume ou adoption par nom. Cleanup/bail de ce run terminé ; les autres cibles restent absentes.

## Options, coûts et recommandation

1. **Recommandé : autoriser un sous-lot Sentinel minimal** sur le classifier de duplicats et ses tests, sans nouvelle API publique, backend, harness, changement du ledger ou refonte globale. Témoins positifs de la réponse FGA réelle et formats historiques ; négatifs garantissant que les autres400, erreurs authentification/autorisation, indisponibilité ou erreurs ambiguës restent propagées. Ne pas traiter tout400 comme succès. Amendement ciblé du contrat par architecte après autorisation. Rejouer l'oracle AC6 inchangé puis seules preuves impactées ; revue indépendante bornée sur ce nouveau risque de faux succès. Coût : diff et tests supplémentaires nécessaires, pas de campagne globale imposée.
2. **Ne pas étendre le périmètre** : conserver le défaut reproduit et la preuve rouge, rendre la mission bloquée/incomplète pour AC6. Aucun abaissement silencieux des garanties ni succès global déclaré. Les vérifications déjà autorisées et indépendantes peuvent avancer, mais ne lèveront pas AC6.

## Décision demandée

Autoriser ou refuser cette seule extension de production Sentinel/classifier+tests, en conservant AC1–AC7 et l'oracle de replay réel. Rien n'autorise une refonte générale, un nouveau transport, une déduplication IAM ou exactement-once. En attendant, pas d'édition Sentinel ; HTTP/résilience indépendants restent dans le périmètre initial et peuvent être exécutés sous bail borné/guards existants.
