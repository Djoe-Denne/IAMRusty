# Rapport 12 — répartir les artefacts sans élargir les permissions

## État et preuves

L'architecte poursuit les sources en lecture seule. Il signale l'absence de
write_memory dans son catalogue et l'obligation d'un pointeur wiki hors des
chemins possédés T1. Aucune écriture ADR ou wiki n'est annoncée.

Le lead a relu .cursor/rules/adr-serena-digest.mdc : la création d'une ADR
numérotée exige un digest MCP dans le même tour ; l'index README est exclu.
Le catalogue du lead expose réellement serena_1mcp_write_memory. Le profil
architecte prévoit un contrat référencé et, si nécessaire, une ADR Proposed.

## Répartition communiquée

- L'architecte rédige le contrat et les index autorisés dans son worktree.
- Il retourne le texte complet de l'ADR Proposed, son chemin et le digest
  suggéré, sans créer lui-même le fichier ADR qui déclencherait la règle.
- Après examen/intégration, le lead pourra créer le texte architecte et écrire
  le digest via MCP dans le même tour. Pas de digest canonique prématuré ni
  de décision structurelle prise par substitution.
- Le pointeur wiki est proposé dans le retour, sans écriture hors ownership.
  Son traitement sera arbitré avant intégration ; aucune ingestion wiki
  implicite. Cela ne suspend pas la rédaction du contrat autorisé.

## Limites

Ce rapport n'atteste ni livraison T1 ni synchronisation déjà réalisée. Aucune
permission, racine MCP, règle ou profil modifié. Pas de runtime/Cargo. T2 reste
bloquée jusqu'au contrat effectivement validé et intégré.
