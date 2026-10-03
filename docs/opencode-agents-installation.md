# Définitions d'agents et MCP sous OpenCode

Photographie mise à jour le 2026-10-03. Origine : portage des définitions Cursor
(`.cursor/agents/*.md`, `.cursor/mcp.json`) et Codex (`.codex/agents/*.toml`)
vers les formats OpenCode V2 le 2026-10-02, puis politique OpenCode dédiée.
`/nul` et `.tmp*` restent hors périmètre.

## Layout OpenCode de ce dépôt

| Emplacement | Rôle |
|---|---|
| `opencode.jsonc` | Config projet : `default_agent`, MCP `grepai` / `serena` / `kubernetes-mcp-server` / `ig-mcp-server`, timeout `startup: 150000` (proxy Serena lent), `skills: [.agents/skills, .cursor/skills]` |
| `.opencode/agents/*.md` | 22 définitions, harness inclus, format V2 : `description`, `mode`, `model`, `permissions`, corps = prompt système |
| `~/.config/opencode/opencode.json` | Config globale (toutes machines/projets) : MCP `qmd`, note plugins rtk-hook |

Les skills sont découverts automatiquement depuis `.agents/skills/` et
`.cursor/skills/` via la clé `skills` de `opencode.jsonc`.

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
- `orchestrator` → **`openai/gpt-6-luna#xhigh`**, défaut inchangé.
- Autres agents inchangés : `architecte`, `security-reviewer`, `adr-*` →
  `openai/gpt-6.1-sol#xhigh` ; `correctness-reviewer`, `rust-perf-reviewer` →
  `openai/gpt-6-astra#high` ; lecture/mécanique et debuggers infra → GLM Flash.

Le mapping initial Luna pour emergency-engineer est historique, pas une
contrainte technique. Aucun variant `ultra` n'est utilisé pour Astra : le
variant vérifié est `max`. Ce mapping ne prétend pas mesurer coût ou performance.

## Modes et permissions V2

- Tous les agents projet sont `mode: subagent`, sauf `orchestrator` et
  `emergency-thinker` (`mode: all`). `orchestrator` reste `default_agent`. Dans une nouvelle session, il
  est déjà le superviseur : il traite la requête directement, sans déléguer
  à un second `orchestrator`. Le harness ne délègue vers cet agent que si un
  autre agent ordinaire est primaire ; une reprise autorisée par thinker est
  acceptée directement, sans rebond vers orchestrator.
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
- L'`orchestrator` primaire délègue via le tool `subagent`. S'il est lui-même
  un sous-agent et ne peut pas imbriquer de délégation, il retourne un
  `WORKER_REQUEST` complet ; le parent relaie le worker explicitement choisi
  et retourne son résultat au contrôle actif, qui garde la validation finale.
  La profondeur effective reste **1** : permissions complètes ne permettent
  pas une imbrication supplémentaire. Aucun réglage de profondeur n'est modifié.

### Reprise d'une tâche bloquée

- `emergency-engineer` est un worker Astra ciblé, **leaf**, sans transfert de
  coordination. `emergency-thinker` est l'exception : coordinateur temporaire,
  délégateur et investigateur/fixer direct dans le contrat fermé, pas worker quotidien.
- Après **2 tentatives techniques substantielles infructueuses sans progrès
  vérifiable sur le même blocage**, orchestrator doit transférer au thinker.
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
- Thinker devient propriétaire de coordination et validation finale ; le root
  relaie ses résultats et WORKER_REQUEST nommés. L'ancien coordinateur suspend
  analyse technique et validation redondantes jusqu'à fin/réaffectation par
  l'utilisateur. À profondeur 1, le root relaie les demandes : pas de retry
  imbriqué, d'appel récursif orchestrator/self/thinker ou de contournement de profondeur.
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
- Avec `orchestrator` comme agent primaire, vérifier qu'il supervise la
  requête directement, sans sous-appel à `orchestrator`.
- Si un autre agent ordinaire est primaire, vérifier le passage unique à
  `orchestrator`. Si l'imbrication de workers est indisponible, le parent
  relaie le `WORKER_REQUEST` sans changer le choix, puis rapporte le résultat
   pour la validation finale par le contrôle actif.
- Pour une reprise autorisée (ou thinker explicitement choisi comme primaire),
  vérifier l'absence de rebond vers orchestrator et le relais au thinker.
  Ces vérifications runtime sont manuelles ; elles ne sont pas réalisées par
  la seule validation statique des fichiers.

## MCP

| Serveur | OpenCode | Cursor | Codex |
|---|---|---|---|
| `grepai` | ✔ `opencode.jsonc` | ✔ `.cursor/mcp.json` | ✔ |
| `serena` (proxy 1MCP) | ✔ | ✔ | ✔ |
| `kubernetes-mcp-server` | ✔ | ✔ | — |
| `ig-mcp-server` (Inspektor Gadget, read-only) | ✔ | ✔ | — |
| `qmd` | ✔ global `~/.config/opencode/opencode.json` | — | — |
| `context-mode` | **—** (plugin TS des autres plateformes, pas un serveur stdio MCP) | ✔ | ✔ |

`context-mode` s'installe comme **plugin TS** OpenCode
(`~/.config/opencode/plugins/`), pas comme serveur MCP stdio — décision
2026-10-02 : sauté pour ce projet, le pattern « analyser dans le sandbox »
est couvert par Code Mode natif d'OpenCode.

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
