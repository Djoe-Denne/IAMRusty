# Format fatal confirmé ; fin de la série de reconnaisseurs

## Résultat et conséquence

La mission cherche la cause du refus de Chromium Linux sans exposer ses journaux
natifs. Le nouvel essai confirme sur Ubuntu 22.04 un message complet au format
fatal Chromium sur stderr, avec stdout vide, puis un SIGABRT primaire observé
avant cleanup. La classification a donc levé l'ambiguïté « aucun texte ou texte
non reconnu », mais elle ne révèle ni emplacement source ni cause.

Les **140 tests Linux réussissent sans skip sur les deux plateformes**. Les outils
et inspections passent ; les contrôles d'arrêt des enfants réussissent. Le
navigateur reste non qualifié et le job échoue comme attendu. L'API confirme zéro
artefact publié. Sur24.04, le message de sandbox indisponible et le refus de
confidentialité sont conservés ; les nouvelles facettes sont retenues.

## Décision immédiate

La règle d'arrêt est atteinte : **pas de nouveau reconnaisseur ni de répétition
identique**. Aucun correctif de Chromium, argument, pin, politique AppArmor ou
critère de qualification ne peut être déduit de ce seul format fatal.

Le source Chromium versionné a été relu après cet essai : le traitement fatal peut
passer par un gestionnaire ou une instruction de crash ; d'autres chemins de
rupture emploient abort. Le format et le signal ne désignent pas un appelant unique,
et les flux peuvent être hérités par des descendants. Aucune causalité exclusive
ou origine primaire du message n'est prétendue établie.

## Prochaine enquête autorisée, sans nouvel essai

La proposition architecturale reste sans emplacement pertinent connu. Avant
d'invoquer une absence de permission, le lead a vérifié le canal existant : le
dépôt GitHub est **PUBLIC**. Ses journaux Actions ne constituent donc pas un canal
opérateur privé. Aucun site inconnu, texte natif, hash opaque ou corps de message
n'y sera envoyé dans le mandat actuel.

L'architecte doit maintenant borner la plus petite option concrète : relevé
structuré privé sur un hôte Linux autorisé, ou autre solution réellement compatible.
Cette consultation ne lance aucun runtime ni collecte, et ne ratifie pas le
contrat proposé. Le lead poursuit les mises à jour documentaires indépendantes ;
aucune nouvelle demande de GO de routine n'est instaurée.

## Annexe technique

- [Run40985953377](https://github.com/Djoe-Denne/IAMRusty/actions/runs/40985953377),
  HEAD publié `6c27e17fb9da8b55749d6d9370a01c7672fa0a15`, source `1c0036f`.
- Tests24 :140/1.173s/OK ; tests22 :140/2.062s/OK, sans skip. Deux jobs failure,
  acquisition/version/inspect réussis. `browser`22 exit2, `diagnose-chromium`22
  exit2 ; direct24 exit1/PRIVACY_FAILED. Aucun dispatch ou retry.
- Direct22 corrélé au `run_id` k6 courant : `direct_text` v1
  stdoutEMPTY/stderrPRESENT/CHROMIUM_FATAL_REPORTED ; `direct_stop` v1
  NATIVE_OBSERVED_BEFORE_CLEANUP/SIGABRT, confidentialité et enfants cleanup PASS.
- K6 des deux côtés : SCRIPT_ABORTED, expected1/started0/completed0/iterations1/
  requests0, summaryVALID ; aucun renderer observé, namespaces NOT_OBSERVED.
- HTTP et selftests live non exécutés, cible conforme toujours absente ; Chromium
  et cleanup runtime parent INCONCLUSIVE. Aucun runtime local créé/redémarré.
- API artifacts exit0 : total_count0/retournés0. Visibilité du dépôt lue via
  `gh repo view --json visibility,isPrivate,nameWithOwner`, exit0/PUBLIC/false.
- Sources recoupées après le run : Chromium156 `base/logging.cc:975–1018` et
  `base/debug/debugger_posix.cc:257–290`, sans attribuer leur branche à ce run.
