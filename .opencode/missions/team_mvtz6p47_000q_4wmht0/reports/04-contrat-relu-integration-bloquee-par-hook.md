# Contrat k6 relu — récupération de l'intégration en cours

## Objectif et étape

Le contrat Linux doit être intégré avant le code k6. Les deux documents de l'architecte ont été relus : ils bornent le GET réel, la confidentialité, Chromium sandboxé et la CI sans nouveau laboratoire. Ils ne revendiquent aucune preuve d'exécution et restent Proposed/Unimplemented avant ratification.

## Problème observé

Le retour auteur n'avait pas de commit. Après arrêt du membre, la fusion native a signalé qu'elle ne pouvait rien intégrer ; le worktree et les deux fichiers modifiés sont conservés. Le lead a staged uniquement ces documents puis tenté leur capture. Le hook du dépôt appelle cargo fmt et échoue sous Windows avec une erreur de longueur de nom/extension206.

Une seconde tentative via un lecteur temporaire court, après vérification que Cargo voyait bien ce chemin court, a échoué identiquement. Le lecteur temporaire a été retiré ; aucun fichier n'a été supprimé. Aucun hook ou contrôle n'a été désactivé. Les WIP étrangers restent intacts.

## Suite autorisée

Les deux tentatives sont consignées dans le ledger. Transfert de diagnostic borné à emergency-thinker, avec routage par le catalogue OpenAI approuvé. Chercher une cause et une récupération conformes, pas une répétition plus coûteuse ni un changement produit. Le code consommateur reste bloqué jusqu'à intégration/ratification réelle. Les recherches indépendantes peuvent continuer ; aucun nouveau GO utilisateur n'est nécessaire pour ce diagnostic dans le mandat.

## Preuves et limites

- Deux documents auteur : docs/adr/0504-validation-fonctionnelle-linux-k6.md et docs/contracts/k6-functional-linux.md, 451 insertions/12 suppressions, diff-check réussi avant capture.
- Base auteur9896140 ; aucun commit supplémentaire acquis.
- Hook .githooks/pre-commit : cargo fmt --all -- --check ; deux erreurs206, pas fmt PASS.
- Aucun runtime Docker/Kind/WSL lancé, aucune qualification k6/Chromium effectuée.
- Détails et identités des branches à réconcilier : .cursor/handoffs/20261029-k6-01-socle.md.

## Actualisation — cause discriminée, récupération préservée

Le spécialiste a mesuré la commande rustfmt : 33768caractères depuis le chemin long, au-delà de la limite Windows ; le même contrôle au même HEAD réussit dans le checkout principal avec21864caractères. Cette preuve explique pourquoi l'alias de lecteur n'a pas suffi : il ne raccourcit pas les chemins physiques après canonicalisation.

Les deux documents et l'index auteur ont ensuite été sauvegardés avec empreintes SHA256 vérifiées. Le déplacement Git temporaire du seul worktree de mission vers un chemin physique court a été refusé par le système (Permission denied). Source toujours présente, destination absente ; aucune répétition forcée ni modification de droits. Le même spécialiste reprend un diagnostic en lecture seule pour identifier cette frontière et proposer une récupération conforme. Code consommateur toujours bloqué ; aucune ratification acquise.

La recherche indépendante avance : l'image officielle grafana/k6:2.3.0-with-browser existe et ses digests publics sont identifiés, mais le Dockerfile épinglé définit no-sandbox et installe Chromium sans version de paquet fixe. Aucun pull/run effectué ; cette image n'est donc pas annoncée qualifiée. Docker Desktop était déjà actif, la distribution ARTIST-Ubuntu arrêtée ; aucune ressource de l'utilisateur n'a été adoptée ou arrêtée.
