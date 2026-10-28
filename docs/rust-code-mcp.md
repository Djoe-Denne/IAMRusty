# Intelligence du code Rust — AIForAll

Décision du 2026-10-24 : rust-code-mcp pour la recherche Rust et les relations
HIR, Serena pour la navigation et l'édition symboliques ; QMD/Obsidian restent
la voie des connaissances. GrepAI est retiré **uniquement d'AIForAll**, ses
index historiques conservés. Canon : bloc **Contrat MCP local** d'`AGENTS.md` ;
méthode : `.agents/skills/aiforall-mcp-workflow/SKILL.md`.

## Installation locale

- Un backend natif Windows partagé par 1MCP :
  `http://127.0.0.1:3062/mcp?tags=rust-code-mcp`.
- Sources/build/binaire/rollback hors application sous
  `%LOCALAPPDATA%/rust-code-mcp/`. Base upstream épinglée
  `dfa9486e4d558cff7739658f645ae4c0f9d37e46`, overlay local exporté et testé.
- Qwen3-Embedding-8B local, Candle/CUDA/F16, 4096 dimensions ; shards vérifiés.
  Pas d'API d'embeddings payante ni de fallback CPU/4B silencieux.
- Batch 1 / budget 1024 tokens, chunks 768/1024 conservés. Le premier run a
  mesuré 97 % d'utilisation GPU globale médiane ; ne pas augmenter les batches
  sans marge VRAM et mesure de débit. Cargo Windows : 3 jobs, un seul slot.
- Données sous `%APPDATA%/rust-code-mcp/search/data/` ; ne pas supprimer index,
  metadata ou snapshots pour contourner un problème de chemin. Les alias
  Windows sont normalisés tout en gardant la collection installée.

## Utilisation

### Instructions par client

`AGENTS.md` contient les obligations communes et les replis autorisés ; charger
`aiforall-mcp-workflow` avant exploration Rust/impact/édition symbolique. Les
profils rappellent leur rôle, sans recopier la procédure. Le MDC historique
`grepai-first.mdc` est seulement l'adaptateur Cursor, pas un second canon.
OpenCode découvre déjà `.agents/skills` via sa configuration ; si un client
n'expose pas le skill, lire son chemin local par un outil autorisé et le signaler.
Les instructions locales précisent les exceptions de procédure, pas un override
des permissions ou des instructions supérieures. Les réglages globaux restent
intacts. En cas de panne réelle de Context Mode, accord avant poursuite sans lui.

### Maintien en VRAM et commande manuelle (décision utilisateur 2026-10-24)

Le8B reste désormais en cache après les tâches et les périodes d'inactivité :
keepalive explicitement demandé, propre à AIForAll. Les déchargements de fin de
campagne précédents étaient volontaires ; le cache n'a pas de TTL automatique.
Cette exception est définie dans AGENTS et rappelée par le skill, l'adaptateur et
les profils écrivains. Les configurations/skills globaux QMD/Obsidian/Serena ne
changent pas.

Commande disponible dans **CMD et PowerShell**, quel que soit le répertoire courant :
- `aiforall-mcp-vram status` : état du cache8B du backend et mémoire GPU globale.
- `aiforall-mcp-vram free` : libérer manuellement les caches mémoire de ce backend
  Rust AIForAll seulement ; **aucun index/graphe/modèle sur disque n'est supprimé**.
- `aiforall-mcp-vram warm` : précharger une fois le8B depuis l'index existant et
  le conserver dans le cache serveur. `--json` est disponible pour les scripts.

Pas de nouveau daemon, timer ou service Windows, pas de modification du PATH :
wrapper `.cmd` dans `%USERPROFILE%/.local/bin`, déjà au PATH, script privé sous
`%LOCALAPPDATA%/rust-code-mcp/vram-control.mjs`. Le client se déconnecte après
chaque commande ; le cache est conservé par le backend, pas par un processus CLI.

`free` ne coupe pas les requêtes en cours : un indexeur/recherche actif peut
retarder la libération de ses références. Le prochain appel sémantique ou `warm`
rechargera le modèle ; aucune boucle ne le recharge juste après `free`.
Après arrêt/restart du backend, de Windows ou du pilote GPU, premier usage ou
`warm` requis. « Résident » n'est pas une garantie de survie à ces événements.

Cycle réel CMD/PowerShell validé : **21 assertions**, cache8B conservé après la
sortie du client, libéré par `free` puis rechargé par `warm`, même backend PID.
Les **655 fichiers persistants** et les **989 entrées applicatives** comparés
restent strictement identiques. Preuve privée `resident-vram-verification.json`.
État final volontaire : **8B chargé**, pas de cleanup automatique de ce modèle.

Découvrir les outils/signatures dans le catalogue. Pour une recherche/indexation,
préciser `embedding_profile="local-qwen3-8b"` et la racine
`C:/Users/djden/source/repos/AIForAll`. Ne pas demander `force_reindex=true`
pour vérifier simplement la reprise. Le background sync est désactivé.

Impact : `who_uses`, `who_calls`, `calls_from`, imports, codemap par noms
qualifiés. Le graphe est HIR rust-analyzer, pas une reconstruction générative.
Un résultat vide ne prouve pas l'absence de consommateurs ; vérifier couverture,
features, macros, cfg et tests. Recouper avec Serena selon le risque. Certaines
résolutions dynamiques et corps d'implémentations de traits restent limités.

Édition : charger `serena`, vérifier racine/version sans réactiver le projet
partagé, lire corps et références/implémentations. Préférer `rename_symbol`,
`replace_symbol_body`, `insert_before_symbol`/`insert_after_symbol` pour leur
opération réelle, si adaptée et permise par le client. Patches pour petits edits intra-corps, nouveaux fichiers,
docs/config/générés ou sources hors couverture. Les tests restent nécessaires.

## Sécurité et couverture

`.tmp`, `.worktrees` et sorties de build sont exclus du parcours. Les répertoires
de secrets/credentials et clés privées restent exclus. Les fichiers Rust dont
le nom évoque les credentials ne sont pas assimilés à un coffre de secrets :
ils passent une vue expurgée, comme les sources détectées par le scanner.

Le masque retire le contenu des chaînes/commentaires dans la vue indexée,
conserve offsets et lignes, puis rescane. Source malformée ou détection résiduelle
→ refus. Le code applicatif original n'est pas réécrit. Le scanner reste une
heuristique, pas une garantie de détection de tout secret imaginable. Le replay
de politique remplace les seules vues concernées et récupère les exclusions,
sans jeter tous les vecteurs non affectés.

## Preuves et retour arrière

Ledger : `.cursor/handoffs/20261024-rust-code-mcp-installation.md`.
Rapports numérotés : `.opencode/missions/team_mvm6j7h3_0000_pz1i8v/reports/`.
La reprise corrective est directe, sans nouveaux agents Ensemble.

Preuves privées sous l'installation : `corrections-preparation-v2.json`,
`workspace-v2-verification.json`, `workspace-source-audit-v2.json`,
`serena-edit-proof.json`, `qmd-migration-proof.json`, `data-backup-v2.json`.
Les tests préalables ne doivent pas être confondus avec les résultats live.

Rollback : binaires upstream/v1 conservés, sauvegarde de données vérifiée et
configurations ciblées avec SHA256. Comparer les changements ultérieurs avant
toute restauration ; aucun reset/stash global, aucun effacement de `.grepai/`.

## Validation finale du 2026-10-24

- Backend actif v3, anciens binaires conservés ; overlay complet reproductible
  `patches/workspace-corrections-v3/` sous l'installation privée.
- Métadonnées persistées à jour pour **914/914 sources**, collection initiale
  `code_chunks_55cdd149_06293987` conservée. Récupération ciblée de 67 fichiers
  (66 exclus auparavant, une vue nouvellement expurgée), pas de ré-embedding global.
- Graphe corrigé : 8085 nœuds, 20970 bindings, 31774 usages ; appels readiness
  vérifiés dans les deux sens et source/span du codemap restaurés.
- Fixture réelle : édition, renommage, suppression et recréation avec mêmes
  contenu/stat PASS, tout en conservant les caches de recherche. Les recherches
  BM25 voient immédiatement les commits ; un échec d'indexation remonte au client
  et n'avance plus silencieusement le Merkle.
- Poids Qwen3 partagés par identité exacte, admission d'initialisation sérialisée,
  références faibles seulement dans le pool. Une initialisation 8B pour le scénario
  multi-workspaces, aucun OOM. `clear_runtime` libère les caches/poids sans effacer
  les données ; ne pas le confondre avec `clear_cache`.
- Redémarrage réel du backend : index réel inchangé en ~398 ms, fixture en
  ~3 ms, graphe réutilisé, documents supprimés toujours absents des recherches.

Preuves finales : `workspace-v3-verification.json` (18 assertions),
`workspace-persisted-metadata-v3.json`, `restart-v3-*-proof.json`,
`migration-final-proof-v3.json`. Bilan : rapport `13-cloture-directe.md`.
Les 989 entrées applicatives/manifests/locks du manifeste initial restent
inchangées. Validation ciblée par le même agent, pas une review indépendante ni
un audit exhaustif des macros/dispatch dynamiques. QMD testé en lecture ; le
runtime Obsidian n'a pas été validé, l'application étant fermée.
