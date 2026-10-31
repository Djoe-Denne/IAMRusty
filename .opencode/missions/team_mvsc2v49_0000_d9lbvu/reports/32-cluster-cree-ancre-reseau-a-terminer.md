# Cluster créé et ancré, réseau à terminer

Les corrections Windows ont permis de créer les trois nœuds du laboratoire et
de vérifier son identité : control-plane, publication API loopback, kubeconfig
privé et UID du cluster concordent. Les créations et leurs volumes sont tracés
par le témoin Engine puis corrélés au bail.

Le bootstrap s'arrête ensuite avant d'appliquer le réseau Calico : le helper
PowerShell n'accepte pas l'entrée par pipeline qu'il reçoit. Un autre diagnostic
montre un décodage implicite incorrect des messages UTF-8 de Kind. Ce ne sont
pas des défauts d'authentification et aucune preuve Envoy ou navigateur n'est acquise.

Je corrige ces deux frontières de conduite avec des tests natifs sans cluster.
Le laboratoire ancré reste sous bail pour poursuivre ce travail ; il ne sera ni
recréé ni réinitialisé. La prochaine exécution utilisera l'ancrage existant
vérifié, sans nouvelle création de nœud. Le cluster legacy reste étranger.

## Preuves et limites

- Les trois IDs de création concordent exactement entre Engine et bail ; volumes corrélés aux mounts, registre `.tmp/docker-resources-20261029.log`.
- UID `kube-system` : d5a5c123-384d-4b3e-a45a-43d9ad6170d7 ; API uniquement sur 127.0.0.1.
- Aucun service applicatif déployé ; réseau, readiness et comportement restent à vérifier.
- Bail actif : session privée 6b7b2d450c6a411584ccdac54cbfadbd ; aucune suppression prévue.

## Correctifs de conduite vérifiés

Les défauts ont chacun été reproduits avant correction. Les 28 tests passent
ensuite : pipeline complet et cible inchangée, UTF-8 strict, sorties invalides
bloquées sans divulgation et échanges binaires inchangés. Aucun test ne contacte
le cluster. Les trois nœuds sont encore NotReady, réseau non installé.

Je poursuis la reprise du seul réseau public épinglé sur l'ancrage vérifié.
La revue sécurité du commit exact sera menée en parallèle, avant le déploiement
applicatif et les preuves OAuth. Aucun nouveau nœud ou cluster n'est demandé.
