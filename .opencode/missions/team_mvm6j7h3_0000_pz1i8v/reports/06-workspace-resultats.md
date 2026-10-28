# 06 — Index et graphe réels : résultats et réserve de couverture

## Résultat du lot

Campagne exécutée du 2026-10-24T10:33:45Z au 10:58:16Z : ~24 min 32 s,
hors préparation du binaire et contrôles ultérieurs. État du runner complete,
index puis graphe séquentiels, aucune erreur GPU ou ligne ERROR observée.
Cela ne clôt pas toute la migration ; la couverture des appels HIR est en défaut
sur un oracle concret décrit ci-dessous.

## Index 8B

- Durée : 1333,10 s, soit 22 min 13 s.
- 914 fichiers Rust candidats, 848 indexés, 66 non indexés.
- 13039 chunks / embeddings ; Qwen3-Embedding-8B local Candle/CUDA, F16,
  4096 dimensions, chunks 768/1024, batch 1, budget 1024 tokens.
- Temps embedding mesuré 1276,57 s, parsing 20,72 s ; ~10,2 chunks/s sur la
  phase embedding. Pas de modèle inférieur ou d'API d'embedding distante.
- Les 1029 sauvegardes .tmp ont été exclues avant indexation.
- Répartition des 66 exclusions, vérifiée dans les logs : 30 détections
  heuristiques de contenu sensible, 21 filtrages de chemins/noms, 15 fichiers
  sans chunks. Ce ne sont PAS 66 secrets avérés. Certains fichiers de production
  sont concernés (dont apparatus-operator/src/controller.rs) : couverture non
  exhaustive, à auditer sans désactiver les protections globalement.
- Recherche réelle après redémarrage : description d'un fallback de transport
  de queue/no-op/readiness -> signal_queue_status en premier, puis is_degraded,
  tests de readiness, classify_transport, etc. Résultats plausibles selon source.
  Aucun réindex automatique signalé lors de cette recherche.

## Graphe persistant

- Durée : 137,81 s, soit 2 min 18 s.
- 4571 nœuds : workspace 1, cibles crate 76, modules 671, items 3305,
  symboles externes 518. Les 76 cibles ne sont pas 76 packages : le workspace
  Cargo possède 65 packages et certaines cibles bin/lib distinctes.
- 21028 liaisons (déclarations/imports), 2680 références d'usage. Ne pas présenter
  ces 23708 relations comme autant d'appels de fonctions.
- Graph ID `0224c9ef7e7146eb56b0bfd95879db0f`, fingerprint
  `d864c12475ce989149ef35fe73fb16b0b60ee24299ab2862d247393d6c896477`.
- Snapshot sous `%APPDATA%/rust-code-mcp/search/data/graphs/` ; aucun LLM
  génératif pour produire les relations, extraction HIR rust-analyzer.

## Reprise après redémarrage réel du backend

Backend MCP ciblé remplacé PID 39492 -> 62308, sources/binaire/chunking identiques,
marqueur --post-index-proof uniquement pour la recharge. Autres MCP préservés.

- index_codebase : 914 fichiers inchangés, 0 modifié/indexé, 0 nouveau chunk,
  241,33 ms ; aucun ré-embedding de documents. Le zéro chunk est un delta,
  pas une perte des 13039 vecteurs persistants.
- build_hypergraph force_rebuild=false : reused=true, même graph ID, mêmes
  fingerprint et compteurs. Pas de reconstruction HIR.
- Recherche vectorielle/hybride ensuite réussie sur le vrai index existant.
- Cela valide la reprise de ce workspace inchangé avec une notation de chemin
  constante. Les tests de modification/suppression et les alias de chemins
  restent distincts, non couverts par ce résultat.

## Défaut observé : appels HIR manquants

Déclarations/imports valides : who_imports(classify_consumer) retourne bien
readiness::factory et le réexport readiness, comme dans le code.

Mais calls_from(create_signaled_event_consumer) renvoie 0 alors que factory.rs
appelle explicitement classify_consumer et signal_queue_status (lignes 78-79).
who_calls et who_uses(classify_consumer) renvoient également 0. Le moteur
syntaxique get_call_graph retrouve les noms attendus sur ce même fichier ; il
ne remplace pas une résolution HIR fiable. ready_response -> probe.report est
également absent dans la requête ciblée.

Conclusion bornée : le snapshot a été créé et est réutilisable, les imports
fonctionnent, mais les graphes d'appels ne sont PAS validés pour une analyse
d'impact autonome. Un résultat vide n'est pas preuve d'absence d'appelants.
Cause non établie ; frontier `rmc-graph/src/graph/usages.rs` (scope/extraction/
attribution) et queries/calls.rs. Ne pas attribuer arbitrairement aux macros.
Aucune tentative de réparation ni nouveau rebuild n'a été lancée sur ce défaut.

Suite ciblée recommandée : reproduire la référence importée dans un mini-crate
sans macro, instrumenter les filtres de références/scopes, ajouter une régression
avant correction. Préserver le snapshot actuel et les embeddings ; si une
correction est nécessaire, reconstruire seulement le graphe après sauvegarde,
pas l'index 8B. Recouper les références critiques avec Serena en attendant.

## Ressources / intégrité

- 88 mesures pendant l'index : utilisation GPU GLOBALE moyenne 93,8 %, médiane
  97 %, maximum 100 %. Mesure globale, pas attribution par PID ; l'utilisateur
  a fermé FFXII pendant la campagne. Le batch 1 n'a pas laissé le GPU au repos.
- Pic VRAM total 22185 MiB, minimum libre 2138 MiB ; minimum RAM hôte libre
  4883 MiB. Aucun OOM observé.
- Après vérifications, cache mémoire du seul workspace libéré sans suppression
  d'index : GPU total 2668 MiB utilisés / 21655 MiB libres au relevé final.
- Les 989 fichiers sources/manifests/locks du manifeste de préflight ont leurs
  SHA256 inchangés, aucun manquant. Cargo.lock inchangé. Aucune modification
  applicative, aucun Docker/Kind/WSL lancé, zéro conteneur créé.

## Preuves

Sous `C:/Users/djden/AppData/Local/rust-code-mcp/` :

- workspace-job-status.json et logs/workspace-*.metrics.jsonl.
- workspace-skipped-files.json (registre des 66 exclusions).
- workspace-restart-index-result.json / workspace-restart-graph-result.json.
- workspace-semantic-search-result.json / workspace-importers-result.json.
- workspace-graph-validation.json (oracles contradictoires, transcrits des appels).
- workspace-final-verification.json (ressources et intégrité des 989 entrées).
- logs/server-2026-10-24T10-33-07.614Z-39492.stderr.log et nouveau server-runtime.json.

GrepAI reste actif provisoirement, Serena/QMD/Obsidian inchangés. Routage et
profils agents d'écriture restent à migrer ; pas de déclaration de mission totale
terminée tant que la couverture requise n'est pas corrigée ou bornée explicitement.
