# Décodage strict — provenance de la fonction promue

Promue explicitement le 2026-10-28. Ce dossier conserve la provenance historique,
pas du code exécutable. Métadonnées : [candidate.json](candidate.json).

Implémentation : [decode-evidence.cjs](../../scripts/decode-evidence.cjs).
Tests : [decode-evidence.test.cjs](../../scripts/decode-evidence.test.cjs).
Import public `helpers.decodeEvidenceText(bytes, options)`. Module seulement,
pour conserver Buffer/Uint8Array sans conversion implicite via JSON.

Limites et preuves de promotion : [VALIDATION.md](../../VALIDATION.md).
Sans BOM auto utilise UTF-8 ; UTF-16 doit sinon être explicite. Aucun fallback
Latin1, remplacement silencieux ou normalisation de retours ligne. Aucune lecture
de fichier, expurgation de données privées ou interprétation du verdict de test.
