# Sauvegardes vérifiées ; deux copies restent à récupérer

## État du nettoyage autorisé

Le résultat k6/Chrome Windows reste publié et réussi. Huit des dix copies
temporaires disposent maintenant d'une sauvegarde récupérable vérifiée : leurs
47 fichiers modifiés ou nouveaux, les versions de base et les versions préparées
pour commit sont conservés, ainsi que quatre patchs publics de livraison.
Chaque copie de sauvegarde a été restaurée dans un autre dossier et comparée par
SHA256, une empreinte qui vérifie l'égalité des octets.

Les deux autres copies sont conservées. Des lectures externes de leurs contenus
avaient été refusées ; le lead n'a pas changé de lecteur ni de permissions pour
les contourner. L'inventaire actuel porte uniquement sur les noms et métadonnées.
La récupération native de leurs seules sources publiques par les auteurs constitue
la prochaine alternative, sans reprendre leur développement ni les essais Linux.

## Disposition des huit copies sauvegardées

Les versions de code conservées sont déjà intégrées ou présentes dans l'historique
Git. Les six fichiers de code et tests Windows finaux correspondent au code publié.
Les anciens guides ne remplaceront pas la photographie vérifiée actuelle.

Les différences documentaires restantes ont été comparées : propositions devenues
des contrats ratifiés et complétés, ou propositions abandonnées. Les anciennes
exigences de conservation du code, verrous et contrôles de permissions ne sont pas
réactivées. La proposition de déplacement/maintenance WSL reste non ratifiée et
hors mandat ; sa source est sauvegardée, pas ajoutée aux obligations actuelles.
Aucune décision Accepted n'est rétrogradée et aucune proposition n'est promue.

Les huit dossiers rustycog de ces copies sont vides, sans lien symbolique dans les
arbres ; le vrai sous-module du checkout principal est laissé intact. Le retrait
ciblé de ces huit copies peut donc suivre l'autorisation déjà donnée, après un
dernier contrôle de stabilité. Les références de sauvegarde restent conservées.

## Preuves et suite

- Sauvegarde : `.tmp/k6-closeout-backup-20261031/`, hors des dossiers à retirer.
- Manifeste SHA256 : `8a14e2d0155e7b7b461fd7b1dc4e827738e1dc1da3fc9e22ecbc115e01aa8079`.
- Inventaires et comparaison : `.tmp/k6-cleanup-metadata-20261031.json` et
  `.tmp/k6-cleanup-disposition-20261031.json`. Aucun fichier privé de campagne copié.
- Copies sauvegardées : pool-3, 4, 7, 9, 10, 12, 22 et 23. Copies encore conservées :
  pool-13 et 20. Aucun dossier n'est encore retiré à la rédaction de ce rapport.
- Suite : demander la livraison native bornée aux deux auteurs, retirer les huit
  copies vérifiées, puis contrôler les inventaires avant l'archivage normal.
  Aucun nouveau GO général n'est demandé ; aucun runtime n'est démarré.
