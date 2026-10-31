# Plan opérateur — laboratoire OAuth local-full et runner Windows

Révision2 approuvée. **Plan non exécuté**, préconditions finales non encore réunies : raccord parent incomplet, aucune création ni ressource possédée. Ce fichier n'est pas un script de lancement automatique et ne remplace pas les gardes/baux existants. Figer les expansions privées et le HEAD effectivement exécuté avant les commandes réelles.

## Cible et outils vérifiés sans démarrage

- Cible unique aiforall-local-full / kind-aiforall-local-full ; configuration ops/deploy/kind/cluster-local-full.yaml,1CP+2workers, API loopback port observé, entrée127.0.0.1:18080, IdP localhost:18081 par forward local gardé existant.
- Python `C:/Python313/python.exe` (PyYAML6.0.2), PowerShell `C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe`, Git Bash `C:/Program Files/Git/bin/bash.exe`, Docker/kubectl sous Docker/resources/bin, Kind `C:/Users/djden/bin/kind.exe`. Pas PATH supposé pour k6/Chrome ; leurs chemins privés restent à figer via le socle Windows.
- Répertoire approuvé existant : Path.home()/AppData/Local/Temp/opencode. Session neuve et privée selon ops/deploy/README.md:40–74 ; ancrage cluster-anchor-task_id absent et distinct, aucune adoption d'ancien bail.

## Identité de campagne unique, pas GUID indépendant au lancement

Le lead choisit un identifiant neuf hex32 avant génération des entrées privées. Le même identifiant lie inputs du fournisseur, compte run_id@example.invalid et K6_RUN_ID du scénario. render-local-full.py:21–38 valide le schéma fermé version/run_id/client_id/client_secret/hmac_secret/oauth_state_secret et injecte run_id ; oauth-idp-simulator.py:87–92 fixe email et observations sur ce run_id. Un nouveau GUID arbitraire généré par le runner après déploiement serait incohérent.

Le parent vérifiera l'attente contre le run_id réellement renvoyé par `/observations` et la configuration/identité chargée. Une entrée privée fournit une attente, jamais une autorité de cible auto-attestée. Secrets générés et fichiers privés ne sont pas imprimés ni versionnés.

## Ordre et formes exactes des commandes existantes

Ne pas exécuter avant intégration du raccord, revue bornée et gates pertinents/réutilisation justifiée des17IT. Variables ci-dessous : chemins absolus neufs à enregistrer dans la fiche privée du run, pas valeurs ambiantes implicites. Chaque code natif doit être contrôlé ; échec = arrêt de phase sans retry/reset automatique.

1. Préparer inventaire BEFORE et bail de ressources ; session privée, inputs OAuth et JSONleasev2 créés exclusivement selon le guide, état pending_creation et aucunID/UID/hash/API prérempli. Avant l'appel opaque de création, journaliser le contexte ; recueillir les vrais IDs/deltas à la création et réconcilier tout UNKNOWN, y compris échec partiel. Pas ressources nouvelles attribuées par nom.
2. Certificats privés neufs, par le script existant avec `$env:LOCAL_FULL_CERT_DIR=$certs` et appel :

   `& 'C:/Program Files/Git/bin/bash.exe' ops/scripts/prepare-local-full-certs.sh --oauth-proof`

   Environnement Bash OpenSSL Git vérifié : génération achevée dans une destination neuve avec `bash.exe -c 'MSYS_NO_PATHCONV=1 sh ops/scripts/prepare-local-full-certs.sh --oauth-proof'`. Cette variable ne vaut que pour cette commande ; sans elle Git Bash transforme le sujet /CN en chemin Windows. Index privé actualisé ; sorties partielles conservées. Huit leaves transport et un workload vérifiés, nomIAMaccepté et mauvaisnomHive refusé par la mêmeCA. Pas installation de CA hôte ni publication des clés.
3. Rendu source offline (ne prend PAS d'inputs runtime privés), sortie privée neuve :

   `& 'C:/Python313/python.exe' -B ops/deploy/deploy-local-full.py render --oauth-proof --output-dir $render`

4. Après toutes les gates, bootstrap existant :

   `& 'C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe' -NoProfile -File ops/deploy/bootstrap-local-full.ps1 -LeasePath $lease -AfterFinalIT`

   Le commit a43b0be ajoute --retain seulement à ce cluster pour garder les nœuds en cas d'échec. Le flag AfterFinalIT atteste les gates déjà satisfaites ; il ne les exécute pas. Relever IDs des trois nœuds, API6443observée, kube-systemUID et hash kubeconfig depuis le nouveau bail, jamais ancien journal.
5. Épinglage après ancrage seulement :

   `& 'C:/Python313/python.exe' -B ops/deploy/local-full-images.py --lease $lease --output $images --oauth-proof`

   ATTENTION : IAMlatestcf02 est antérieur au correctif, ne pas le déployer. L'image IAMqualifiée IDb6f140d16558ae9b9dae53a4944e897226952ab91b8b09d02f9dcff69fc6d960 et son tag immuable sont présents. Remplacer dans le seul manifeste privé nouvellement généré l'entrée aiforall-iam-service:latest par ce tag immuable après vérification de source/identité ; ne pas retaguer latest partagé. Les neuf autres choix restent à rapprocher des sources historiques retenues ; toutes dix images sont présentes, aucune construction n'est requise par simple absence de tag. Le déployeur vérifie chaque tagsha256 contre Engine avant import.
6. Déploiement par les gardes existantes, aucun profil plugin additionnel :

   `& 'C:/Python313/python.exe' -B ops/deploy/deploy-local-full.py deploy --lease $lease --image-manifest $images --cert-directory $certs --after-final-it --oauth-proof --oauth-proof-input-file $proofInputs`

7. Inventaire et contrôle de la cible créée :

   `& 'C:/Python313/python.exe' -B ops/deploy/deploy-local-full.py inventory --lease $lease`

   Vérification infra indépendante bornée : identités/Ready/namespace/configuration et trajet Windows→Envoy→IAM/connecteur/IdP. Pas preuve produit/HA/Manifesto globale, aucun accès Secret par le reviewer.

   Précision de campagne à consigner dans le nouveau bail avant admission auth : le déployeur full passe IAM à deux répliques, tandis que la preuve historique et le raccord ciblé exigent un unique Pod IAM pour son identité/rollout. Sur ce laboratoire neuf possédé seulement, observer DeploymentUID et replicas, puis utiliser la CLI gardée existante `local_full_target.py kubectl --lease $lease --cluster aiforall-local-full -- scale deployment/iam -n aiforall-local-full --replicas=1`, attendre le rollout gardé et inventorier le PodUID réel. Pas de nouvelle variante de manifest, migration, suppression ni modification d'un laboratoire étranger. Le verdict reste explicitement mono-instanceIAM, pasHA. Ne pas lancer si cette préparation n'est pas prouvée ou si une identité est ambiguë.

## Inspection/exécution auth : verrou restant

L'entrée Windows auth est intégrée dans51e74e1 mais reste derrière les correctifs d'inspection et de restauration décrits au rapport11. Commandes désormais livrées : `powershell.exe -NoProfile -NonInteractive -File ops/tests/k6/run-windows.ps1 -Mode auth-inspect -Output $InspectDirectory` sansConfig, puis `-Mode auth -Output $AuthDirectory -Config $PrivateAuthConfig`. Fixer les expansions privées et chemins absolus avant run ; ne pas lancer auth avant fermeture des défauts.

Argv enfant : `version` ; `inspect scénario-copié` (delta attendu : phaseSAN explicite fermée, pas tout l'environnement) ; `run --quiet --address 127.0.0.1:0 --system-tags name,check,scenario,group scénario-copié`. Arguments séparés, pas shell de commande libre. SchémaConfig réellement livré décrit dans ops/tests/k6/AUTH-WINDOWS-PARENT.md.

Entrées fermées du JS intégrées : K6_ORIGIN exact, K6_RUN_ID de campagne, K6_SUMMARY_PATH neuf, version navigateur attendue ; SANPHASE good/bad/restored déjà défini pour inspect.26checks OAuth et2parphaseSAN, schéma project inchangé, troisrésumésSAN distincts du résuméOAuth.

Séquence de corrélation à maintenir : snapshotEnvoy avant sans forwardOAuth ; ouvrir forwardOAuth gardé ; observationsIdPavant ; parcours navigateur ; observationsIdPaprès ; fermer le forward possédé ; snapshotEnvoyaprès. L'API Admin existante refuse un forwardOAuthactif. La phase certificat a ses propres baselines, ACK/UID/bytes, compteurSAN mêmeincarnation, puis restauration et positif de récupération en finally. Aucun ancien wrapper hardcodé importé tel quel.

## Sortie et cleanup

Résumé expurgé uniquement, cas/compteurs/verdicts/versions/SHA et limites, aucun JWT/cookie/code/state/clé/URL sensible/HAR/trace. Fermer enfants/fowards possédés et restaurer sur toute issue. Inventaires avant/après et registre append-only contemporain par vrais IDs ; stop gracieux des seuls nœuds créés devenus inutiles. Pas suppression/prune/reset, pas arrêt Desktop/WSL préexistants ou workload étranger. En échec partiel, garder les IDs et données privées nécessaires, signaler tout cleanup bloqué, pas relance aveugle.
