# Rapport 60 — appel shell architecte interrompu et vérifié

Après nouvelle demande explicite de l'utilisateur et capture montrant l'outil « En cours », le lead retrouve le binaire CLI OpenCode dans le chemin du service installé. API de session consultée : appel shell `call_3babd9e3de484610937cc85460e4b018`, session architecte `ses_e87561177fferReewD4GYQX2c3`, bien `running`, entrée correspondant au script0503. Aucune permission en attente retournée par l'API. Le défaut portait sur l'exécution d'outil encore marquée active, pas sur un PID PowerShell prouvé possédé.

Opération native ciblée `POST /api/session/ses_e87561177fferReewD4GYQX2c3/interrupt?resume=true` réussie : `interrupted:true`. Vérification indépendante suivante : **même callID, status error, type aborted, Tool execution interrupted**. Aucun arrêt OS arbitraire, kill du service OpenCode, terminal Ollama ou MCP.

ADR0503 et contratHTTP toujours absents après interruption : ne pas annoncer une rédaction livrée. Notification de cancellation du sous-agent reçue, cohérente avec cette interruption. Reprendre la même session avec les instructions actualisées : patchs dédiés, quatre documents canoniques, E1 rouge déjà intégré b291eee. Pas relance du gros shell, doublon d'auteur, correctif SDK ou baisse d'AC7.
