# Nettoyage et archivage vérifiés

## Résultat

**Les dix copies temporaires autorisées sont retirées et l'équipe est archivée.**
Les sauvegardes récupérables et les 11 références Git préservées sont conservées.
Le checkout principal, ses modifications étrangères, le sous-module, les outils
installés et les données WSL restent en place. Aucun runtime n'a été démarré.

Le résultat fonctionnel Windows reste **PASS** : cas minimal about:blank/42,
k6 2.3.0 et Chrome for Testing 156.0.8078.73. Les **34 tests Windows sans skip**
et le vrai cas navigateur ne sont pas rejoués pour ce nettoyage ; leurs preuves
restent valides, les six fichiers de code/tests et les trois sources du run étant
inchangés par comparaison SHA256.

Ce bilan clôt le nettoyage autorisé, **pas la qualification globale mission1** :
HTTP/selftests restent INCONCLUSIVE, WSL est suspendu, Linux reste non qualifié.
Le PASS navigateur n'est pas une certification du sandbox ni un benchmark.

## Préservation du travail

- 79 fichiers modifiés ou nouveaux sauvegardés entre les dix copies, avec leurs
  versions de base/index disponibles ; **141 versions et leurs restaurations**
  comparées par SHA256. Quatre patchs publics de livraison sont également conservés.
- Cinq sources WSL tardives jusque-là non livrées sont publiées comme textes
  d'archive inactifs, avec un manifeste. Aucune exécution ni qualification déduite.
  Les 24 sources archivées sont toutes vérifiées par taille et SHA256.
- Anciennes propositions et versions remplacées : disposition explicite, sans
  fusion aveugle, rétrogradation de contrat ou réintroduction des gardes abandonnées.
- L'outil de nettoyage natif a retiré les références préservées malgré l'absence
  de purge demandée. Ce défaut a été détecté et corrigé : les **11 noms et commits
  exacts sont restaurés et vérifiés**, sans toucher à HEAD. Voir le rapport106.

## Vérifications de clôture

Les dix dossiers sont absents du disque, de Git et de l'inventaire natif OpenCode.
Les branches de checkout temporaires correspondantes ne subsistent plus ; seules
les références de sauvegarde explicitement conservées restent. Les onze commits
préservés sont déjà inclus dans master. Le seul worktree restant est le principal.

Le nettoyage natif est exécuté sans force ni purge. Une lecture seule de la base
confirme `team.status=archived`, 23 membres shutdown, 22 tâches completed et une
cancelled. Le ticket Linux différé a été annulé par cet archivage sans être publié
ou exécuté. Le champ interne `mission_state=running` est conservé par le mécanisme
natif ; il ne désigne pas un worker ou runtime actif et n'a pas été modifié à la main.

`git diff --check` et les hooks normaux de formatage réussissent. Aucun diff
de source, contrat ou workflow n'est produit par les « safety-net merges » natifs.
Les seules modifications étrangères suivies restantes concernent Obsidian et
rustycog, laissées intactes ; HEAD du sous-module inchangé. Aucune donnée privée,
configuration de campagne, journal, profil, bail ou binaire n'est publié.

Le code fonctionnel est publié dans `201ec05b` ; les archives tardives dans
`a24a7ce5`, tous deux sur master, référence distante vérifiée. Ce bilan documentaire
est ensuite committé et publié avec les hooks normaux, puis sa référence distante
est vérifiée. Le reçu local de cette dernière publication donne le commit exact.

## Récupération et preuves

Sauvegardes locales conservées sous `.tmp/k6-closeout-backup-20261031/` et
`.tmp/k6-closeout-final-backup-20261031/`, hors des dossiers retirés. Elles ne
doivent pas être incluses dans un nettoyage générique. Leurs manifestes et copies
`verified-restoration` décrivent précisément comment retrouver chaque version.

Preuves privées : `.tmp/k6-preserved-refs-restored-20261031.json`,
`.tmp/k6-closeout-proof-reuse-20261031.json` et
`.tmp/k6-closeout-final-state-20261031.json`. Dernière publication :
`.tmp/k6-closeout-final-publication-20261031.json`.

Localisation : snapshots des anciennes copies et archive documentaire WSL.
Impact : aucun comportement produit ou scénario actif changé par ce nettoyage.
Limites : celles du rapport99 restent ; aucun nouveau test réel HTTP/Linux/runtime.
