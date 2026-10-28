# Rapport 47 — troisième expérience recoupée, calibration recevable sous revue

## État réel et décisions précédentes

L'expérience45 avait été exécutée avant réception du HOLD/comparaison46. Aucun quatrième run ni correctif SQS n'est déclaré. L'alternative d'ordre46 n'a pas été exécutée : elle reste une proposition, pas une preuve à substituer au résultat réel.

## Preuves recoupées par le lead

Run `t3-exp3-6125e6e3d1d54e21` : summary et observations.json lus via Context Mode, marqueurs du log vérifiés. Test vertical seul1PASS/0failed/0ignored/6filtered,exit0,issues[],35.30s.

Visibilité entrante initiale30s/délai0,5s relues avant première app. Second IAM invisible aux observations0/1/3s, plus invisible à6/12s ; même event_id IAM8c3b35f0-8b44-4fd6-96d6-5e226299bd81 reçu5.75s après envoi, commitSQL5.807s. Nominal111ms. Aucun nouveau Receive de diagnostic ni renvoi IAM selon source auteur ; observations des Receives déjà présents = IDs[] sur les deux phases.

Marqueurs nominaux/après panne réels : NotificationCreated.event_idc0b6ecfc-5363-4f6a-b63d-73393cef3389 puis719315eb-5ea5-4152-98c3-63c4be2b7212 ; chaque published/tuple1/HTTP403autre/200destinataire/is_readtrue. Rejeu ledger neuf et assertions de corrélation/identités demeurent dans le test. Ce succès est **conditionné à la calibration entrante5s**, pas au défaut30s ni à une annulation distante prouvée.

Cleanup summary :4créés,zeroownedrunning/unknown_new[],avant7/après8 ; node non possédé intact, nouveau SMTP6e377c...exited retenuask. Rollup47créés/40retirés/7SMTPask/16volumesopaquesask annoncé par auteur, pas recompté globalement ici. Aucun cleanup automatique de ces retenus.

## Jugement documentaire séparé du résultat vert

Même architecte `ses_e87561177fferReewD4GYQX2c3` juge recevable sous conditions : calibration explicite de redelivery sur fixture réelle, pas augmentation25s ou réduction d'oracles métier/outbox/droits ; AC n'imposent ni visibilité30s ni absence de redelivery. Il ajoute§6.2 au contrat, sans code ou runtime, historiques préservés et gate lead+T4.

Lead vérifie le diff et intègre **un seul document** au commit `fad885a1fbe1c04240f590b28f389509a4d895a9`,+10lignes, diffcheck et commit normal réussis. Copie locale T3 `.tmp/t3-inputs/contract-inbound-calibration-approved.md`, hashSHA256bab685ada8e968f1ae95328aaef9020afe7c2236fa449e13703099589049526a, égalité source/copie vérifiée.

Décision lead : retenir ce réglage dans le **candidat à intégrer/revoir**, pas réception finale. Pas de quatrième run ni conversion des deux rouges historiques en succès. T4 doit vérifier indépendamment diff, chronologie, budgets, non-masquage et frontière des garanties ; un finding bloquant restera bloquant. Aucun claim causeSQS, SLA5s, exactly-once ou AC7distant. Les diagnostics/échecs historiques restent documentés.

## Suite bornée

Même auteur T3 prépare remise et commit avec hook normal ; pas de nettoyage opportuniste de code qui invalide le snapshot de preuve. En cas d'erreur206, réutiliser le checkout physique court et contrôles de snapshot déjà validés en T2, pas nouveau subst ni bypass. Intégrer avant T4/T5, puis publier ces branches indépendantes en parallèle. Attention : HEADlead contient les amendements documentaires09a59ee/fad885a absents de la base auteur ea2c009 ; l'intégration du code ne doit ni les écraser ni prétendre l'identité entière des arbres auteur/lead. Comparer les blobs de code livrés et préserver le contrat intégré.
