# Comparaison d'inventaires — provenance de la fonction promue

Promue explicitement le 2026-10-28. Ce dossier conserve la provenance historique,
pas du code exécutable. Métadonnées : [candidate.json](candidate.json).

Implémentation : [compare-inventories.cjs](../../scripts/compare-inventories.cjs).
Tests : [compare-inventories.test.cjs](../../scripts/compare-inventories.test.cjs).
Import public `helpers.compareOwnedContainerInventories(inventory, options)`.
CLI : `invoke compare-owned-container-inventories`, stdin `{inventory, options?}`.

Limites et preuves de promotion : [VALIDATION.md](../../VALIDATION.md).
Les IDs et états complets sont requis ; la complétude/propriété sont déclarées
par l'appelant. Aucun Docker, stop ou autorisation de suppression. Un résultat
inconclusive n'est pas un succès de cleanup, même si la CLI sort à 0.
