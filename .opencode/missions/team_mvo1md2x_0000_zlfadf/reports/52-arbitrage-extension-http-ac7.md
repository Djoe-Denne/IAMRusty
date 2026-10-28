# Rapport 52 — arbitrage de périmètre HTTP nécessaire pour fermer AC7

## Objectif et état

Conserver AC1–AC7 intégralement et ne pas clôturer sur une garantie incomplète. CodeT3 intégré aa400f3, preuve outbox/transport/FGA/HTTP/calibration5s examinée ; T4 indépendante rend **BLOCK C-1 HIGH**, confidenceHIGH. T5docscandidat staged, non intégré, hook206 mécanique connu ; pas de nouveau test ou code appliqué pour cette décision.

## Défaut et preuves

Finding final persisté avant correction : `.cursor/review-briefings/20261026T1704Z-correctness-telegraph-outbox-t4-aa400f3.md`, index mis à jour. Standalone app.rs175–179 et monolithe runtime.rs96–100 abandonnent serve_router par selectstop. SDK builder.rs268–275 appelle axum::serve0.8.9, dont les connexions sont spawnéesséparément. Joindre ce parent n'assure pas la destruction du handler async bloqué, qui peut continuer et muter après retour d'arrêt. Contrat97/115–117 exige explicitement absence d'orphelinHTTP et deadlineglobale6s. Analyse statique étayée, **aucun témoin rouge HTTP exécuté encore**. Ce n'est ni un résultat de test inventé ni une annulation distante exclue parcontrat.

Autres surfaces examinées par T4 sans défaut certain établi : atomicité writer, no-op, supervision horsHTTP, classifierSentinel/négatifs, vrai parcoursFGA/HTTP/replayfreshledger, calibration5s bornée etSMTPownership. Ces résultats ne neutralisent pas C-1 et ne constituent pas une revue sécurité/perfglobale.

## Avis architecte — aucune implémentation effectuée

Même architecte consulté en lectureseule : aucune solution T3 seule démontrée sûre avec APIactuelle sans recréer un runtimeHTTP parallèle. Graceful seul peut attendre indéfiniment ; middleware qui annule le handler ne prouve pas jointure des connexions/bodies ; handleaxum-server timeout seul ne prouve pas nonplus jointure complète.

Recommande une **extension strictement bornée rustycog-http**, distincte de l'extension autorisée rustycog-events/SQS et de l'exceptionSentinel : propriétaire HTTP additif sur pileAxum/Hyper existante, contrôle admission/grâce/annulation/jointure observable de descendants locaux ; préserver consommateurs existants de serve_router et adopter seulement Telegraph standalone/monolithe. Couvrir les connexions, tâches requêtes/stream/body/exécuteurHTTP2 et handshakes utiles aux protocoles déjà supportés ; pas désactivation silencieuse TLS/mTLS/dual-bind ou nouveaubackend/forkAxum.

## Options, coûts et recommandation

1. **Recommandé : autoriser le sous-lot HTTP SDK minimal**, tests ciblés puis publication/revue/pin exact. Surface SDKhttp(builder,exports/moduleprivé etmanifest strictementnécessaire à pileexistante), raccordementTelegraphsetup/runtime+monolithe et fixtures/tests existants. API additive d'ownership, aucune refonte métier/outbox/authz/schema, backend ou harness nouveau. Première étape : témoin rouge TCPloopback sansDocker sur handlerasyncpending+RAII via chemin réel, puis correction et verts destruction avant retour/absenceactivitétardive/budget6s, requêtegracieuse et bloquée, streams/protocoles supportés pertinents, standalone ethost. Contrat ciblé/ADRforwardProposed pararchitecte après accord ; aucune ratificationautonome. CodeSDKisolé, SHA revu publié avant gitlink. Coût/risque : vrai sous-lot concurrent partagé, pas un changement cosmétique ; vérificationindépendante bornée indispensable, pas quatreagents imposés ni recampagneglobale automatique.
2. **Ne pas étendre** : préserver code/preuves et rendre la mission bloquée/incomplète surAC7. Ne pas réduire la garantie à parentjoint ni annoncer succèsglobal. Les livrables métier déjà prouvés demeurent dans leur portée.

## Décision demandée

Autoriser ou refuser cette seule extension HTTP SDK+tests+publication/pin, AC1–AC7 inchangés. Tout besoin de nouvellepile/backend/harness, APIgénéralepluslarge ou désactivation de protocole impose un nouvel arbitrage, pas extensionimplicite. Aucun quatrième run historiqueSQS etaucun changementclassifierSentinel requis par C-1. Dernières fixturespossédées arrêtées, node nonpossédé et retenusask préservés ; aucune ressource nouvelle créée par cette analyse.
