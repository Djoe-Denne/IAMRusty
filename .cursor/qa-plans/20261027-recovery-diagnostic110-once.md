# Diagnostic110 unique — frontière arrêt/reconstruction

Autorité : réponse utilisateur après rapport109, contrat110. Expert hard-implementer fresh, `team_handoff({phase:'implementation',reasoning:'high'})` accepté avant analyse substantielle, enquête strictement RO. Source AI1f2 + test6ac7be0e, SDKd60 propre/pin129. AC1–7, ADR0313/0503Proposed et0301Partial inchangés. Plan intégré par lead avant instrumentation.

## Faits et inconnues

Échec109 : mark_failed retourne1 vers08:43:47.500Z. Ancien dispatcher retrouve un candidat à53.528Z, stop demandé53.548Z, UPDATE de claim loggé53.549Z avec0rows. Nouveau worker démarre53.719Z, treize sélections candidates vides jusqu'à expiration du waitPUBLISHED25s, tandis que les lectures par aggregate_id retournent1. Le template de mise à jour et les0rows loggés lors de l'annulation ne prouvent pas l'état persistant.

Hypothèse : claim persistant, puis annulation avant publish/mark, laissant publishing/ancienworker/lock30s au-delà du contrôle. Alternatives : next_at futur, plafond20, verrouNULL/incohérent, identité/DB/horloge/sélection. Pas de changement de production pour départager ces hypothèses.

## Préparation test-only

Propriétaire : même test-engineer pool14/session existante. Préférence d'ownership : seul test vertical, instrumentation locale temporaire ; si une lecture doit être instrumentée dans le helper fixture, justifier et limiter à ces deux fichiers tests, sans mutation de production ou API applicative. Aucun nouveau harness d'exécution/dépendance/feature.

1. Réutiliser les lectures du premier FAILED et de la ligne failed avant stop. Tracer les champs sûrs, pas de nouvelle lecture/await entre pré-stop et stop. Marqueurs synchrones stop_begin/stop_return/rebuild_begin/rebuild_return/published_wait_begin/deadline.
2. Une seule lecture SQL supplémentaire après retour réussi du stop. Projection SQL uniquement sûre, sans payload/metadata/last_error brut ; pas FOR UPDATE ou transaction maintenue. La faire progresser dans une future locale possédée EN CONCURRENCE avec le futur CreateQueue déjà prévu. Si restauration termine d'abord, drop l'observation et continuer immédiatement ; si lecture finit d'abord, garder son résultat puis poursuivre le MÊME futur restauration. Pas de join imposant d'attendre l'observation après restauration. Borne propre courte100ms, pas nouveau budget d'assertion ; absence/cancel/queryerror sont observations manquantes, pas nouvelles assertions de succès ou d'échec métier.
3. Première lecture déjà prévue dans waitPUBLISHED après rebuild = premier état de reprise. Tracer changements du tuple et au plus une ligne/seconde à tuple stable. Garder dernier échantillon/compteur en mémoire ; au timeout imprimer ce dernier état AVANT conserver le MÊME échec. Aucun SELECT après deadline ou réinitialisation/allongement des25s. Une instrumentation locale de la même boucle de lecture doit conserver exactement son comportement hors traces ; ne pas changer les autres consommateurs du helper.
4. Champs : stage/sequence, aggregate_id/outbox.id/event_id,status,attempts,next_attempt_at,locked_by,locked_until,updated_at ; error_present/error_nonempty booléens seulement ; UTC début/fin de lecture et monotonic commun, durée/deadline publiée. Pas Debug du Model/erreur/connexion, payload/JWT/secrets.
5. Aucun snapshot figé, healer SQL, spawn/détachement, retry automatique, probe réseau/Receive ajouté ou nouveau budget. Préserver toutes assertions, sampler0/1/3/6/12, visibilité5, backoff5/exponentiel/plafond20/verrou30, budgetstop6s et sourceproduction.

## Preuves et gate de lancement

Auteur peut fmt et compiler cible --no-run sous préflight/bailCargo Windows24 unique ; AUCUN test exécuté/runtime à l'étape auteur. Rendre diff contre correction6ac7be0e ET contreAI1f2, sources/patchhash, constance de la boucle25s/sampler/replay/droits et configuration effective. Lead inspecte/archives avant préflight/bail frais et UNrun110 via lanceur existant adapté à une seule cible. Pas de NoOp/redondance, pas de retry jusqu'au vert.

## Interprétation préétablie

- Même ligne publishing/ancien locked_by, attempts augmenté, updated_at à frontière, locked_until après deadline : soutient claim abandonné, pas heure exacte de commit prouvée.
- failed/pending et next_at après deadline : backoff ; attempts>=20 : exclusion plafond ; publishing+locked_untilNULL : non-éligibilité, pas simple attente30s.
- Éligibilité durable sur plusieurs lectures, attempts<20 et horloges cohérentes malgré sélections vides : piste DB/pool/horloge/sélection à examiner ensuite sans réparation ici.
- Nouveau worker/attempts ou failed après reprise : dispatcher travaille ; corréler aux logs existants sans erreur brute.
- Vert seul n'explique pas109 : instrumentation peut changer l'ordonnancement. Observation absente/contradictoire = inconclusif, jamais PASS causal synthétique.

## Ressources

Inventaire109 est historique :4créés/3absents/1SMTPExited2ASK/2volumesASK/0ownedrunning/Cargo0,11préexistants préservés. Avant run110 nouveaupréflight fullIDs/baux ; créations fraîches tracées au registre, cleanup même échec et zéro possédé actif. Ressources étrangères/volumesASK hors bail, aucun arrêt global Docker/WSL.
