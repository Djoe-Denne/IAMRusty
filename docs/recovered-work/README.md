# Travail récupéré lors du nettoyage Git

Ces artefacts préservent du travail de branches retirées et la correction d'outil
autorisée pendant leur nettoyage. Le statut est indiqué pour chaque artefact ;
leur présence ne constitue pas une décision Accepted. Ne pas les appliquer à l'aveugle.

## SQS / LocalStack

`20261106-rustycog-sqs-readiness.patch` est généré depuis le commit SDK historique
`871bf674b5b5cf8bba8a3918f33c3d8c9f3a4109`, parent `519db92`.
Il provient de la branche locale retirée `codex/manifesto-openfga-read-tuples`.

La lecture actuelle des fonctions `TestSqs::wait_for_localstack` et
`create_test_queues` confirme que la readiness repose toujours sur TCP avec un
délai fixe, sans la sonde protocolaire et les retries de cette ancienne branche.
Cette différence n'est donc pas déclarée intégrée par simple similarité de noms.
Les lectures de tuples OpenFGA du commit parent sont déjà intégrées (`7c89d4c`).

Le patch historique comporte également des changements de singleton et d'adresse
hôte. Il doit être réconcilié avec les fixtures host-aware et leur cycle de vie
actuels, pas réintroduit littéralement. Aucun changement Rust ni du pin SDK
`93b890ff7b821e9cb2e7d0aeb81957d098778a2f` n'est effectué pendant ce nettoyage.
Aucun test de ce correctif n'est annoncé exécuté. Le bundle externe complet des
quatre anciennes branches SDK reste récupérable avec le manifeste de sauvegarde.

## Compatibilité des anciens noms Ensemble — appliquée à l'outil global

`20261106-ensemble-legacy-branch-names.patch` conserve le changement source
minimal appliqué, sur autorisation utilisateur, au plugin installé et à sa source
de déploiement. Il ne modifie pas la base de données ni les protections de purge.
La régression locale `legacy-branch-purge.test.cjs`, dans le paquet installé,
vérifie les fonctions extraites du bundle réellement utilisé : ROUGE avant fix,
puis 11 tests PASS. La purge native a ensuite fonctionné avec ses confirmations
habituelles. Ce patch concerne l'outillage, pas le code applicatif AIForAll.
