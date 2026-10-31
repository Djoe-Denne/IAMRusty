# Références de sauvegarde à restaurer après le nettoyage natif

Les dix dossiers autorisés sont maintenant retirés après sauvegardes vérifiées.
Le nettoyage natif de l'équipe a annoncé sa réussite, sans option de purge.
Le contrôle réel qui suit révèle toutefois que ses références Git préservées ne
sont plus présentes. Cette suppression n'était pas autorisée et n'est pas traitée
comme une conséquence acceptable de l'archivage normal.

Le HEAD principal n'a pas changé, aucun diff de source/contrat/workflow n'a été
introduit par les « safety-net merges » annoncés, et les sauvegardes locales restent
intactes. L'inventaire pris avant l'opération contient chaque nom de référence et
son commit exact. La réparation ciblée consiste à recréer les seules références
manquantes à ces mêmes commits, sans écraser de référence existante, sans déplacement
de HEAD, reset, checkout ou nouvelle fusion. Chaque objet sera vérifié avant écriture.

Cette restauration relève de l'obligation explicite de conserver ces références.
Elle ne purge pas l'archive d'équipe et ne modifie ni le produit ni le harness.
L'archivage et la préservation ne seront annoncés conformes qu'après vérification.

Preuves privées : `.tmp/k6-cleanup-metadata-20261031.json` pour les références
antérieures et `.tmp/k6-cleanup-final-verification-20261031.json` pour le constat
après nettoyage natif. Aucun log ou secret n'est publié.

## Réparation vérifiée

Les 11 références ont été recréées avec `git update-ref`, uniquement lorsqu'elles
étaient absentes. Chaque objet est bien un commit déjà inclus dans master. Les
noms et commits correspondent exactement à l'inventaire antérieur ; aucune
référence existante n'a été écrasée et HEAD est inchangé.

L'équipe est confirmée `archived` par lecture seule de la base native ; ses
23 membres sont shutdown. Le ticket restant a été annulé par l'archivage normal,
sans publication ni exécution Linux. Aucune correction du harness ou de sa base
n'a été faite. Voir le [bilan final](107-nettoyage-et-archivage-verifies.md).
