# Campagne de valeur rust-code-mcp — résultats du 2026-10-24

## Conclusion
**7 scénarios PASS, 1 PARTIAL**, sur **17 appels métier / 14 outils distincts**.
La valeur utile est démontrée pour localiser du code, préparer un changement
transversal et contrôler une surface d'API. La codemap à prompt libre demande
encore une sélection humaine des seeds ; « le serveur répond » n'est pas son verdict.

[Plan écrit avant exécution](20261024-rust-code-mcp-value-plan.md).
Run réel : **32,26 s**, profil local Qwen3-Embedding-8B, une seule initialisation,
aucun warning/erreur natif et aucun appel métier en erreur. Ni Cargo, ni rebuild,
ni reindex, ni modification de source ou de politique du projet.

## Ce que les outils ont réellement apporté

### V1 — Recherche d'un comportement inconnu : PASS, classement perfectible
Question sans nom de symbole : service qui doit devenir non prêt lorsqu'un
backend de messages configuré se transforme silencieusement en adaptateur inerte.
- Hybride : **signal_queue_status en premier**, ComponentStatus en deuxième.
  Le top 5 comporte aussi deux constructeurs d'erreurs génériques moins utiles.
- Dense seul : trois health_check génériques avant signal_queue_status, **quatrième**.
- Même classement hybride à chaud. **30,218 s à froid**, **154 ms à chaud** ;
  dense chaud 126 ms. Le chargement initial 8B domine, pas chaque requête.
Valeur : un point d'entrée réel, pas une réponse métier complète sans lecture.
Choix d'usage : hybride d'abord, dense comme recherche complémentaire.

### V2 — Voisins de refactor : PASS, candidats à relire
Depuis `telegraph_infra::event::consumer::EventConsumer::transport_status` :
- voisin1 : méthode homologue Manifesto, cosinus **0,9391** ;
- voisin2 : méthode homologue Lazaret attendue avant run, **0,9361** ;
- seed Telegraph absent des résultats, donc pas d'auto-match flatteur.
Les deux méthodes ont même rustdoc, signature et corps, vérifiés avec Serena.
Le troisième candidat est un getter `status() -> &str` de fixture DB : forme
proche, pas un doublon métier confirmé. Durée 157 ms. Les cosinus ne sont pas
des probabilités ; la factorisation reste une décision de conception.

### V3 — Analyse d'impact transverse : PASS
Les **3/3 sites connus** sont retrouvés, octets start/end exactement conformes :
Telegraph consumer.rs:40, Manifesto consumer.rs:56, Lazaret consumer.rs:37.
Le filtre Telegraph ne retourne que son constructeur. Les sorties de la factory
  retrouvent classify_consumer et signal_queue_status. Appels 10–20 ms.
**Attention : calls_from renvoie 5 références, pas 5 invocations** : 3 appels
de fonction et 2 références de type/constructeur. C'est conforme à son contrat,
mais il ne faut pas transformer ce nombre en métrique de vrais appels.

### V4 — Signatures et couverture paginée : PASS
Exactement les deux factories async prenant `&QueueConfig`, une par page,
total=2 annoncé sur les deux pages. La signature consumer confirme le paramètre
emprunté et le résultat `Result<SignaledConsumer, ServiceError>`.
HIR l'affiche sous `impl Future<Output = Result<...>>`, différence de présentation
à connaître pour raisonner sur une API async. Appels11–20 ms.
Valeur : préparer une migration de signature sans perdre la deuxième page.

### V5 — Surface publique : PASS
Les **15/15 réexports explicites** de readiness/lib.rs sont présents. La chaîne
de classify_consumer distingue la déclaration canonique sous classify et le
chemin public réexporté à la racine. Appels5–17 ms.
Valeur : savoir quels imports publics préserver lors d'un déplacement interne.

### V6 — Garde-fou architectural : PASS
Trois règles synthétiques : **une seule violation, celle attendue**. L'exception
readiness et le lien absent readiness→telegraph_infra ne déclenchent rien.
Le vrai lien readiness→rustycog_framework porte QueueConfig comme symbole témoin,
18 symboles distincts et64 références. Durée133 ms.
**Ce n'est pas une violation d'ADR du projet** : l'interdiction était artificielle.
Valeur : le moteur discrimine un lien réel, une exception et une absence.

### V7 — Audit de derives : PASS comme inventaire, pas comme autofix
Les deux témoins sans Debug (SignaledConsumer/SignaledPublisher) sont signalés,
ainsi que ReadinessProbe, confirmé dans la source. ComponentStatus déjà conforme
n'est pas signalé ; ses attributs Debug/Clone/PartialEq/Eq sont correctement lus.
Les trois findings ont des fichiers/spans vérifiés. Appels5–20 ms.
Il reste à déterminer si Debug est demandé et réalisable pour leurs champs :
l'outil ne prouve ni la nécessité d'une correction ni sa compilation.

### V8 — Codemap orientée tâche : PARTIAL
- **Seed explicite : utile**, 20 nœuds/24 arêtes en215 ms. La factory, sa chaîne
  classify/signal et les trois constructeurs consommateurs sont visibles.
  Tous les fichiers/spans des deux cartes sont conformes aux sources.
- **Prompt seul : moins fiable**, 20 nœuds/21 arêtes en385 ms, avec8 seeds.
  Signal/classification readiness sont présents, mais **9 nœuds concernent la
  signature IAM**, hors de la question ; la factory consumer n'est pas retenue.
- Diagnostic : **52 hits perdus à line-resolve**, zéro à path-norm/kind-filter.
  La cause de ces pertes n'est pas établie par cette campagne. Ne pas en déduire
  que le graphe est absent ou qu'il faut reconstruire tout l'index.
Valeur partielle : utile à partir d'un symbole choisi, pas une carte autonome
de confiance à partir de toute phrase. Les cartes plafonnées ne sont pas exhaustives.

## Mode d'emploi recommandé après cette preuve
1. **search hybride** pour trouver un point d'entrée ; lire les résultats utiles.
2. **who_calls / calls_from / signatures / reexports** pour préparer l'impact.
3. **codemap à seed explicite** pour visualiser le voisinage utile ; vérifier les
   diagnostics avant d'utiliser une carte à prompt libre.
4. **similar_to_item** pour proposer des voisins, puis comparer leurs corps/types.
5. **Serena** pour naviguer/éditer les symboles, et tests proportionnés après changement.

Suite technique utile, non exécutée ici : approfondir le raccordement hit→nœud
`line-resolve` et la pertinence des seeds de codemap. Pas d'initialisation globale
manquante démontrée, ni de correctif applicatif à tirer automatiquement des audits.

## Preuves, limites et cleanup
Sous `C:/Users/djden/AppData/Local/rust-code-mcp/` :
- `value-campaign-preflight.json` et `value-campaign-source-oracles.json` : source
  indépendante des réponses (oracles fixés dans le plan, consolidation ensuite).
- `value-campaign-20261024.json` : paramètres, réponses intégrales, durées,
  empreinte du plan, contrôles de fin. `evaluate-value-campaign-20261024.mjs` et
  `value-campaign-20261024-evaluation.json` : comparaisons et verdicts explicités.
- Source989 entrées inchangées, plan inchangé, aucun index/snapshot effacé ou
  reconstruit, aucune ressource Docker/Kind/WSL créée/démarrée.
- Caches créés pour la campagne libérés, état runtime final0 ; VRAM globale
  libre **21819 MiB** au dernier relevé. Les trois relevés ne mesurent pas un pic.

Campagne bornée autour de readiness et ses consommateurs, pas benchmark exhaustif
des55 outils ou de tous les services. Validation par le même agent, pas review
indépendante. Les7 PASS ne sont pas un taux statistique de fiabilité générale.
Les réponses brutes restent privées ; aucun secret ni dump de configuration
n'est reproduit dans ce rapport. Les limites de valeur observées ne sont pas masquées
par le succès technique des17 appels.
