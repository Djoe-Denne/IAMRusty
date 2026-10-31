# Copie native ciblée autorisée

Après le retrait vérifié de huit copies, l'utilisateur a choisi explicitement
**« Copie native ciblée »** pour les deux copies restantes. Cette décision autorise
une nouvelle lecture/copie par l'outil natif de fichiers ou de shell, limitée aux
32 sources publiques inventoriées dans pool-13 et pool-20, vers une sauvegarde locale.

Cette opération est distincte des anciens appels refusés au lecteur MCP. Aucun
réglage de permissions, configuration MCP ou garde Windows n'est modifié. Tout
nouveau refus sera respecté ; aucune suppression sans récupération complète.

Les logs, profils, configurations de campagne, baux, caches et binaires restent
exclus. Le journal privé de fixtures de pool-20 n'est pas sauvegardé comme source.
Les propositions et scripts WSL seront seulement conservés comme historique :
aucun test, sonde, runtime ou développement suspendu n'est relancé.

Suite autorisée : copie native, comparaison SHA256 et restauration indépendante,
disposition des anciennes versions, retrait des deux seuls dossiers après stabilité,
puis archivage normal sans purge des archives ou références préservées. Le résultat
Windows déjà publié et les limites HTTP/WSL/Linux restent inchangés.
