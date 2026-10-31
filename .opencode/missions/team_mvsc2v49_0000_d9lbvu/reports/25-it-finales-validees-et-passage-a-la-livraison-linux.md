# IT finales validées, passage à la livraison Linux

La suite IAM intégrée au SDK publié termine avec **290 tests réussis, aucun
échec et un test explicitement ignoré**. Les tests de garde de compte, de sessions,
de concurrence, de quotas et de publication des clés passent réellement.

Le test ignoré est l'ancien signupSQS opt-in. Le binaire Kafka ne contient aucun
test sans sa feature ; aucun succès Kafka n'est revendiqué. Les deux tests de
routage SQS IAM sont, eux, exécutés et réussis.

Le nouveau registre a été utilisé par les fixtures réelles et corrélé au témoin
Engine. Les36 conteneurs créés ont tous été retirés, sans fuite ni ambiguïté.
Les20 conteneurs préexistants conservent leurs identités et états. La cause exacte
de l'ancien échec reste non démontrée ; la reprise réussie ne la réécrit pas.

## Preuves conservées et limites

- Hive :67 succès déjà acquis, code métier et oracles inchangés ; pas de nouvelle exécution annoncée sous le dernier SDK.
- Ext-authz :13 succès HTTP déjà acquis ; ils ne prouvent pas Envoy réel.
- SDK :sept unités du registre réussies, deux revues sans défaut bloquant et intégration IAM réussie. Le test enfant Windows réel reste non exécuté.

Le prérequis des IT finales avant Kind est satisfait. Je passe au contexte de
livraison Git propre, puis aux images Linux et au laboratoire local-full
préautorisé, avec les contrôles d'identité et de propriété requis. Envoy réel et
le navigateur restent à qualifier ; la mission n'est pas déclarée terminée.

## Annexe technique

- Application exécutée00c6cf2 ; SDK publié02e5db2 ;41 cibles précompilées séquentiellement,24 jobs inchangés.
- Résultats : `.tmp/auth-iam-incarnation-1d48036bc5614e888ff9d22e3edf5e00/iam-it.log` ; campagne `.tmp/itfixtures-it-1de7a72631cf4f4493a908fbc49f09b3/`.
- Cleanup36/36,0 conteneur possédé présent ou actif, pic2 possédés,0 erreur ; registre contemporain du runner.
