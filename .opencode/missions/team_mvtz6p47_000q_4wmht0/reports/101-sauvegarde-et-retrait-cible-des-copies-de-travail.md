# Sauvegarde et retrait ciblé des copies de travail

L'utilisateur a choisi **« Sauvegarder puis nettoyer »** et autorisé le retrait
forcé des dix dossiers temporaires listés au rapport100, uniquement après sauvegarde
vérifiée de leurs changements utiles. Aucun autre dossier ni référence de sauvegarde
n'est couvert. Les archives d'équipe ne doivent pas être purgées.

Le code fonctionnel et ses preuves restent publiés ; le dernier commit documentaire
`678ed432c440c0304231ac5700a4717785bdcc22` a également été poussé et vérifié sur
master. Aucun code de test ou de service n'est à modifier pour ce nettoyage.

## Conditions appliquées

- Inventorier les fichiers modifiés et nouveaux, ainsi que les sous-modules/liens,
  sans collecter de secrets ou de données de campagne privées.
- Conserver une sauvegarde locale récupérable des changements utiles, son HEAD de
  base et des empreintes ; en vérifier la restauration avant tout retrait.
- Ne retirer aucun arbre si sa sauvegarde/disposition est incomplète ou si un outil
  refuse l'accès nécessaire. Ne pas contourner un refus ni supprimer par motif global.
- Préserver le checkout principal, les modifications étrangères, le sous-module
  actuel, les autres missions, les outils, WSL et son VHDX sur I:.
- Après retrait autorisé, vérifier Git, les dossiers et l'inventaire natif, puis
  tenter l'archivage normal de l'équipe sans purge des historiques ou sauvegardes.

Ce rapport prépare l'opération ; il ne prétend pas encore que les sauvegardes,
le retrait ou l'archivage soient achevés. Le résultat fonctionnel Windows PASS et
les limites HTTP/WSL/Linux du rapport99 restent inchangés.

## Résultat ultérieur vérifié

Huit copies ont été sauvegardées, restaurées puis retirées. Deux restent conservées,
faute de récupération complète après des refus de lecture antérieurs. L'archivage
de l'équipe n'est pas terminé. Voir le [bilan précis et la reprise ciblée](103-huit-copies-retirees-deux-sauvegardes-restent-bloquees.md).
