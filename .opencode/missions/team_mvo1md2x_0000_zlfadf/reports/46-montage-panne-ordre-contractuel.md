# Rapport 46 — décision : montage de panne conforme, sans vis5

## Nouvelle proposition et relecture spécialisée

L'auteur propose de garder l'app nominale active jusqu'à l'enregistrement du second IAM et de son intention failed, puis de reconstruire seulement pour la reprise de cette intention. Proposition reçue après la préparation de l'expérience visibilité45 ; le lead a demandé HOLD de cette exécution pour éviter deux expériences cumulées.

Le même emergency-thinker, session ses_e87132599ffe670zEfAua7YdHW, réévalue en lecture seule et recommande **cette unique variante**, sans vis5 ni sommeil de quiescence. AC2 exerce le publisher réel rendu indisponible pendant que l'entrée reste active ; AC3 exige reconstruction après intention durable sur la même DB ; AC5 conserve second IAM unique et chaîne réelle. La reconstruction additionnelle avant le second IAM n'était pas un oracle demandé.

## Décision locale bornée

Remplacer la proposition45 par un delta test-only d'ordre des phases, sans modifier la configuration broker, le SDK, le budget25s ou les assertions. Cette interprétation des oracles existants ne décide aucune nouvelle architecture ni nouvelle garantie. Si l'expérience45 a déjà été exécutée ou est en cours, ne pas en lancer une autre : rapport/cleanup d'abord, décision ensuite.

Sous condition qu'aucun troisième run n'ait démarré :
1. Nominal complet et sortie acquittée, garder app/consumer/dispatcher actifs.
2. Supprimer la seule destination outbound ; envoyer une fois le nouvel IAM. Avant stop, prouver 1notification/1livraison/1intention, capturer toutes identités/payload et event_id NotificationCreated (distinct de l'event_id IAM), puis failed+erreur réelle+budget non épuisé.
3. Stop/jointure/destruction de l'ancienne app et tâches selon borne existante ; restaurer seulement outbound ; reconstruire sur même DB, sans recréer fixture ni modifier lignes métier.
4. Même intention/payload/identités devient published, vrai message correspondant ; counts métier inchangés. Garder replay ledger neuf, tuple unique, HTTP403 sans mutation et HTTP200 destinataire. Aucun renvoi IAM ni tolérance aux doublons.

Un seul test vertical filtré et une seule validation bornée, après préflight/bail comme précédemment. Aucun rerun des cinq classifiers/suites déjà vertes par principe. Diff et preuves à remettre pour contrôle indépendant final.

## Limites obligatoires

Les deux échecs antérieurs restent archivés et inexpliqués ; aucun correctif SQS démontré. Le montage corrigé ne prouve ni absence de Receive distant après arrêt ni réception immédiate d'un IAM après reconstruction. SQLcommit n'est pas ACKIAM ; unicité de scénario n'est pas exactly-once. AC7 local reste distinct et inchangé. Même si le scénario corrigé passe, pas de causalité historique prouvée ni PASS global avant revue/intégration.
