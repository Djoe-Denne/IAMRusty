# Certificats locaux et reprise après arrêt Docker

## Décision locale du 8 novembre 2026

Conserver les identités TLS existantes pendant l'activation OAuthConnect, sans
rotation automatique. Le mode explicite `deploy --oauth-connect
--preserve-platform-tls` préserve les trois Secrets TLS de plateforme et la
référence TLS OpenBao du bail. Il crée la feuille du nouveau service seulement.
Il n'autorise ni lecture des Secrets Kubernetes ni désactivation de CA, SAN,
hostname, date de validité ou mTLS. Ce n'est pas une politique cloud.

## Ne pas confondre les incidents

L'utilisateur rappelle un précédent supposé de certificat expiré pendant que
Docker était arrêté. **Ce précédent n'a pas été établi dans cette session.**
Un certificat a une période de validité calendaire ; arrêter Docker ne suspend
pas cette période. La reprise peut donc révéler un certificat devenu expiré,
même si aucun processus de renouvellement ne tournait pendant l'arrêt.

Le préflight actuel a établi un autre problème : le vieux dossier PKI référencé
était incomplet ; le bundle complet du registre correspondait à la CA publique
OpenBao live, mais pas à l'empreinte de la feuille Lazaret conservée lors de la
réparation du 2 novembre. Un déploiement global depuis ce bundle aurait changé
des identités existantes. Les CA locales inspectées expirent le 31 octobre et
le 1er novembre **2027** ; la feuille workload du bundle complet expire le
30 janvier **2027**. Aucun de ces certificats inspectés n'était expiré le
8 novembre 2026. Cela ne prouve pas la validité de tous les Secrets live, qui
n'ont pas été lus.

## À traiter ultérieurement

- Définir un préflight de reprise qui distingue expiration, mauvaise CA/SAN,
  source PKI incomplète et fichier différent de l'identité effectivement montée.
- Définir une procédure explicite de renouvellement **sous la même CA** lorsque
  possible : nouveaux inputs privés, calendrier, chargement/redémarrage contrôlé,
  vérification réelle et conservation des données. Le changement de CA reste
  une opération distincte de distribution de confiance.
- Décider du traitement local d'une expiration survenue pendant l'arrêt ; aucune
  exception « accepter tous les certificats » n'est implémentée implicitement.
- Ajouter une preuve arrêt/reprise avec validité franchie et consommateurs
  vérifiés, sans modifier l'horloge hôte ni extrapoler cette tolérance au cloud.
- Réconcilier les références PKI/lifecycle historiques avant de faire d'un
  redéploiement global le parcours de reprise d'un environnement existant.

Sources : [procédure Ops](../../ops/deploy/README.md),
[préservation Lazaret du 2 novembre](../missions/20261102-local-platform-repair.md),
[RFC 5280, §4.1.2.5](https://www.rfc-editor.org/rfc/rfc5280.html#section-4.1.2.5).
