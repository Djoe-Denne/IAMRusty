# Revue lead de la livraison native des sondes ponctuelles

## Entrée et périmètre

Livraison autorisée par canal natif `result_mvv6l21b_0002_hn28nk`, obtenue après
refus de lecture directe du worktree externe par Context Mode. Aucun élargissement
de permissions ni lecture de ce chemin par un autre outil. Les octets fournis par
l'auteur, pas un fichier externe relu, sont l'entrée de cette revue.

Script627lignes/SHA2567d1db94e963bf665f9a1143da4ceddba4967ba6282ca8dfaba98e71d4a4f5995 ;
tests379lignes/SHA256af4690ae43bb5efa4832875845ccf6502d1ef31729da059907c7e41086f42fc1.
Rapport complet auteur `result_mvv6gtnt_0000_auer38` :18tests/0.126s/exit0/sansskip,
preuve attribuée, pas encore revue/test de la copie canonique. OPS/4modules gelés.
Contrat `k6-chromium-linux-probes.md` Accepted/Unimplemented.

## Correction exigée avant exécution — signal distinct d'un exit normal

Dans `run_one`, un `child.code < 0` est actuellement projeté comme
`termination=EXITED`, `complete=true` si aucune exception du superviseur n'a été
levée. `project_probe` le présente ensuite comme NONZERO_EXIT. Le code négatif
subsiste, donc aucun faux succès, mais la catégorie de terminaison est fausse.
Le contrat distingue explicitement exit/signal/timeout/interruption ; cette
distinction est justement importante pour l'enquête de démarrage.

Corriger le seul script ponctuel pour représenter explicitement le signal
natif. Conserver le code négatif observé et le verdict non réussi, pas de nouvelle
table de signaux ou texte natif. Mettre à jour la validation fermée de projection
et ajouter une fixture qui exerce le vrai câblage run_one → project_probe →
public_bytes avec par exemple code-6, cleanup/identité observés, sans lancer un
processus réel. Exit1, timeout, interruption, overflow et identité absente restent
distincts. Ne pas transformer un signal en test réussi ni en cause Chromium.

Pas de correction OPS/contrat/Child/env/pins/ACL, nouveau framework ou runtime.
Même auteur/session/profil/modèle ; test ciblé ROUGE avant correction si possible,
puis suite ponctuelle seulement. Aucun nouveau test194 redondant.

## Préparation d'entrée requise, pas permission de contourner une garde

Le script exige `private_chain(source)` hors Git, alors que le chemin source
observé désigne le clone887b, dont un ancêtre contient `.git`. L'utiliser tel quel
est donc refusé avant les sondes. Ne pas lancer pour confirmer un refus déjà
déductible. Cette contrainte locale n'autorise pas à relâcher private_chain.

Le lead préparera plutôt un **export public limité aux quatre fichiers vérifiés**
dans un répertoire privé neuf hors Git, conservera le clone initial inchangé et
vérifiera chaque hash avant utilisation. La valeur source_dir ne sera pas reprise
telle quelle de l'ancien fichier de chemins. L'export ne sera ni un bail ni une
preuve d'identité fraîche. Le commentaire relatif à « Linux22 existant » est
inexact pour l'instance AIForAll Ubuntu24.04 ciblée : le reformuler comme
diagnostic non qualifiant, sans supposer une plateforme mesurée ou modifier la
garde de qualification du runner.

## État restant

Preuve lead sur la copie reçue par livraison native, SHA inchangés : suite auteur
18tests/0.118s/exit0/sansskip. Régression indépendante
`test_signal_regression.py` :1test/0.005s/exit1, échec attendu
`EXITED != SIGNAL`, sur le câblage réel run_one → project_probe ; aucun processus
natif lancé. C'est une preuve ROUGE du script, pas un échec d'environnement.

Pas de sonde unshare/ldd, Chrome/k6 ou collecte exécutée. Runtime au dernier
inventaire : AIForAll arrêtée, étrangers inchangés. La fermeture S1/S2 est acquise
sur ses blobs propres, pas réouverte ici. Après correctif et livraison native,
intégration ciblée, hash, tests lead, export/identités et bail neuf avant le run.
