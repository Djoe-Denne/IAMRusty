# Exclusions récursives avant le build Linux

La mission doit encore qualifier Envoy réel et le parcours navigateur. Avant
de construire leurs images, le contrôle du contexte a trouvé des métadonnées
d'agents imbriquées dans le SDK : le filtre racine seul ne les excluait pas.
Aucun transfert Docker n'a été lancé.

Les deux filtres applicables excluent maintenant ces répertoires à toute
profondeur. Le test renforcé détecte bien l'oubli avant correction ; les 24 tests
de la suite passent ensuite. Le fichier local utilisateur reste inchangé.

Je fige ce delta, actualise la copie de filtre identifiée dans le contexte
possédé, puis passe au build Linux autorisé. Les succès IT existants restent
valides et ne sont pas rejoués. La revue statique ne vaut pas compilation réussie.

## Preuves

- Suite : `python -B -m unittest discover -s ops/tests -p test_local_full_acceptance_source.py` — 24 PASS.
- [Revue du contexte et limite constatée](../../../../.cursor/review-briefings/20261029-infra-delivery-context-4fcca1d.md).
- Aucun code Rust, contrat d'authentification, réglage des 24 jobs ou SDK publié modifié.

## Contexte final figé

Le contexte propre est maintenant au commit cf02c4a, avec SDK 02e5 inchangé.
La copie générée du filtre a été actualisée après contrôle de son ancienne
identité, puis comparée au template final. Le premier build réel portera seulement
sur les artefacts Linux, avec un tag de mission distinct ; l'ancien artefact
de provenance inconnue sera conservé et non utilisé. Les IDs de création seront
inscrits au registre, sans nettoyage automatique de cache.
