# SIGABRT confirmé avant cleanup ; localiser la cause sans changer les protections

## Résultat nouveau

Le direct22 rapporte maintenant **SIGABRT**, avec statut primaire observé avant cleanup. Cette observation confirme la distinction de contrôle établie au rapport31. Elle ne révèle ni l'émetteur du signal, ni la raison de l'arrêt : un abort de bibliothèque ou du programme est compatible, pas prouvé.

Les129tests Linux passent sans skip sur chacune des deux plateformes. Les outils, versions et inspections réussissent ; k6 reste arrêté avant le cas avec SCRIPT_ABORTED. Les enfants sont arrêtés correctement, aucun PASS Chromium/HTTP ou cleanup parent n'est acquis.

## Recherche et contrôle statique après l'essai

Les recherches spécifiques SIGABRT renvoient plusieurs familles non équivalentes, dont des incidents d'autres versions, des erreurs d'initialisation ou des assertions. Aucun n'établit notre cause. Pas de changement GPU, Crashpad, sandbox, pin ou paquet sur cette seule ressemblance.

Une piste de permissions d'extraction a été examinée indépendamment : le provisionneur copie tous les membres de l'archive et préserve le caractère exécutable pour leur propriétaire, en retirant les droits étrangers. Les métadonnées centrales de l'archive officielle versionnée montrent chrome, crashpad_handler, chrome_sandbox, libEGL et libGLESv2 exécutables ; ICU, resources et snapshot sont lisibles. Aucun défaut de perte de bit exécutable n'est identifié dans ce chemin statique. Ce contrôle de métadonnées n'est pas une mesure des fichiers sur le runner ni une nouvelle vérification SHA256 de l'archive complète.

## Décision de suite

Ne pas répéter le même essai. Transmettre le nouveau SIGABRT à l'expert retenu, afin de recommander une observation fermée qui fournisse un vrai discriminant de cause. Le libellé actuel « non classé » confond éventuellement absence de texte et texte non reconnu ; ce point doit être examiné avant une longue liste spéculative de messages. Aucun élargissement de collecte, publication de flux brut ou changement de contrôle n'est autorisé par ce suivi.

La qualification reste ouverte et la mission non clôturée. La cible HTTP conforme manque toujours ; les propositions temporaires sont sauvegardées, pas supprimées/archivées.

## Annexe technique

- Publication vérifiée `6b5810844f51af1e20e38797288c2440468d6d4f`, source783fa80 ; push exit0/ls-remote identique.
- Run [40982576736](https://github.com/Djoe-Denne/IAMRusty/actions/runs/40982576736), même HEAD/push ;129tests22/0.915s/OK et129tests24/1.938s/OK, sans skip. Jobs123825903790/123825903986 failure, pas de normalisation en succès.
-22 : k6 summaryVALID/counters1/0/0/1/0/SCRIPT_ABORTED, privacy/enfants cleanupPASS ; direct corrélé au run_idk6 courant, direct_stopv1={path:NATIVE_OBSERVED_BEFORE_CLEANUP, signal:SIGABRT}, SANDBOX_UNPROVEN/exit2, privacy/enfants cleanupPASS. Parent runtime INCONCLUSIVE.
-24 : refus pré-cas, directNoUsableSandbox/SIGNAL/PRIVACY_FAILED ; nouvelle facette retenue à cause du refus privacy, comme prévu. Upload skipped.
- Extraction lue avec Serena sur la racine canonique : provision-linux.py:40–66, boucle complète, chmod0500 si exécutable sinon0400. Recherche exacte complète cette localisation ; aucun code modifié.
- Archive officielle : métadonnées seules via deux réponses HTTP206 bornées, tail65536octets et répertoire central33038octets,309membres ; aucune extraction, sauvegarde ou exécution. Membres ciblés et droits comparés au chemin du provisionneur, pas preuve runtime.

Sources complémentaires : archive Google versionnée du lock, code provision-linux.py, recherches SIGABRT/Chrome Linux et sources officielles déjà recoupées au rapport31. Les incidents d'autres versions et les propositions désactivant une protection ne sont pas repris comme remèdes.
