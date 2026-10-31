# Dernières sources sauvegardées, sans reprise WSL

La copie native ciblée autorisée a réussi : **32 sources publiques sauvegardées
et restaurées**, avec SHA256 identiques avant/après et sur les copies restaurées.
Aucun nouveau refus, changement de permissions, outil métier ou runtime lancé.
Les dossiers restent présents jusqu'à publication des derniers livrables durables.

## Disposition vérifiée

- Ancienne copie du conducteur WSL : 13 sources correspondent à la version actuelle,
  deux sont déjà dans l'historique Git. Deux scripts ponctuels sont déjà archivés à
  empreinte identique. Les cinq autres sources du raccord `ldd-only` sont récupérées
  et ajoutées à l'archive inactive, sous suffixe `.txt`, avec un manifeste distinct.
- Ancienne copie du raccord Windows : une source correspond à la version actuelle,
  neuf sont des versions historiques remplacées. Aucune ancienne protection ou
  photographie documentaire ne remplace le code simplifié et vérifié.

Les cinq sources tardives WSL ne sont pas exécutées, testées, ratifiées ou ajoutées
au lanceur. Elles restent hors découverte des tests et de la CI. Les 19 sources
initiales de l'archive sont inchangées ; le total est maintenant de 24 sources.
Ni seconde mesure de dépendances, ni sonde, ni qualification Linux n'est effectuée.

## Preuves et suite

Commande native : `powershell.exe -NoProfile -NonInteractive -File
.tmp/k6-closeout-final-backup-20261031.ps1`, code 0 ; analyse syntaxique préalable :
zéro erreur. Sauvegarde et restauration sous `.tmp/k6-closeout-final-backup-20261031/`.
Manifeste SHA256 : `569ea4a91aa18a3240e91fe2d59ed5db49ae38c708166744eff980862a629dc6`.
Les deux dossiers rustycog sont vides ; les statuts Git des deux arbres sont inchangés.

Les cinq archives tardives sont comparées octet par octet aux sauvegardes, tailles
et SHA256 conformes. `git diff --check` réussit. Les journaux et données de campagne
privées ne sont pas copiés ni publiés. Les sauvegardes locales et références Git
préservées restent conservées.

Suite autonome : publier les archives documentaires, recontrôler les 32 sources et
leurs sauvegardes, retirer les deux derniers dossiers autorisés, puis vérifier
l'archivage natif sans purge. Le cas Windows reste PASS ; HTTP/WSL/Linux gardent
leurs limites précédentes. L'équipe n'est pas encore annoncée archivée.
