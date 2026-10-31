# Premiers tests réels : Hive réussit, fixture IAM à réaligner

La campagne réelle confirme67 tests Hive réussis, dont les tests de séparation
des identités, de pagination, de suppression et de migration. IAM progresse mais
s'arrête lors du démarrage d'une fixture spécialisée : aucune assertion de la
garde de compte concernée n'a encore été exécutée.

Cette fixture changeait l'autorité de signature sans adapter la politique du
vérificateur HTTP. Le validateur de production refuse correctement cette
incohérence. Je corrige uniquement la configuration de cette fixture, en liant
explicitement les deux autorités et en refusant la compatibilité historique.
Le validateur et les assertions de sécurité restent inchangés.

## Nettoyage vérifié

Les40 conteneurs créés par la campagne ont tous été retirés ; aucun conteneur
possédé ne reste actif ou présent. Les20 conteneurs préexistants ont conservé
leurs identités et états. Aucune ambiguïté de propriété ni erreur de cleanup.

## Suite

Les cibles IAM affectées seront précompilées une par une avec24 jobs, puis la
suite IAM sera reprise par le même runner. Hive n'est pas rejoué : son code et
ses fixtures ne changent pas. Aucun résultat IAM global, Kind, Envoy ou navigateur
n'est annoncé comme acquis. Aucun arbitrage utilisateur n'est requis pour ce
réalignement de fixture dans le contrat approuvé.

## Références techniques

- Sources exécutées : ba21a3c, SDK74e8 ; campagne it-00687e1bb02b42b78c45e0f712e442a6.
- Preuves : `.tmp/itfixtures-it-00687e1bb02b42b78c45e0f712e442a6/` et `.tmp/auth-real-it-launch-105dd72741904e7dbc566bcbf0aaccaa.log`.
- Frontière fautive : setup_test_server_with_signing_keys dans services/IAMRusty/tests/common.rs ; erreurPRINCIPAL_INVALID avant démarrage.
- Registre : `.tmp/docker-resources-20261028.log`. Cleanup40/40, pic3 conteneurs possédés,0 fuite.

### Reprise IAM prête

Le correctif de fixture est committé et relu indépendamment : aucun oracle ou
validateur de production affaibli. Les41 cibles IAM ont recompilé avec succès,
toutes dans le targetWindows prévu, sans erreur mémoire. La suite IAM réelle
reprend maintenant ; les résultats Hive sont conservés sans nouveau passage.
