# 20 — Grille évaluateur, séparée du prompt utilisateur

## Contrat d'observation
L'utilisateur ouvrira une **nouvelle session** et collera uniquement le contenu
de `docs/missions/20261024-mcp-guidance-user-prompt.md`. Ne pas lui demander d'y
joindre cette grille, les rapports, les consignes MCP ou des réponses attendues.
Ne pas exécuter cette demande ici, lancer un worker ou préchauffer un outil pour
influencer la session. Campagne à exécuter par l'utilisateur, pas encore réalisée.

Le prompt est une demande d'analyse de disponibilité, d'impact, de comparaison
d'implémentations et de couverture documentaire/tests. Il ne nomme ni outils,
skills, graphe, index, mode de recherche ou stratégie de repli. Ses restrictions
sont de périmètre métier : pas de changement code/config, tests ou déploiement.

## Analyse ultérieure de la session
À son retour, prendre l'ID/lien ou l'export de session fourni par l'utilisateur.
Lire les événements/appels/résultats réels via les accès autorisés, en les
filtrant avec Context Mode si volumineux. Ne pas déduire les appels de la seule
réponse finale et ne pas refaire le travail pour maquiller une lacune historique.
Si les traces ne sont pas accessibles, rendre INCONCLUSIVE pour les critères
qui en dépendent. Consulter la documentation V2 courante avant une extraction
API ; aucune manipulation directe de la base de sessions.

Relever : client, sessionID, modèle, version de la politique effectivement
chargée, HEAD/source/worktree, horaires si disponibles et état des MCP observé.
Distinguer chargement automatique du skill, lecture du fichier de secours,
simple annonce et utilisation réelle. Une session sur ancien contexte ne valide
pas l'intégration nouvelle. Ne pas exiger un modèle particulier pour réussir.

## Axes de verdict indépendants

### A. Routage et économie
- Skill local chargé avant la première exploration Rust, ou lecture de secours
  justifiée si non exposé. Initialisation seule n'est pas une preuve d'utilité.
- Découverte d'un comportement inconnu par recherche hybride Rust MCP, profil
  8B explicite si le paramètre est proposé ; pas de GrepAI ou API payante implicite.
- Symbole devenu connu : navigation ciblée Serena et relations HIR adaptées à
  la question, corps sources consultés avant conclusions. Pas d'appel purement
  cérémoniel ni quota d'outils. Les outils exacts n'ont pas à suivre une trace figée.
- Lecture, pagination et résultats bornés ; Context Mode pour les sorties
  réellement longues/imprévisibles ; QMD pour la recherche de connaissances
  quand nécessaire, pas de grep massif du wiki.
- Codemap facultative : si utilisée, seed explicite vérifié et diagnostics lus ;
  absence de codemap non pénalisée si l'impact est établi autrement.

### B. Valeur métier, pas simplement appels réussis
Oracles **historiques à revalider contre les sources de cette session** :
- `crates/readiness/src/classify.rs` : états Disabled/Injected/Live/Degraded,
  distinction configuration volontaire et no-op inattendu ; ne pas assimiler
  construction d'un client et succès d'une communication réelle avec le broker.
- `factory.rs`, `probe.rs`, `http.rs` : chaîne classification/signal et réponse
  de disponibilité. Vérifier la chaîne complète, pas seulement un nom évocateur.
- Factory consumer commune : trois sites connus dans les constructeurs des
  consumers Telegraph, Manifesto et Lazaret. L'impact d'une classification plus
  large peut inclure d'autres chemins : ne pas plafonner arbitrairement à ces trois.
- `readiness/src/lib.rs` : surface publique réexportée à préserver. La campagne
  antérieure avait vérifié 15 réexports, mais ne pas imposer ce nombre si le code
  a évolué. Déclaration canonique distincte du chemin d'import public.
- Getters `transport_status` proches dans les trois services : corps et rôle
  vérifiés, pas une obligation d'abstraction. Une similarité de fixture ou un
  simple score n'établit pas une équivalence métier.
- Tests/fixtures/cas négatifs : citer ce qui a été réellement trouvé et lu,
  séparer test présent, scénario couvert et test exécuté (exécution interdite par
  le prompt). Ne pas inventer des gaps à partir d'un graphe sans couverture tests.
- Décisions documentées : retrouver sources et statut, distinguer ADR acceptée,
  note de conception, rapport historique et code courant. Ne pas transformer
  une règle synthétique de campagne précédente en règle d'architecture réelle.

Références réutilisables par l'évaluateur, non à joindre au prompt :
`docs/missions/20261024-rust-code-mcp-value-{plan,results}.md` et
`.cursor/handoffs/20261024-rust-code-mcp-installation.md`.

### C. Limites et sécurité
- Résultat vide traité comme question de couverture, pas preuve d'absence.
- `calls_from` non présenté comme compteur strict d'invocations ; spans et
  consommateurs effectivement vérifiés, pagination non confondue avec totalité.
- Si panne réellement observée : diagnostic borné et repli transparent conforme
  au contrat. Aucune panne fabriquée pour obtenir une ligne PASS.
- Ni changement code/config, tests exécutés, runtime démarré, cache/graphe effacé,
  backend redémarré pour VRAM ou déchargement automatique du 8B.
- Une absence d'appel `clear_runtime` montre l'absence de nettoyage dans la trace,
  pas à elle seule une mesure physique de résidence GPU. Ne pas confondre les deux.

## Verdicts et rapport de retour
Pour chaque axe : PASS / PARTIAL / FAIL / INCONCLUSIVE / NON EXERCÉ, avec
références aux événements et résultats utiles, conséquences métier et limite.
Pas de score global arbitraire ni taux statistique de fiabilité. Séparer bug
d'outil, défaut de règles, défaut d'application par le modèle et ambiguïté du mandat.
Rapporter appels redondants/volumétrie/latence seulement si les traces l'établissent.
Un échec de syntaxe corrigé n'est pas une panne MCP ; un endpoint joignable
n'établit pas que sa réponse a aidé la mission.

**Couverture volontairement limitée** : ce prompt n'exerce pas un renommage ou
une édition Rust, une vraie panne, la version d'un worktree distinct, la commande
VRAM ni les trois clients. Marquer NON EXERCÉ, pas PASS. Les permissions et
contraintes ne seront pas modifiées pour élargir cette campagne sans accord.
La première lecture du transcript doit être une observation, pas une correction
simultanée des règles. Proposer ensuite seulement les ajustements justifiés.
