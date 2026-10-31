# SDK publié, reprise IAM avec le registre corrigé

Le correctif privé du registre de fixtures est publié sur la branche de mission
autorisée et sélectionné par l'application. Le commit ne contient que les deux
fichiers examinés ; les sept tests, le formatage et les deux revues indépendantes
portent sur les mêmes octets. Aucun changement de production d'authentification,
de dépendance ou de format des événements de preuve.

La création reste exclusive : une nouvelle incarnation de processus reçoit un
nouveau fichier, sans réutiliser ou tronquer le précédent. La réutilisation d'un
PID reste une hypothèse pour l'échec historique, pas une cause rétroactivement
déclarée certaine.

## Vérification intégrée suivante

Les41 cibles IAM seront précompilées une par une après le changement de crate
partagée, avec24 jobs inchangés. Si toutes réussissent et que les sources restent
figées, le runner IAM existant reprendra automatiquement avec inventaire,
registre et nettoyage de ses seules fixtures. Aucun nouveau harness n'est créé.

Les67 succès Hive et les13 succès HTTP d'ext-authz sont conservés avec leurs
versions exécutées explicitement indiquées. Ils ne sont pas présentés comme une
nouvelle exécution sous le SDK sélectionné. Le test enfant Windows réel demeure
non exécuté ; les unités ne le remplacent pas. Kind et le navigateur restent
après les IT finales.

## Traçabilité

- SDK publié :02e5db23aac6016aab062c3078696d5f7fd464ef, parent74e8a89, branche mission/authz-principal-sdk-pool3-20261028 sur github.com/Djoe-Denne/rustycog ; avance rapide et SHA distant exact vérifiés.
- Application sélectionnée :00c6cf2c0223cb3cfee046088405214795d7b0c9, commit normal avec contrôle de formatage.
- Configuration utilisateur Serena du sous-module préservée par comparaisonSHA256 ; aucun push de l'application.
- [Revue sécurité](../../../../.cursor/review-briefings/20261029T0134Z-security-sdk-fixture-ledger-74e8a89.md), [revue des oracles](../../../../.cursor/review-briefings/20261029T0136Z-tests-sdk-fixture-ledger-74e8a89.md).
- La documentation [Microsoft sur les identifiants de processus](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/ns-processthreadsapi-process_information) confirme qu'un PID peut être réutilisé après libération du processus. Cela justifie l'invariant corrigé, sans prouver rétroactivement la cause du run en échec.
