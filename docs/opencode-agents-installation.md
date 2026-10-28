# Définitions d'agents et MCP sous OpenCode

Routage Ensemble mis à jour le 2026-10-10. Origine : portage des définitions Cursor
(`.cursor/agents/*.md`, `.cursor/mcp.json`) et Codex (`.codex/agents/*.toml`)
vers les formats OpenCode V2 le 2026-10-02, puis politique OpenCode dédiée.
`/nul` et `.tmp*` restent hors périmètre.

## Layout OpenCode de ce dépôt

| Emplacement | Rôle |
|---|---|
| `opencode.jsonc` | Config projet : `default_agent`, MCP `rust-code-mcp` / `serena` / `qmd` / infrastructure locale, `grepai` désactivé, timeout `startup: 150000`, `skills: [.agents/skills, .cursor/skills]` |
| `.opencode/agents/*.md` | 20 définitions spécialisées, format V2 : `description`, `mode`, `model`, `permissions`, corps = prompt système |
| `~/.config/opencode/opencode.json` | Config globale (toutes machines/projets) : MCP `qmd`, note plugins rtk-hook |

Les skills sont découverts automatiquement depuis `.agents/skills/` et
`.cursor/skills/` via la clé `skills` de `opencode.jsonc`.

## Ensemble : continuité sans coordinateur intermédiaire

La session racine utilise `build` par défaut. Elle travaille directement ou
devient lead Ensemble selon les lots réellement indépendants. Les définitions
OpenCode `orchestrator` et `agent-harness` sont retirées ; les autres clients
conservent leurs profils historiques. Canon : `AGENTS.md` et le complément
OpenCode de `.cursor/rules/00-agent-harness.mdc`.

Le plugin local est déjà configuré globalement depuis
`C:/Users/djden/source/repos/opencode-ensemble-session/dist` et les skills depuis
le dossier `skills` du même checkout. Aucun doublon local n'est ajouté. Le skill
`opencode-ensemble` et les outils `team_*` sont présents au catalogue observé.
Cette configuration globale reste hors périmètre d'écriture du dépôt.

- Invoquer `@opencode-ensemble <mission>`. Pas de taille d'équipe ni de pipeline
  explorer/architecte/implementer obligatoire.
- Continuer avec le même membre, ou changer de profil par handoff. Pour une
  branche indépendante, hériter des découvertes par fork/checkpoint ; utiliser
  compact_fork uniquement depuis une source distincte et inactive, pas pour
  chaque enfant. Lire la référence de continuité du skill avant ces opérations.
- Un fork n'est pas une mémoire live partagée. Le lead persiste les deltas dans
  un ledger canonique passé par chemin absolu et transmet les changements aux
  membres concernés. Les fichiers gitignored ne suivent pas les worktrees.
- Reviewers fresh avec diff/contrats/faits, sans conclusions du développeur.
  Le lead conserve intégration, validation finale, slot Cargo unique et baux infra.
- Les index canoniques n'incluent pas les modifications non mergées des
  worktrees temporaires : inspecter leurs fichiers/symboles. Héritage vers un
  worktree seulement avec arbres propres et HEAD compatibles ; aucune mise à
  l'écart automatique des changements utilisateur ni fallback silencieux.
- Le bridge 1MCP mutualise les MCP ; il est distinct du bridge HTTP vers le
  **même serveur OpenCode** pour le fork natif Ensemble. Le fork local découvre
  maintenant automatiquement le serveur partagé existant et ses headers via
  l'API publique OpenCode, en mémoire, sans démarrage de service ni secret dans
  le dépôt. URL/auth explicites restent réservées aux serveurs privés. La
  découverte authentifiée réelle et le fork d'équipe sont vérifiés : deux
  agents read-only accèdent au MCP Kubernetes ; l'enfant restitue une balise
  fournie seulement au parent depuis son historique natif hérité. Cela ne
  valide pas l'accès à l'API du cluster, précédemment indisponible.
- L'héritage évite de refaire l'analyse, mais peut encore facturer l'historique
  en entrée. Aucun facteur d'économie garanti sans mesure.

## Politique modèles (utilisateur, 2026-10-03)

Modèles privilégiés — rien d'autre :

- **GLM 5.3 Flash** : `zai-coding-plan/glm-5.3-flash` (principal, éco) et
  `opencode-go/glm-5.3-flash` (secours).
- **OpenAI GPT‑6 / 6.1** : `openai/gpt-6.1-sol`, `openai/gpt-6-luna`,
  `openai/gpt-6-astra`.

Mapping actuel OpenCode :

- `implementer` → `openai/gpt-6.1-sol#medium`.
- `hard-implementer` → `openai/gpt-6.1-sol#high`.
- `expert-engineer` → `openai/gpt-6.1-sol#max`, toujours read-only.
- `emergency-engineer` et `emergency-thinker` → `openai/gpt-6-astra#max`.
- `build` (lead par défaut) → sélection habituelle du modèle en session ;
  aucun nouveau modèle imposé. L'ancien profil Luna orchestrator est retiré.
- Autres agents inchangés : `architecte`, `security-reviewer`, `adr-*` →
  `openai/gpt-6.1-sol#xhigh` ; `correctness-reviewer`, `rust-perf-reviewer` →
  `openai/gpt-6-astra#high` ; lecture/mécanique et debuggers infra → GLM Flash.

Le mapping initial Luna pour emergency-engineer est historique, pas une
contrainte technique. Aucun variant `ultra` n'est utilisé pour Astra : le
variant vérifié est `max`. Ce mapping ne prétend pas mesurer coût ou performance.

## Modes et permissions V2

- Tous les agents projet sont `mode: subagent`, sauf `implementer`,
  `hard-implementer` et `emergency-thinker` (`mode: all`). Les deux premiers
  permettent un handoff séquentiel sans recréer la session ; modèles et
  permissions restent inchangés. Mode all ne confère pas de leadership :
  seul le lead Ensemble crée des membres, les membres restent des feuilles.
- Champ `readonly`: remplacé par des règles `permissions` en liste
  ordonnée (`action` / `resource` / `effect`), dernier match gagne :
  - reviewers → `edit .cursor/review-briefings/**: allow`, `edit *: deny`
  - `expert-engineer`, `infra-verifier` → `edit *: deny` total
  - `architecte` + `adr-*` → `allow` sur leur plage ADR uniquement, `deny` ailleurs
  - `k8s-operator` → gate shell sur `kubectl apply/delete/rollout/scale`,
    `kind delete*`, `helm *`
  - `emergency-thinker` → `action: "*"`, `resource: "*"`, `effect: allow`
    (tous outils). Ce wildcard n'est ni consentement aux destructions, ni
    dérogation aux instructions prioritaires, à infra-safety ou au périmètre.
- Aucun réglage de profondeur native n'est modifié. Les droits de lead/member
  viennent d'Ensemble, pas du nom de profil ni de ses permissions. Handoff garde
  identité, historique, tâches, répertoire et modèle sélectionné ; choisir le
  modèle séparément si la politique du profil cible le nécessite.

### Reprise d'une tâche bloquée

- `emergency-engineer` est un worker Astra ciblé, **leaf**, sans transfert de
  coordination. `emergency-thinker` est l'exception : coordinateur temporaire,
  délégateur et investigateur/fixer direct dans le contrat fermé, pas worker quotidien.
- Après **2 tentatives techniques substantielles infructueuses sans progrès
  vérifiable sur le même blocage**, le lead/parent doit transférer au thinker.
  Une troisième est exceptionnellement permise pour une hypothèse différente,
  étayée et bornée ; **3 maximum**, puis transfert. Pas besoin de traverser
  toute la chaîne expert/emergency-engineer. Nouveau worker ou reformulation
  ne remet pas le compteur à zéro.
- Progrès = reproduction vérifiée, frontière de panne localisée, hypothèse
  causale confirmée ou correctif validé, pas prose. Attentes de build/outils,
  autorisations utilisateur et infra indisponible ne sont pas des tentatives ;
  répéter un build placebo n'est pas une nouvelle tentative ni un progrès.
- Le paquet contient objectif, contraintes/invariants/interdits, diff et base
  git, briefings, commandes exactes de reproduction/échec, validation, registre
  des hypothèses/actions/observations/alternatives éliminées/progrès et blocages.
- Sous Ensemble, le lead peut changer de profil par handoff en conservant son
  leadership et son historique ; un membre demande ce transfert au lead et ne
  s'autopromeut pas. Pas de second contrôle actif ni de validation redondante.
  Hors Ensemble, le root relaie les WORKER_REQUEST nommés, dans les limites
  natives : pas de retry imbriqué, de boucle self/thinker ni de bypass de profondeur.
- Contradictions structurelles → `architecte` et décision humaine si nécessaire,
  jamais changement silencieux du contrat. Les autres workers restent leaves.
- Cette règle est une **politique de prompt**, pas un compteur déterministe,
  un déclencheur codé ou une promotion primaire automatique au runtime.
- L'utilisateur peut sélectionner manuellement `emergency-thinker` comme
  primaire : cela autorise la reprise. **Sélectionner l'agent ne change pas le
  modèle stocké dans la session** : choisir aussi `openai/gpt-6-astra#max`.

## Vérifier le routage

- Tester dans une **nouvelle session** après chargement de la configuration :
  les changements de prompt ne transforment pas rétroactivement l'agent
  primaire d'une session déjà ouverte.
- Statique : valider JSONC et frontmatter, comparer modèles/permissions aux
  définitions antérieures, vérifier le périmètre du diff et l'absence de sources
  de skills/blocs dupliqués. Un second setup ne doit rien ajouter.
- Runtime, uniquement sur autorisation explicite : vérifier une nouvelle session
  build, un fork depuis une source de la même équipe et un handoff entre les deux
  implementers. Vérifier rôle/répertoire/modèle et le ledger après les opérations.
  Un échec d'héritage doit être explicite, sans repli fresh ou shared-directory.
- Ces vérifications runtime ne sont pas réalisées par la validation statique.

## MCP

- `rust-code-mcp` : backend natif partagé via 1MCP port 3062 ; exposé dans les
  configurations projet OpenCode, Cursor et Codex. Recherche Rust et graphe HIR.
- `grepai` : désactivé uniquement pour AIForAll (2026-10-24), anciens index et
  autres projets conservés. Aucun fallback automatique.
- `serena` : connexion projet partagée des trois clients, édition symbolique
  prioritaire directement inscrite dans leurs profils écrivains.
- `qmd` : conservé, configuration OpenCode globale et serveur projet ; notes et
  connaissances restent dans QMD/Obsidian, sans migration vers l'index Rust.
- `kubernetes-mcp-server` et `ig-mcp-server` : inchangés, lecture seule et périmètre
  local prévu dans AGENTS.md.
- `context-mode` : MCP global dans OpenCode V2, utilisé via Code Mode pour les
  sorties volumineuses. Ne pas réinstaller le plugin TS V1 incompatible ni
  prétendre que Code Mode seul remplace ce MCP. Autres clients inchangés.

Détails et rollback : [rust-code-mcp](rust-code-mcp.md). Les configurations MCP
et profils sont rechargés par OpenCode sans redémarrage global du service.

## Accès auth OpenAI

Les agents référencent 3 modèles du provider `openai`
(`gpt-6.1-sol`, `gpt-6-luna`, `gpt-6-astra`). Ces slugs nécessitent une
auth OpenAI active (`opencode auth login` — choisir OpenAI, ou
`OPENAI_API_KEY` dans l'environnement). Sans auth, la session échouera à
charger ces agents ; basculer temporairement sur un modèle GLM.

## Maintenance

- Chaque plateforme a sa source de vérité : `.cursor/agents/` pour Cursor,
  `.codex/agents/` pour Codex et `.opencode/agents/` pour la politique OpenCode
  dédiée, issue du portage du 2026-10-02 puis évoluée séparément.
- Les évolutions sont par plateforme. Une synchronisation manuelle vers
  Cursor/Codex nécessite une demande distincte ; cette évolution OpenCode
  ne modifie pas les autres plateformes.
- Les règles Cursor toujours-appliquées (`.cursor/rules/*.mdc`) n'ont pas
  d'équivalent automatique OpenCode ; leur contenu vital est repris dans
  `AGENTS.md` (toujours lu par OpenCode) et dans les prompts système
  agents ci-dessus.
