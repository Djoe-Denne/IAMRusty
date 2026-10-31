# Garde IAM validée, dernier récepteur de test à configurer

La reprise confirme que la garde de compte IAM fonctionne : ses deux tests
passent maintenant, ainsi que les témoins de refresh et de séparation des jetons.
La correction précédente de fixture est donc validée réellement.

La suite s'arrête plus loin dans deux tests d'admission de clés. Leur récepteur
SDK créé directement n'a pas reçu la nouvelle politique de principal obligatoire.
Il refuse de se construire, avant de vérifier la signature ; ce n'est pas un
échec observé du quota. Les trois autres tests de ce binaire passent.

Je renseigne uniquement les deux champs de configuration du récepteur de test,
en reprenant son autorité déjà connue et sans compatibilité historique. Les
assertions de quotas, de publication et de signature restent intactes. La
recherche ciblée confirme qu'il n'existe pas d'autre constructeur direct oublié
dans les tests IAM.

## Nettoyage et suite

Les30 fixtures de cette reprise ont toutes été retirées, sans fuite. Les20
conteneurs préexistants ont conservé leurs identités et états.

Seul le binaire d'admission est à recompiler. La suite IAM sera reprise par le
runner existant, qui ne propose pas de sélection par binaire : je conserve ses
garanties de propriété et de cleanup plutôt que créer un nouveau harness.
Hive n'est pas rejoué. La qualification IAM globale reste ouverte ; Kind et le
navigateur restent différés.

## Références techniques

- Source exécutée0ae4222 ; campagne it-c2e84f197dbb4268b355f511b3591796.
- Log privé `.tmp/auth-iam-it-retry-9cb6e9ae830d4f13bfb6798f2195326b.log` ; inventaires dans `.tmp/itfixtures-it-c2e84f197dbb4268b355f511b3591796/`.
- Frontière : signing_admission.rs::assert_platform_receiver, constructionSDK refuséePRINCIPAL_POLICY_UNAVAILABLE ; delta de deux champs test-only.
