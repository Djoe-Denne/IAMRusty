# Les espaces de noms fonctionnent ; dépendances encore non établies

Le diagnostic a enfin exécuté les deux commandes prévues, une seule fois chacune,
sans navigateur ni k6. Il apporte un résultat utile sur la piste utilisateur :
**la création et le mapping d'un espace de noms utilisateur réussissent dans le
contexte supervisé**. Le blocage spécifique décrit dans le ticket Claude Code
ne s'est pas reproduit ici. Cela ne prouve pas tous les mécanismes de Chrome.

## Mesures vérifiées

Parent et enfant : Ubuntu24.04, UID/EUID/GID1000, noyau
`6.6.87.2-microsoft-standard-WSL2`, seccomp0, filtres0, NoNewPrivs0. AppArmor0 ;
limite max_user_namespaces47785 ; les deux autres paramètres userns/AppArmor lus
sont absents, pas égaux à zéro. Les identités de namespaces user/pid/mnt observées
sont identiques ; session, groupe de processus et cwd diffèrent comme prévu.
L'enfant utilise DEVNULL, le parent PIPE.

- `unshare --user --map-root-user -- /usr/bin/true` : sortie0, terminaison normale.
- `ldd` du binaire vérifié : sortie0, mais la sortie n'a pas été reconnue complète
  par le parseur strict ; **nombre de bibliothèques absentes inconnu**, pas zéro.
- Confidentialité, nettoyage des enfants et retrait du répertoire de run : PASS
  à leur portée. Résumé allowlisté et schéma reconfirmés sous Windows avant lecture.
- Script et parent : sortie2, `DIAGNOSTIC_INCOMPLETE`. Aucun PASS global.

Les commandes n'ont pas été exécutées dans un vrai terminal utilisateur. Les
contextes sont comparés, pas deux exécutions A/B des commandes.

## Nettoyage et suite autorisée

Les quatre invocations Windows possédées ont été jointes et vérifiées par le même
handle, PID et création ; SID/image observés avant exit, aucune fraîcheur SID
après exit inventée. Les cinq fichiers publics temporaires et leur arborescence
sont retirés. AIForAll est observée arrêtée au contrôle final, sans stop explicite.
ARTIST reste arrêtée, docker-desktop reste actif et étranger. Données/outils et
clone d'origine conservés ; RAM globale non mesurée.

La limite du parseur sera examinée par lecture statique de ses branches et de
l'ELF déjà autorisé, sans relancer `ldd` ou `unshare`. Une absence de compte ne
devient pas une absence de dépendances manquantes. AppArmor, sysctls, sandbox et
pins ne changent pas. La cause du FATAL Chrome reste inconnue ; Linux natif et
Windows restent des replis non exécutés. La collecte privée est toujours interdite.

## Annexe technique

- Bail réel `02fce7bacbc8435293dbbbbebaad17bd`, statut final
  `released_after_final_runtime_observation`, consommateurs vides.
- Résultat fermé `.tmp/k6-linux-probes-result-20261030-02fce7bacbc8435293dbbbbebaad17bd.json` ;
  validateur exit0, mêmes octets canoniques, aucun flux natif brut conservé/publié.
- Champs du script `parent_runtime_cleanup=INCONCLUSIVE` et
  `copied_script_retirement=PARENT_REQUIRED` conservés tels quels : les preuves du
  parent et de l'observation finale sont distinctes, pas réécrites dans ce résultat.
- Bail initial2dedd réconcilié : entrées constatées absentes par l'inspection du
  nouveau bail, runtime arrêté ; l'échec initial de preuve Windows après exit
  reste documenté et n'est pas promu en PASS.
- Scripts corrigés et temporaires uniquement ; sources durables toujoursf279e71.
  Aucun dispatch/push, ressource Docker ou opération Kind.
