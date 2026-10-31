# Initialisation des files bloquée par une dépendance circulaire

La mission doit encore qualifier Envoy et le navigateur. Le déploiement s'est
arrêté sur LocalStack, le simulateur de files : sept dépendances sur huit sont
prêtes, mais aucune application n'a encore été déployée.

La cause est confirmée : l'initialisation et la sonde interne appellent le nom
réseau du service. Or ce service n'envoie du trafic au conteneur qu'une fois sa
sonde prête. Le simulateur répond bien sur sa boucle locale, mais ses files n'ont
donc pas pu être créées. Le script monté est exécutable, sans défaut d'encodage.

Je corrige uniquement ces appels internes pour utiliser la boucle locale. Le
nom annoncé aux applications reste inchangé ; aucune règle réseau, permission,
image ou frontière d'authentification n'est assouplie. Un contrôle ciblé précède
la correction, puis le conteneur possédé sera renouvelé avec ses identités tracées.

La suite reprendra après les dépendances, avec le même identifiant de campagne.
Elle ne recréera pas les secrets déjà créés, ne supprimera ni jobs ni données et
ne relancera pas aveuglément le déploiement complet. Aucun succès fonctionnel
d'authentification n'est déduit de ce diagnostic d'environnement.

## Preuves

- Déploiement exit1 sur attente300s d'une dépendance ; 110 ressources tracées.
- LocalStack3.0.2 : service SQS running, hook READY `init-aws.sh` en ERROR.
- Commande interne actuelle : endpoint `http://localstack:4566`, inaccessible.
- Même commande avec endpoint loopback : réponse SQS réelle « file absente ».
- Aucun défaut de shebang/BOM/CRLF, mode du script0555 ; aucune lecture Secret.

## Correction vérifiée dans le laboratoire

La reprise ciblée a réussi : le script d'initialisation est maintenant marqué
SUCCESSFUL et les deux files demandées répondent réellement. Le Deployment a
gardé son identité ; seul son Pod a été renouvelé. L'ancienne configuration est
conservée, sans suppression de secret ni de job.

Je poursuis les étapes de bootstrap et d'application qui n'avaient pas encore
été exécutées, avec le même identifiant de campagne, sans relancer la fondation.
