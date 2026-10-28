# Validation du skill d'ingestion

## Vérifié le 2026-10-25

- Syntaxe des2fichiers CommonJS : node--check, exit0.
- Windows/Node24.11.1 :
  `node --test --test-reporter=tap scripts/inspect-trace.test.cjs` :
  **19tests PASS,0FAIL,0SKIP**,252,0632ms observées (pas un benchmark).
- Chargement explicite du skill par son ID : réussi. Le frontmatter manuel utilise
  disable-model-invocation:true, documenté par OpenCodeV2 ; il n'est pas un verrou
  de sécurité et aucune invocation spontanée d'un autre modèle n'a été testée.
- Smoke réel via Context Mode/Bun : capture existante293961octets,52messages,
  87toolcalls ;21records Code Mode inventoriés,3renvoyés dans la fenêtre demandée,
  troncature explicite. Les3records sont marquésunattributed, pas « validés ».
  Pas d'entrée/code/résultat métier émis ni de script extrait exécuté.
- Source du smoke SHA256 :
  d8b33fb046ef5d3eaae860fa50431a8fd729f91380899da6de157b0c1bdc23ec.

## Couverture

Inventaire natif/snapshot/pageAPI, états d'erreur/exitcode/pending, completed non
assimilé à un succès, rejets/isError structurés attribuables manuellement, exemples
de texte contenant « error » sans faux verdict. Prompts, réponses, raisonnements,
corps de résultats et messages d'exception non recopiés ; faux tools d'un user ignorés.
Code source sentinelle contenant une écriture : jamais exécuté. Pas d'import de
candidates ou de modification du catalogue. Doublons exacts séparés par runtime/
type, inventaire paginé, complétude non inventée, formes invalides/non supportées,
fichier absent/relatif/trop gros, binaire/UTF-8/JSON invalide, CLI/erreurs bornées.
Fixtures uniquement temporaires possédées, retirées par le teardown.

## Limites

Revue par l'auteur, pas review indépendante. Pas d'ingestion effective d'une
mission ni de candidate extraite/promue dans ce lot : le smoke teste l'outil.
Pas de parseur de JavaScript imbriqué, détection sémantique, déduplication floue,
preuve de succès métier ou anonymisation universelle. Les erreurs/corrections
et les candidats restent à attribuer/revoir par l'agent dans le mandat autorisé.
Node18minimum prévu par les API, seuls Node24/Bun testés ; pas de qualification
Linux/macOS, Cursor/Codex/JSONL ou campagne de comportement LLM. Aucune garantie
de gain de coût ou de qualité. Pas d'accès DB/API caché ou lancement d'un modèle.
