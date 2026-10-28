# Rapport 45 — expertise : visibilité SQS, troisième expérience discriminante bornée

## Retour spécialiste, pas verdict de cause

emergency-thinker session `ses_e87132599ffe670zEfAua7YdHW`, lecture seule et aucun Cargo/Docker/édition, examine sources réelles et logs. Hypothèse prioritaire : ReceiveMessage ancien annulé localement mais encore actif côté LocalStack, qui réserverait le second IAM sans nouveau handler. Stop consumer→SendACK secondIAM :125.5ms dans une fenêtre longpoll configurée2s. La fixture ne fixe pas VisibilityTimeout ; si supérieur aux25s d'oracle, une redelivery légitime pourrait dépasser l'attente. Valeur réelle antérieure non capturée : hypothèse non prouvée.

Ce n'est pas une réouverture RP1 : destruction locale des tâches n'implique pas l'annulation d'effets réseau distants, limite contractuelle déjà explicite. Alternatives : nouveau Receive transport bloqué, endpoint différent, traitement abandonné avant handler. SilenceINFO ne tranche pas.

Instrumentation autour du vrai Receive dans SDK serait une nouvelle surface non autorisée. Le lead ne l'autorise pas implicitement. À la demande du lead, le même spécialiste propose une variante strictement test-only avec API SQS de fixture existante.

## Expérience unique retenue — sans SDK ni nouveau harness

Troisième expérience autorisée, après expertise, sur hypothèse nouvelle étayée et bornée :
1. Avant première app, lire les attributs de la seule entrée possédée, fixer VisibilityTimeout5s par API existante puis relire. Capturer valeur initiale ; si déjà5s, intervention non distincte, arrêter et rendre inconclusif plutôt que répéter.
2. Transition nominal/stop/suppression seule sortie/nouvelle app/secondIAM une fois inchangée. Aucun sommeil ou appel diagnostique entre stop et secondSend.
3. Quelques snapshots passifs des attributs visible/invisible/delayed aprèsACK, par exemple0/1/3/6/12s, dans la deadline GLOBALE25s existante et requêtes bornées. Pas de Receive diagnostique supplémentaire, purge/reset/renvoiIAM, tâche détachée ou instrumentation SDK. Journaliser seulement IDs issus des Receives déjà présents si utile.
4. Même test vertical filtré, unicité métier, reprise même event_id, vrai handler/ledgerneuf et HTTP403/200 inchangés ; seules preuves impactées, pas rerun des suites vertes.

## Interprétation obligatoire

- Invisible, aucune notification, puis réception du même IAM et commit autour de la fenêtre raccourcie : corroboration de visibilité/redelivery, pas attribution certaine à l'ancien Receive.
- Visible mais aucune progression : visibilité ne restaure pas, prioriser consumer/endpoint/transport.
- Vert immédiat sans invisibilité observée : cause inconclusive, ordonnancement peut avoir bougé.
- Échec ou doubles créations : rouge conservé, pas absorption du défaut.

Attributs approximatifs, pas de lease individuel identifié. Même succès ne prouve ni absence de bug local ni correction permanente valide. Pérenniser le réglage demanderait jugement séparé de contrat/preuves, pas simple PASS. Pas de sommeil2s de quiescence qui masquerait l'entrelacement.

## Bail et suite

AuteurT3 reste seul écrivain ; un seul run diagnostic autorisé sous préflight frais et bail uniquement nouvellesfixtures, uniqueCargo24jobs. Protéger node et sixSMTPhistoriquesask,14volumesopaquesask annoncés ; stop/cleanup fin même échec et aucun rm2/force/volume. Rendre preuves et attribution, puis décision avant autre run ou changement production. Aucun troisième run exécuté par le lead ; aucune extension SDK/contrat fonctionnel.
