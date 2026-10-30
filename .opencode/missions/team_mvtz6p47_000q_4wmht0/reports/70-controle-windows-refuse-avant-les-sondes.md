# Contrôle Windows refusé avant les sondes : cause du lancement identifiée

Le premier lancement s'est arrêté avant les commandes Linux recherchées. Le
contrôle d'identité Windows en fin de processus n'a pas confirmé ses données.
Ce n'est ni un échec de `unshare` ou `ldd`, ni un nouvel échec de Chrome.

Une expérience distincte, avec un petit processus Windows connu, a reproduit le
refus de l'observateur composite après la fin. Le même handle conservé fournit
néanmoins le PID, la création et la fin du processus : cette autre lecture a été
vérifiée avec succès, sans WSL. Le parent temporaire utilise désormais ces faits,
avec SID/image observés avant exit et sans prétendre les avoir relus après exit.

Le premier bail reste documenté comme incomplet, pas converti en réussite. Deux
invocations de préparation/retrait ont été jointes ; les entrées Linux seront
inspectées avant toute nouvelle opération. AIForAll est observée arrêtée, les
ressources étrangères inchangées. Aucun effacement forcé ou arrêt global.

Une nouvelle opération bornée peut donc poursuivre les sondes, sous bail neuf,
sans répétition de commande `unshare`/`ldd` puisqu'aucune n'a encore été exécutée.
La limite analogue du conducteur durable est consignée séparément ; celui-ci et
la collecte privée restent non admis. HTTP et Chromium non qualifiés.

## Preuves

- Bail initial `.tmp/k6-linux-probes-run-20261030-2dedd5a876e34e3fa34eb480deb43680.json` :
  phases prepare/retire jointes, captures propres, zéro phase probes.
- Windows `verify-exit` : code0, handle/PID/création conservés, fin observée,
  observateur composite après exit refusé, `wsl_started=false`.
- Brief complet : `.cursor/review-briefings/20261030-k6-windows-ended-handle-observation.md`.
- Ces résultats constituent un progrès vérifié du diagnostic parent, pas une
  autorisation d'essais aveugles ou une mesure des capacités Linux recherchées.
