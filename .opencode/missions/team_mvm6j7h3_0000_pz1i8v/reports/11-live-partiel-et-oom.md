# 11 — Réel : HIR restauré, récupération acquise, défaut GPU découvert

## Résultats acquis (pas à rejouer)
Preuve privée `workspace-v2-verification.json`, run du 12:04:08 au 12:12:45 UTC.

- Graphe forcé après backup : **8085 nœuds, 20970 bindings, 31774 usages**.
  Appels sortants classify_consumer/signal_queue_status, appelant inverse et
  codemap avec fichier voisin vérifiés sur readiness réel. Ancien snapshot
  sauvegardé ; même identité de workspace/fingerprint, extraction corrigée.
- Replay source : **67 fichiers indexés, 1279 nouveaux chunks** en 167 s.
  Les 66 exclusions initiales + une vue désormais expurgée sont concernées.
  Les 847 autres sources n'ont pas été ré-embeddées. Le compteur les présente
  incorrectement comme « skipped » au lieu de « unchanged » : fix local en cours.
- Alias `C:\...`, `C:/.../.`, `\\?\C:\...` : même collection
  `code_chunks_55cdd149_06293987`, aucun document embeddé, ~318–352 ms.
- Recherche réelle readiness pertinente, sans réindex automatique.

## Défaut bloquant découvert sur fixture isolée
Après recherche mise en cache puis édition d'un fichier, l'index incrémental
supprime ses vieux chunks puis échoue à charger le modèle :
`CUDA_ERROR_OUT_OF_MEMORY` (log natif 2026-10-24T12:12:42.157Z).
L'API renvoie pourtant « No files needed indexing updates » et sauvegarde le
Merkle courant : erreur de ressources masquée en succès, remplacement absent.
La fixture a donc bien arrêté la validation. Les 14 assertions précédentes
restent valides ; rename/delete/recréation non exécutés à ce point.

## Cause et correction bornées
- Constat source : `EmbeddingGenerator::with_backend` alloue un Qwen3 distinct
  à chaque construction ; les caches recherche conservent leurs propres Arc.
  Ce chemin peut empiler des poids 8B, indépendamment du batch=1.
- Pool faible partagé par identité exacte + force_cpu, admission sérialisée,
  modèle déjà protégé par Mutex. Pas de fallback, changement de modèle/dimension
  ni rétention forte globale : le dernier client libère les poids.
- Remplacement préparé avant suppression des vieux chunks ; invalidation de la
  seule metadata pour forcer la relecture. Les erreurs hard remontent, donc le
  nouveau Merkle n'est plus sauvegardé comme un succès après échec d'indexation.
- Compteurs : fichiers inchangés séparés des exclusions, sans warnings faux.

Tests du pool en compilation, shell sh_16dd6a837001sVTHB3htaaxP0q, jobs=3.
Une seule tentative de correction de ce défaut, pas d'échec répété sans progrès.
Les caches des deux workspaces ont été libérés par le finally de vérification.

## Suite
Valider pool/erreur/compteurs puis binaire v3 côte à côte ; reprendre uniquement
la fixture et la reprise après restart, **pas** reconstruire le graphe ni rejouer
les 67 sources déjà récupérées. Conserver preuve rouge et sauvegardes.
Pas de décision supplémentaire requise ni d'agent/d'infrastructure ajouté.
