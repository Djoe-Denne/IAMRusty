# ADR-0503 : Le runtime HTTP possède et joint ses descendants avant de déclarer son arrêt terminé

- Statut : Proposed
- Réalité : Unimplemented
- Date : 2026-10-26
- Décideurs : à renseigner à la ratification ; périmètre correctif autorisé par l'utilisateur, rapport 53
- Jalon concerné : Telegraph outbox — AC7 / C-1, hors jalons P-Apparatus
- SuperSède : aucune
- SuperSédée par : —

L'autorisation de corriger ne vaut ni Accept de cette ADR ni preuve de correction. ADR 0313 reste Proposed et ADR 0301 reste Partial.

## Contexte

Sur la baseline historique AIForAll `aa400f38608e92a34c463f1f89b02168f8364440`, SDK `129dcc1575e7dd59b4c7935de0b3794dbb7e7d07`, les callers Telegraph et monolithe abandonnent le futur serveur lors du stop, puis joignent seulement son parent. Axum 0.8.9 crée des tâches de connexion ; leur shutdown gracieux peut conserver un handler async en cours.

Le finding indépendant T4 C-1 HIGH contredit le contrat Telegraph §5.1/§5.2 : absence de tâche orpheline locale et budget global six secondes, stop et HTTP inclus. Un handler async local n'est pas un effet réseau distant.

**E1 est désormais rouge exécuté et intégré**, pas attendu : baseline aa400f3 / SDK129dcc1, 0 PASS / 1 FAILED / 0 ignored / 4 filtered, exit 101. Le cleanup retourne Ok avec parent terminé mais RAII du handler à 0 au retour ; après libération, mutation tardive à 1 et RAII détruit. Le nettoyage précède l'assertion rouge. Source intégrée dans `b291eee3b7537bfa02cebb5df844d5b4ca2019bf`, blob `221abe3fc7a1b751c8a1d574b422f767ee68abca`. Cette intégration du témoin n'est pas un correctif SDK : C-1 reste ouvert.

Le rapport 53 autorise une extension HTTP SDK additive et bornée, sur la pile existante, sans nouveau backend/harness ni réduction d'AC7.

## Décision proposée

1. Ajouter dans `rustycog::http` un propriétaire conservé par le caller. Lui seul peut attester la terminaison après observation/jointure des descendants : connexions, requêtes, bodies/streams, tâches d'exécuteur HTTP/2 et handshakes pertinents aux protocoles existants.
2. Séparer observation de sortie du futur serveur, demande d'arrêt, jointure complète et Drop. La disparition du coordinateur ne détruit pas les moyens du propriétaire d'annuler et d'observer ses descendants.
3. Utiliser les échéances absolues communes calculées par le caller avant tout stop : grâce à t0 + 5 s, deadline à t0 + 6 s. Annuler les descendants restants puis observer leur terminaison ; ni graceful illimité ni succès par abandon du parent.
4. Drop ferme l'admission et ordonne l'annulation, sans prétendre effectuer une jointure async. Une cancellation du parent ne produit aucun rapport de cleanup réussi.
5. Préserver l'API legacy `serve_router`, ses consommateurs, les préfixes, TLS/mTLS, dual-bind et protocoles supportés. Adoption initiale limitée à Telegraph standalone, au listener commun du monolithe et à leurs fixtures existantes.
6. Conserver la pile Axum/Hyper/rustls existante ; pas de nouveau backend, fork Axum ou runtime HTTP parallèle dans les services. L'API minimale et ses responsabilités sont fixées dans le [contrat HTTP](../contracts/http-runtime-owned-shutdown.md).

## Conséquences

- Le caller conserve une ressource HTTP distincte du vecteur de handles métier. Une fin parent ou un compteur de connexions nul n'est pas une preuve exhaustive.
- L'ownership couvre les spawns indirects et les bodies, pas seulement l'accept loop et le handler.
- Les erreurs/panics, résultats déjà terminés et dépassements sont observés et contextualisés, sans effacer l'erreur primaire.
- Le coût assumé est une API additive de lifecycle et un groupe HTTP dans le SDK, pas une nouvelle infrastructure.
- AC1–AC7, exclusions exactes §5.2, preuves Sentinel et calibration INBOUND §6.2 restent inchangés. Aucun nouveau SLA de production.
- E1 est déjà intégré ; le contrat/API doit encore être intégré avant implémentation. Son rouge historique est conservé.

## Alternatives non retenues

- Abort/jointure du seul parent : reproduit C-1.
- Graceful seul : un handler/body async bloqué peut attendre indéfiniment.
- Timeout/middleware du handler seulement : ne démontre pas l'ownership des connexions, bodies et tâches d'exécuteur.
- Simple handle de shutdown : son signal ne prouve pas une jointure complète.
- Serveur recréé dans setup et monolithe : duplication du runtime HTTP et de ses règles TLS.
- Réduction d'AC7 ou assimilation du handler local à un effet distant : contraire au contrat conservé.

## Non décidé ici

- Aucun changement de backend, protocole, threading global, déduplication IAM ou exactly-once.
- Aucune garantie supplémentaire sur sync non coopératif, spawn_blocking, kill OS ou annulation d'effets distants.
- Aucune Accept de 0503/0313, promotion de 0301, clôture C-1 ou validation globale.
- Registre, notifications et exécuteur concrets : choix privés bornés par le contrat, pas une nouvelle API générique.

## Références et réalité

- [Contrat HTTP](../contracts/http-runtime-owned-shutdown.md) ; [contrat Telegraph](../contracts/telegraph-notification-outbox.md), §5 et AC7.
- [0404 — dual runtime](0404-runtime-microservices-et-monolithe.md) ; [0502 — SDK feature-gated](0502-rustycog-framework-feature-gated.md), photographies non réécrites.
- Dépôt principal : rapport 53 `.opencode/missions/team_mvo1md2x_0000_zlfadf/reports/53-autorisation-http-sdk-et-temoin.md`, rapport 55 `55-http-c1-e1-rouge-recoupe.md`, rapport 57 d'intégration du témoin ; QA `.cursor/qa-plans/20261026-http-ac7-c1.md` ; revue `.cursor/review-briefings/20261026T1704Z-correctness-telegraph-outbox-t4-aa400f3.md`.
- Sources : `services/Telegraph/setup/src/{app,runtime}.rs`, `services/Telegraph/http/src/lib.rs:62–67`, `runtime/monolith/src/runtime.rs:95–127`, `services/Telegraph/tests/fixtures/outbox_runtime.rs:47–51`, `rustycog/rustycog-http/src/builder.rs:236–299`.
- SDK unifié : `rustycog/Cargo.toml`, pas de manifeste HTTP séparé. Dépendances examinées : Axum 0.8.9 `src/serve/mod.rs:267–303,389–415` ; Hyper 1.10.1 `src/proto/h2/server.rs:310–318`.
- **Réalité Unimplemented** : E1 reproduit C-1, mais propriétaire/API et correctif non livrés. Aucun Cargo/runtime exécuté par l'architecte.
