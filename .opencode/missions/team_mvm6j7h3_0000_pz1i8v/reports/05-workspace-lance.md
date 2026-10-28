# 05 — Campagne réelle lancée

## Delta et preuves

Préparation terminée en ~39 minutes : compilation des cibles de test indexing
~31 minutes, graph ~6 minutes, binaire ~1 min 40 s. Les tests eux-mêmes prennent
0,03 s + 0,01 s. Trois tests ciblés PASS, trois commandes Cargo exit 0.

Overlay externe testé et exporté : `patches/workspace-exclusions-v1.patch` sous
le dossier privé d'installation. Nouveau binaire côte à côte :
`bin/rust-code-mcp-workspace-v1.exe`, SHA256
`97ca275092dfc8b0cbc92be6471b6d42d323d085081cadf735c7ff6ab66796d3`.
Le binaire upstream initial est conservé. Le code applicatif n'a pas été modifié.

Backend 1MCP ciblé rechargé avec --workspace-v1, PID 39492, wrapper 54600.
Ancien backend quiescent avant reload (zéro cache/tracking/task background).
Timeout MCP propre à ce backend porté à 12 h pour une campagne locale longue.
Aucun autre serveur MCP redémarré.

## Campagne réelle

Shell `sh_16d793ee3001Og5RiempverRSa`, runner PID 62784, début
2026-10-24T10:33:45Z. État observé : index_codebase_running.
Le calcul GPU est effectivement entré dans le premier lot à 10:34:15Z,
lot 1/58 (16 fichiers). Ce n'est pas encore un résultat d'indexation complète.

Paramètres effectifs confirmés dans le log : batch 1, budget 1024 tokens,
Qwen3-Embedding-8B, aucun changement des chunks/dimensions.
La suite du runner est build_hypergraph puis workspace_stats, séquentiellement.
Vérification du slot Cargo avant le graphe, garde SHA256 du Cargo.lock,
cleanup des seuls caches mémoire du workspace à la fin ; aucun index supprimé.

## Ressources et estimation

Machine constatée : Ryzen 7 5800X, 8 cœurs / 16 threads. Le chargement HIR
upstream utilise num_cpus::get_physical(), donc 8 workers ; Rayon du parsing
d'index = 2 (GPU traité séparément). Ce n'est pas un graphe limité à un worker.

Premier relevé après chargement : VRAM totale utilisée 21996 MiB / libre
2327 MiB ; RAM hôte libre 8104 MiB au relevé du moniteur. Ces valeurs sont
globales, pas une attribution précise au processus. FFXII et d'autres apps
graphiques sont ouverts ; aucune application utilisateur n'est arrêtée.

User rappelle de ne pas sous-dimensionner les jobs : dimensionner à partir de
VRAM/RAM et débit mesurés, pas d'un quota arbitraire. Batch 1 est le démarrage
prudent avec ~2,3 Gio libres ; pas d'augmentation à l'aveugle ni redémarrage
destructif d'un index en cours pour un échantillon d'utilisation instantané.
Le batch n'est pas modifiable à chaud dans cette version. Reconsidérer au
prochain point contrôlé, à qualité/chunking identiques, si les mesures le justifient.

Estimation préalable très incertaine : plusieurs dizaines de minutes à quelques
heures. Pas de pourcentage temporel/ETA fiable tant qu'un débit représentatif
de chunks longs et courts n'a pas été observé.

## Suivi durable

`C:/Users/djden/AppData/Local/rust-code-mcp/workspace-job-status.json` : phase,
appels et résultats, identité native, dernière progression, min/peak mémoire.
`logs/workspace-*.metrics.jsonl` : relevés 15 s.
`logs/server-2026-10-24T10-33-07.614Z-39492.stderr.log` : journal natif.
`workspace-preparation.json` : tests/build/patch/binaire vérifiés.

Attendre notification du shell sans polling. Aucun Docker/Kind/WSL lancé ; zéro
conteneur créé. Ni index ni graphe complet annoncé avant leurs résultats réels.
