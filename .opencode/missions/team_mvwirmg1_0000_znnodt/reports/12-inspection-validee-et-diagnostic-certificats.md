# 12. Inspection validée, préparation des certificats à achever

La mission vise toujours une campagne d'authentification réelle sous Windows. Le raccord d'inspection passe désormais : version et options des quatre cas sont validées, confidentialité et nettoyage passent, sans navigateur ni accès à la cible.

La relecture indépendante ferme le défaut de restauration dans le code : la réparation ne dépend plus de la santé des Pods, tout en conservant les contrôles de cible, propriété et données attendues. Ce PASS statique ne prouve pas encore la restauration réelle.

La préparation locale des certificats a échoué avant toute création du laboratoire. La cause est maintenant observée : Git Bash transforme le nom du certificat en chemin Windows. Un essai comparatif échoue normalement puis réussit en désactivant cette conversion pour la seule commande ; la clé partielle est inchangée.

Je poursuis la génération dans une destination neuve, sans écraser les sorties partielles ni modifier l'environnement global. Les identités, le déploiement et le trajet réseau devront encore être vérifiés avant les appels d'authentification. Aucun nouveau GO n'est requis pour cette correction locale conforme au mandat.

## Preuves et limites

- Inspection : `5dc47c613727497eab36374e9f8b67bd`, exit0, quatre inspections, confidentialité/cleanup PASS, k6/Chrome0→0, sortie retirée.
- [Relecture corrective](../../../../.cursor/review-briefings/20261031-security-k6-auth-parent-s1-d00ab08.md), delta d00ab08, aucun finding bloquant nouveau.
- Diagnostic privé : commande OpenSSL normale exit1, avec `MSYS_NO_PATHCONV=1` exit0 ; hash de la clé inchangé. Aucun certificat déployé, cluster créé ou run auth effectué.
