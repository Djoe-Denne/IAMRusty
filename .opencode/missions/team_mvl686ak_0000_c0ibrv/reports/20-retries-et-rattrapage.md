# 20 — Retries et nécessité de rattrapage

## Vérification sans interruption
Question utilisateur : retries sur les290erreurs et nécessité d'une deuxième
passe. Source exacte du binaire installé consultée ; GET authentifié
`/router/settings`, sans génération, arrêt, redémarrage ou modification.

## Faits vérifiés
- LiteLLM actif : **num_retries=2**, retry_after=0. Les tentatives internes
  dépendent des classes d'erreur et de la politique native ; ce réglage ne prouve
  pas que chacun des290échecs a fait exactement deux nouvelles tentatives.
- Les erreurs comptées dans les logs HTTP du proxy sont les réponses finalement
  rendues à GrepAI, pas un décompte des tentatives internes du proxy.
- `rpg/extractor_llm.go:callCompletion` : un seul client.Do, erreur sur HTTPnon200 ;
  pas de boucle de retry ni file de rattrapage dans GrepAI.
- `ExtractAtomicFeatures` utilise l'extracteur local sur erreur/réponse invalide.
  Un élément peut donc être présent mais non enrichi par Luna ; repli pas toujours
  journalisé individuellement. `GenerateSummary` propage l'erreur : résumé vide.
- `summary.go` abandonne le reste d'un niveau après trois échecs consécutifs.
- `RefreshDerivedEdgesFull` peut réessayer les résumés hiérarchiques vides, pas
  toutes les features locales ni tous les résumés de fichiers. Le moniteur finit
  à readiness : aucun rattrapage complet garanti par ce mécanisme.

## Conclusion / limites
Prévoir un rattrapage des éléments restés incomplets. Les290réponses d'erreur ne
correspondent pas nécessairement à290éléments distincts ou290résumés absents.
L'audit final sur copie recense les résumés manquants, sans certifier la provenance
Luna de chaque feature remplacée localement.

La CLIv0.37 n'expose pas la reprise ciblée d'une liste d'appels échoués. Ne pas
lancer automatiquement une reconstruction complète supplémentaire ni promettre
un rattrapage ciblé déjà disponible. Un ciblage fiable et un marquage des échecs
nécessiteraient un changement dédié à évaluer séparément.
La campagne actuelle continue ; aucun retry/cap ajouté.
