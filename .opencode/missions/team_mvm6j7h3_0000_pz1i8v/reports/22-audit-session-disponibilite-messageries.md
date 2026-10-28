# 22 — Audit de la session « Disponibilité des messageries et contrôles de santé »

**Complément ultérieur sur les latences Serena :**
[rapport24](24-serena-latence-diagnostic.md). Les19,569s client de l'appel
publisher comprennent15,273s d'attente en file et4,258s d'exécution ; ne pas
les interpréter comme20s de calcul de références ou une panne de cache.

## 1. Résultat et périmètre

**Le guidage a été appliqué et a produit une analyse métier utile ; le parcours
reste perfectible en économie de contexte et en localisation des chemins.**

Observation terminée, sans rejouer la mission ni changer les règles. Cet audit
fait suite à l'[application 21](21-guidage-mcp-application-resultats.md) et utilise
la [grille préétablie 20](20-campagne-session-grille-evaluateur.md). Seuls le
présent rapport et le ledger d'audit sont créés. Aucun correctif MCP, source,
profil, configuration ou runtime modifié ; aucun agent supplémentaire.

## 2. Session et méthode de collecte

- ID : **`ses_e91ded28fffdwym6L16xLXxQFD`**.
- Titre exact communiqué par l'utilisateur, un seul résultat retrouvé.
- Client observé : OpenCode V2, agent `build`, modèle
  `openai/gpt-6.1-sol`, variante `medium`.
- Répertoire : `C:/Users/djden/source/repos/AIForAll`.
- HEAD relevé dans la session : `93e376c5530a439aaafe61a64d3e881865c1acf1`.
  Le modèle a identifié un arbre déjà modifié et n'a pas attribué ces travaux à
  sa propre analyse. Le HEAD seul ne décrit pas ces modifications non commitées.
- **39 messages** : 1 utilisateur, 37 assistant, 1 idle. Page suivante consultée :
  0 message, donc pas de fin de transcript supposée à partir de la première page.
- Prompt utilisateur : correspondance textuelle exacte après trim avec le fichier
  livré, SHA256 de référence
  `a06aeb3a44daf4d4a61cb549e0dfa9f9069b86e4e7a964dc8ef457b14c2d9e83`.
- Durée entre premier message et idle : **321,01 s**, soit environ 5 min 21 s.
  Ce n'est pas une latence de recherche MCP ni un benchmark comparable à la
  précédente campagne de 17 appels.

Collecte en lecture seule par CLI/API officielle : `GET /api/session?search=...`,
`GET /api/session/<id>/message?limit=100&order=asc`, puis curseur suivant. La
documentation V2 et son OpenAPI officiel ont été consultés avant les requêtes.
Le nom `opencode` n'était pas au PATH du shell de collecte ; utilisation du CLI
installé et déjà utilisé par le Desktop, sans réparation ou changement de PATH :
`%APPDATA%/ai.opencode.desktop/cli/2.1.4-x64/opencode-cli.exe api get ...`.
Pas d'accès direct à la base de sessions ni de requête locale sans contexte
d'authentification. Sorties analysées dans Context Mode ; seuls les résultats
utiles sont reproduits ici, pas les dumps privés ou champs de raisonnement.

## 3. Ce qui a réellement fonctionné — PASS

### Chargement et priorité locale
Premier appel : `skill(aiforall-mcp-workflow)`, événement
`msg_16e212e23001dCEREaWL7gLkbt`. Le contenu retourné est bien le skill local
créé, pas une simple déclaration d'intention. Puis chargement de `serena` et de
`qmd` avant leur parcours respectif. Aucun routage GrepAI implicite observé.

### Recherche Rust utile, pas simple initialisation
`msg_16e215247001MxVFKlteNCgviP` : recherche hybride descriptive, racine AIForAll,
profil explicite `local-qwen3-8b`. Les trois premiers résultats sont les getters
`transport_status` Lazaret/Telegraph/Manifesto, puis apparaît `ComponentStatus`
dans readiness. Ces points d'entrée sont ensuite lus et exploités.
L'appel complet a pris environ **515 ms** dans la trace ; aucune mesure GPU ne
permet de transformer ce temps chaud en preuve physique de résidence du modèle.

Le serveur a retourné 10 résultats par défaut, tous bornés et courts ; le modèle
n'a pas filtré la réponse aux seuls 3–5 candidats recommandés. Ce n'est ni une
recherche non bornée ni un motif de répéter l'appel pour atteindre un quota.

### Serena et impact HIR substantiels
Initialisation/configuration : `msg_16e217e2d001gYR2dTCa6jCTuw`, projet AIForAll,
LSP prêt. Overviews, références et corps ciblé `build_readiness` ont ensuite
fourni de vraies informations. Les références de `classify_publisher` retrouvent
notamment les wrappers historiques IAM/Hive, au-delà des factories communes :
`msg_16e21f111001b77iTuXZZnsmXo`.

HIR `who_uses`, en résumé, avec totaux/limites :
- Consumer : **3 résultats sur 3**, limit20, dans
  `msg_16e2287c2001C1qiJqh4WL0bF5`.
- Publisher multi-queue : **5 résultats sur 5**, limit25, dans
  `msg_16e237a78001lqwZcMgxfP7Z3i`.

Les sources des consommateurs/setups ont été consultées après ces résultats.
Pas de confusion observée entre le compteur de `calls_from` et des invocations :
cet outil n'a pas été utilisé. Aucune codemap n'était nécessaire pour réussir.

### Connaissances et tests
QMD : une recherche lexicale ciblée sur `queue-readiness-signaling`, puis
lecture du document retrouvé, événements `msg_16e23079d001ZXyskM8AxSS672` et
`msg_16e231c7b0010RxxbhnQ9Y8TKx`. Le modèle distingue ensuite note wiki et ADR,
et compare au code plutôt que de traiter le wiki comme vérité courante.

Tests communs et frontières services ont été lus, pas exécutés. La réponse
finale distingue explicitement tests trouvés, injection qui neutralise une
preuve, cas négatifs non protégés dans le périmètre examiné et absence
d'exécution. Elle ne prétend pas qu'un simple zéro du graphe prouve un gap.

## 4. Valeur métier obtenue — PASS sur les preuves examinées

La réponse finale `msg_16e248061001eyx9o1aV2pe4Ea` ne répète pas seulement le
classement de recherche. Elle étaye notamment :

- `Disabled`, `Injected`, `Live`, `Degraded`, et la remontée d'un no-op inattendu
  vers le rapport puis HTTP 503 ; distinction `/health` et `/ready`.
- **Transport construit ≠ broker utilisable** : portée de GetQueueAttributes,
  metadata Kafka consumer et sonde publisher Kafka plus faible ; pas de promesse
  de publication/consommation métier de bout en bout.
- **`Live` sans transport peut réussir** : comportement actuellement testé,
  donc un durcissement serait un changement de contrat, pas un correctif neutre.
- Statut degraded conservé après construction, sans reconstruction automatique
  du no-op : le retour du broker ne suffit pas à récupérer ce chemin.
- Différences importantes de conservation du consumer/statut entre Manifesto,
  Lazaret et Telegraph ; ne pas fusionner les responsabilités métier parce que
  trois getters ont le même corps.
- Surface d'impact plus large que les trois consumers : publishers IAM/Hive,
  agrégation du monolithe, chemins publics Rust et HTTP, contournement des
  wrappers par `sentinel-sync`.
- Différence entre injection du harness IAM et preuve de classification réelle ;
  tests de liveness/probe non confondus avec HTTP 503 ou tests de routage SQS.
- Documentation historique partiellement dépassée, sans transformer une note
  ou une interdiction synthétique de campagne en décision d'architecture réelle.

Ces conclusions s'appuient sur des corps/sources effectivement retournés dans
la session. L'audit n'est pas une nouvelle exécution de tests ni une revue
exhaustive de chaque affirmation fonctionnelle de cette longue réponse.

## 5. Efficacité du parcours — PARTIAL

### Mesures observées
**77 appels de premier niveau** : 4 skills, 22 `execute`, 15 lectures, 31 grep,
4 glob, 1 shell. Les wrappers `execute` incluent découverte de catalogue et appels
MCP ; ne pas additionner aveuglément leurs sous-appels à ce total.

Les scripts `execute` demandent 3 invocations Rust MCP, 11 Serena, 2 QMD et
10 Context Mode. Ce sont des occurrences d'invocation dans les scripts, pas un
compteur certifiant que chaque sous-résultat a été exploité. Par exemple le
résultat de références `classify_transport` n'est pas visible dans la sortie
composite où un script Context Mode échoue.

Environ **286 469 caractères de textes de résultats d'outils** sont présents,
catalogues et skills inclus, hors encodage JSON. Ce n'est ni un nombre de tokens
ni une mesure des octets uniques de source lus. Les compteurs de session
annoncent 106233 input, 2113536 cache-read et 8457 output ; ce sont des cumuls
multi-tours, pas une fenêtre de contexte unique ni une facture démontrée.

### Quatre chemins supposés, quatre erreurs évitables
Appels grep avec erreur `Search path does not exist` :
- `monolith`, `msg_16e22ae6e001iix4slaC5nQlAl` ;
- `runtime/monolith/tests`, `msg_16e235ddd001lkGSRoOrM0lbpa` ;
- `docs/adr/0404-oodhive-monolith.md`, `msg_16e23e20b001MgL2KNa3OGBb9P` ;
- `services/Telegraph/tests/sqs_event_routing_tests.rs`,
  `msg_16e244ca8001nrauQr5Pt1RZ2j`.

Le modèle retrouve ensuite plusieurs chemins corrects. Ce ne sont pas des
pannes MCP ; une localisation bornée des noms inconnus aurait évité ces essais.

### Context Mode utilisé, mais pas toujours pour synthétiser
Plusieurs scripts lisent des tranches de 3 à 10 fichiers et les impriment
presque telles quelles : résultats composites d'environ 13–22 k caractères,
sans `intent` ni traitement visant une réponse ciblée. Exemples :
`msg_16e229bad001eN68U2vaSsCWD1`, `msg_16e22c634001ZX7Yw1rSIucsSH`,
`msg_16e232e9d001QU1P0GEd64Yq79`.

Les extraits sont bornés et souvent pertinents, donc pas de lecture exhaustive
du dépôt. Mais passer les mêmes tranches brutes par un sandbox ne réalise pas
complètement l'économie annoncée. Il faut filtrer/agréger selon la question ou
demander des corps plus ciblés ; ajouter `intent` seul ne remplace pas ce travail.

### Découverte de catalogue et commande QMD
Deux premiers catalogues généraux renvoient environ 15,8 k puis 9,7 k caractères
sérialisés, avec chevauchement autour de Context Mode. Des demandes plus ciblées
et réutilisées réduiraient cette charge sans supprimer les initialisations utiles.

Le shell `qmd skill show` renvoie une documentation longue hors Context Mode.
Son lancement découle du skill global QMD chargé, donc ce n'est pas une panne
ou un abandon du MCP de recherche. La priorité locale aurait néanmoins pu mieux
encadrer la volumétrie de cette étape de bootstrap.

## 6. Incidents et replis — PARTIAL, pas de panne réelle démontrée

- `msg_16e21b195001dmi4ZfIxd7U213` : `Unterminated string literal` dans le script
  Context Mode. Le modèle corrige l'échappement et réessaie ; bonne distinction
  entre erreur de script et panne du MCP.
- `msg_16e21c92f001WlpeRh14rQm5wE` : binaire `rg` absent du PATH du sandbox.
  Passage aux outils natifs grep, sans installer ni modifier le PATH. Repli
  fonctionnel approprié, mais le premier essai avait masqué l'erreur et rendu
  une sortie vide ; distinguer plus tôt succès vide et échec de commande.
- Serena signale un ensemble de références trop long et fournit une forme
  compacte à `msg_16e237a78001lqwZcMgxfP7Z3i`. Ce n'est pas une panne serveur.

Le modèle n'a pas annoncé clairement ces replis ponctuels, mais n'a pas invoqué
une fausse panne ni lancé une réparation hors mandat. Aucun scénario ne prouve
le comportement en cas d'indisponibilité réelle de Rust MCP, Serena, QMD ou
Context Mode : **NON EXERCÉ**, pas PASS.

## 7. Respect du périmètre — PASS observable

Aucun appel d'édition, test, build, déploiement, création de runtime ou opération
de nettoyage du backend dans la trace examinée. Les snapshots assistant n'indiquent
aucun fichier changé. Le seul shell direct est `qmd skill show` ; les commandes
embarquées pertinentes sont observationnelles, notamment Git et recherche.

Pas de `clear_runtime`, `clear_cache`, restart backend, GrepAI ou fallback
CPU/4B/API payante. **L'absence de déchargement dans la trace n'est pas une
mesure physique de VRAM**. La commande manuelle et la rétention GPU réelle ne
sont pas retestées ici. Les modifications préexistantes ne sont pas imputées
à cette session.

## 8. Limites de la campagne

- Parcours en lecture seule : édition symbolique Rust, vraie panne, worktree
  distinct et intégration des clients Cursor/Codex **NON EXERCÉS**.
- Le modèle a lu par l'outil `read` le ledger historique
  `.cursor/handoffs/20261024-rust-code-mcp-value-campaign.md`, limit85, avant
  la synthèse. Cette lecture est présente dans la page de messages auditée.
  Les résultats antérieurs donnent des pistes et oracles : ce n'est donc pas
  une preuve de généralisation sur un sujet totalement nouveau. Il n'a pas
  reçu la grille 20 dans le prompt, et les sources courantes ont bien été relues.
- Audit par le même agent qui a écrit les règles, pas une review indépendante.
  Les verdicts reposent sur les événements et sorties observés, pas sur une
  promesse de conformité garantie de tous les modèles.

## 9. Recommandation — aucune correction appliquée

Conserver le design actuel : **pas de nouvelle initialisation ou de changement
de modèle démontré nécessaire**. Les améliorations utiles sont procédurales :
1. Localiser les chemins inconnus avant les recherches groupées, ne pas inventer
   des noms plausibles de tests ou d'ADR.
2. Demander à Context Mode une extraction/synthèse ciblée, avec plafond de sortie
   et corps exacts seulement là où ils apportent la preuve.
3. Réutiliser les signatures découvertes et traiter explicitement les erreurs
   d'exécutable au lieu de les assimiler à une recherche vide.

Ces points sont non bloquants pour exploiter les MCP dès maintenant ; aucun
défaut fonctionnel bloquant de l'installation n'a été mis en évidence par cette
session. Une future campagne sur un autre domaine mesurerait mieux la
généralisation ; elle ne doit pas être lancée ou élargie implicitement ici.

**Aucune décision bloquante demandée** : compte rendu d'observation uniquement,
pas d'autorisation supposée pour modifier encore les règles. Les sources de
preuve restent accessibles dans la session native via ses IDs, sans dump brut
privé ajouté au dépôt. Aucun Docker/Kind/WSL/Cargo lancé ou ressource créée.
