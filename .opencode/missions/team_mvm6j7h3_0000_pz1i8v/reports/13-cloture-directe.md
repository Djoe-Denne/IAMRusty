# 13 — Clôture directe : corrections et migration validées

## Résultat
Mandat exécuté directement, sans nouveaux agents ni nouvelle mission Ensemble.
Les défauts HIR, exclusions, alias Windows, allocation GPU et incrémentalité
découverts pendant la migration sont corrigés et vérifiés sur leurs scénarios.
Les [rapports 08–12](12-partage-modele-et-reprise.md) gardent la progression,
les essais rouges et leurs limites ; ils ne décrivent pas l'état final.

## Acquis vérifiés
- **914/914 sources** avec metadata persistée conforme au contenu/stat courant,
  collection initiale `code_chunks_55cdd149_06293987` conservée. Les 66 exclusions
  sont récupérées ; un fichier déjà indexé est aussi remplacé par une vue expurgée,
  soit 67 fichiers/1279 chunks traités, sans jeter les 847 sources non affectées.
- **HIR réel corrigé** : 8085 nœuds, 20970 liaisons, 31774 usages. Vérification
  readiness caller/callee inverse et codemap source/span, puis même oracle après
  redémarrage. Ces compteurs ne sont pas tous des appels de fonctions.
- **Alias Windows** en `/`, `\`, verbatim et `/.` : même collection, zéro nouveau
  document embeddé. Après restart, index réel inchangé en 398 ms, fixture en
  2,7 ms ; graphe `reused=true`, sans nouvelle construction HIR.
- **Incrémentalité réelle** : édition, renommage, suppression, recréation avec
  contenu/stat identiques ; vieux résultats absents et nouveaux présents. Le
  scénario v3 termine en ~37 s, avec caches recherche maintenus pendant les edits.
- **GPU** : une seule initialisation Qwen3-Embedding-8B pour recherche/indexation
  multi-workspaces, aucun OOM dans le scénario corrigé. Pool Weak par identité
  exacte, admission sérialisée, derniers clients libérés sans poids épinglés.
- Les échecs d'indexation remontent et n'avancent plus le Merkle comme un succès ;
  remplacement préparé avant suppression des vieux chunks, compteurs unchanged
  séparés des exclusions, lecteur BM25 frais après les commits.
- **Serena** : édition de corps, rename inter-fichiers et insertion réellement
  éprouvés ; probes retirées. Priorités directement dans les profils écrivains
  OpenCode/Cursor/Codex, pas seulement dans AGENTS/règle générale.
- **GrepAI** : retiré du backend 1MCP AIForAll, désactivé dans les trois clients
  locaux ; catalogue OpenCode sans GrepAI. Lifecycle AIForAll arrêté, autres
  projets disponibles ; index `.grepai/`, installation globale et QMD conservés.
- **QMD** : status, recherche et lecture réels réussis, sans réindex du wiki.

## Localisation / impact
Application : `AGENTS.md`, `.cursor/rules/grepai-first.mdc`, configurations MCP
projet, profils `.opencode/agents/`, `.cursor/agents/`, `.codex/agents/`,
`docs/rust-code-mcp.md` et documentation d'installation OpenCode. Aucun code
applicatif, manifeste Cargo ou lock applicatif édité pour cette migration.

Outil externe : `%LOCALAPPDATA%/rust-code-mcp/source/`, base upstream
`dfa9486e4d558cff7739658f645ae4c0f9d37e46`. Helpers paths HIR/indexing,
redaction scanner/chunker, incremental/cache, pool Qwen3 et lecteur BM25.
Overlay complet exporté sans staging : `patches/workspace-corrections-v3/`,
21 fichiers, manifest SHA256 et fichiers nouveaux inclus.

Backend final : `bin/rust-code-mcp-workspace-v3.exe`, SHA256
`dd8647649275f7b295d73acecb396efcc1b5867ffe537356d283cc871b56a28f`,
1MCP projet port 3062 ; restart vérifié PID 54960 → 41512. Modèle local 8B,
CUDA/F16/4096 dimensions inchangés, pas d'API payante ni de fallback plus petit.

## Preuves et QA bornée
Sous `C:/Users/djden/AppData/Local/rust-code-mcp/` :
- `corrections-preparation-v2.json`, `corrections-preparation-v3.json` et logs
  TestEmbeddingPool : checks natifs focalisés et build, jobs=3, un slot Cargo.
- `workspace-v2-verification.json` : HIR/récupération PASS, puis preuve rouge OOM
  fixture conservée ; ne pas confondre avec le résultat v3.
- `workspace-v3-verification.json` : **complete, 18 assertions**, une initialisation
  8B, zéro OOM. `workspace-persisted-metadata-v3.json` : **914/914, zéro embedding**.
- `restart-v3-runtime-proof.json`, `restart-v3-persistence-proof.json`,
  `restart-v3-query-proof.json` : index/graphe/recherche après restart réel.
- `serena-edit-proof.json`, `qmd-migration-proof.json`, `qmd-document-read-proof.json`.
- `migration-static-verification-v2.json`, `migration-final-proof-v3.json`.

Tests pertinents : HIR9, paths2, sécurité22, pool4, BM25 fraîcheur1, erreurs/
snapshot incrémental4 ; contrôles de chunks/compteurs et audits dédiés exécutés.
Trois tests upstream nécessitant un modèle restent `ignored` dans le filtre
incrémental natif ; ils ne sont pas annoncés PASS. Les scénarios réels ci-dessus
ont été exécutés séparément avec le modèle8B, pas remplacés par ces skips.
Auteur de correction et validation = même agent, pas de revue indépendante.
Aucune suite applicative complète lancée, car aucune logique applicative changée.

## Intégrité, cleanup, rollback
- Base Git applicative `93e376c5530a439aaafe61a64d3e881865c1acf1` inchangée.
  Les **989 sources/manifests/locks** du manifeste initial restent présents et
  SHA256 identiques ; changements utilisateur antérieurs conservés.
- Aucune opération commit/stash/reset/merge/push. Backups privés des 6 configs
  pré-migration avec hashes ; données pré-v2 copiées et vérifiées sur 310 fichiers.
  Binaires upstream/v1/v2 conservés. Restaurer seulement après comparaison ciblée,
  jamais globalement sur des changements plus récents.
- Dernier état réel : caches recherche/sémantiques et suivi0, background sync
  arrêté ; poids libérés. VRAM globale au relevé final : 3234 MiB utilisés,
  **21089 MiB libres** (pas une attribution par PID). Backend partagé conservé
  disponible à la demande, pas de preview ou job de compilation laissé actif.
- **Aucune ressource Docker créée, aucun Kind/WSL démarré** pour ce travail.

## Limites non bloquantes / hors périmètre
Le graphe HIR garde les limites documentées de macros/features/cfg et dispatch
dynamiques/impls de traits : aucun résultat vide ne prouve l'absence d'appelants.
Le scanner est heuristique, pas une garantie de détection universelle des secrets.
L'audit de tout le graphe ou de tous les secrets n'a pas été demandé ni prétendu.

Obsidian est fermé : son CLI ne peut pas joindre l'application. Le vault n'a
pas été modifié ni l'application forcée au démarrage ; runtime GUI **non vérifié**,
mais QMD a bien retrouvé/lu les documents du vault. La commande `opencode` n'est
pas sur le PATH du shell : pas de preuve `mcp list` CLI, remplacée par les appels
réels de la session OpenCode et du client HTTP partagé. Aucun incident bloquant
restant dans le périmètre fonctionnel vérifié de cette migration.
