# ADR-0503 : Le runtime HTTP possède et joint ses descendants avant de déclarer son arrêt terminé

- Statut : Proposed
- Réalité : Implemented (API additive et adoption Telegraph/monolithe ; réconciliation 2026-10-28)
- Date : 2026-10-26
- Décideurs : à renseigner à la ratification ; périmètre correctif autorisé par l'utilisateur, rapport 53
- Jalon concerné : Telegraph outbox — AC7 / C-1, hors jalons P-Apparatus
- SuperSède : aucune
- SuperSédée par : —

L'autorisation de corriger ne vaut pas Accept de cette ADR. La correction est
désormais livrée avec les preuves bornées ci-dessous ;0313 et0503 restent
Proposed,0301 reste Partial. Fusion Git ne signifie pas validation globale.

## Contexte

Sur la baseline historique AIForAll `aa400f38608e92a34c463f1f89b02168f8364440`, SDK `129dcc1575e7dd59b4c7935de0b3794dbb7e7d07`, les callers Telegraph et monolithe abandonnent le futur serveur lors du stop, puis joignent seulement son parent. Axum 0.8.9 crée des tâches de connexion ; leur shutdown gracieux peut conserver un handler async en cours.

Le finding indépendant T4 C-1 HIGH contredit le contrat Telegraph §5.1/§5.2 : absence de tâche orpheline locale et budget global six secondes, stop et HTTP inclus. Un handler async local n'est pas un effet réseau distant.

**E1 historique rouge exécuté et intégré** : baseline aa400f3 / SDK129dcc1, 0 PASS / 1 FAILED / 0 ignored / 4 filtered, exit101. Cleanup Ok et parent terminé mais RAII handler0 au retour, puis mutation tardive1 après libération et RAII détruit. Nettoyage avant assertion rouge. Source intégrée dans `b291eee3b7537bfa02cebb5df844d5b4ca2019bf`, blob `221abe3fc7a1b751c8a1d574b422f767ee68abca`. C-1 restait ouvert à cette étape ; la livraison ultérieure figure en fin d'ADR, sans effacer ce rouge.

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
- E1 et le contrat/API ont été intégrés avant le correctif. Le témoin raccordé au propriétaire livré est vert ; son rouge sur la baseline historique est conservé.

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
- Aucune Accept de0503/0313, promotion globale de0301 ou validation globale ; le correctif ciblé C-1 est livré sans élargir les garanties aux callers legacy.
- Registre, notifications et exécuteur concrets : choix privés bornés par le contrat, pas une nouvelle API générique.

## Références et réalité

- [Contrat HTTP](../contracts/http-runtime-owned-shutdown.md) ; [contrat Telegraph](../contracts/telegraph-notification-outbox.md), §5 et AC7.
- [0404 — dual runtime](0404-runtime-microservices-et-monolithe.md) ; [0502 — SDK feature-gated](0502-rustycog-framework-feature-gated.md), photographies non réécrites.
- Dépôt principal : rapport 53 `.opencode/missions/team_mvo1md2x_0000_zlfadf/reports/53-autorisation-http-sdk-et-temoin.md`, rapport 55 `55-http-c1-e1-rouge-recoupe.md`, rapport 57 d'intégration du témoin ; QA `.cursor/qa-plans/20261026-http-ac7-c1.md` ; revue `.cursor/review-briefings/20261026T1704Z-correctness-telegraph-outbox-t4-aa400f3.md`.
- Sources : `services/Telegraph/setup/src/{app,runtime}.rs`, `services/Telegraph/http/src/lib.rs:62–67`, `runtime/monolith/src/runtime.rs:95–127`, `services/Telegraph/tests/fixtures/outbox_runtime.rs:47–51`, `rustycog/rustycog-http/src/builder.rs:236–299`.
- SDK unifié : `rustycog/Cargo.toml`, pas de manifeste HTTP séparé. Dépendances examinées : Axum 0.8.9 `src/serve/mod.rs:267–303,389–415` ; Hyper 1.10.1 `src/proto/h2/server.rs:310–318`.
- **Réalité Implemented dans le périmètre décrit** : propriétaire/API SDK et adoption Telegraph standalone/listener commun du monolithe livrés ; ni les autres standalones ni les callers legacy n'acquièrent rétroactivement cette garantie.

### Livraison et preuves réconciliées au 2026-10-28

- SDK exact d60a88b intégré dans RustyCog main par9e9ef27 ; application39024b2
  intégrée à AIForAll master par66c16f1, gitlink d60a88b. Le
  [rapport150](../../.opencode/missions/team_mvo1md2x_0000_zlfadf/reports/150-mission-et-sdk-integres-aux-branches-principales.md) vérifie ascendances et identité du code livré, pas une nouvelle exécution complète.
- [Rapport77](../../.opencode/missions/team_mvo1md2x_0000_zlfadf/reports/77-correctif-tests-verts-et-formatage-c1.md) : diagnostics persistants et destruction indépendante du driver,10 tests propriétaire PASS sur le SHA b7 alors testé. Preuves conservées à travers le formatage, sans inventer une nouvelle campagne sur d60.
- [Rapport79](../../.opencode/missions/team_mvo1md2x_0000_zlfadf/reports/79-trois-callers-ai-verts-sur-sdk-final.md) : E1, helper standalone et vrai chemin monolithe, chacun1 PASS/0 ignoré sous le SDK9e alors testé ; destruction avant retour et absence de mutation tardive sur ces témoins, pas trois suites intégrales.
- [Rapport95](../../.opencode/missions/team_mvo1md2x_0000_zlfadf/reports/95-review-compat-pass-it-restantes.md) : revue ciblée port0 et Extended CONNECT close sans nouveau finding sur SDKd60 ; les IT encore attendues à cette étape sont couvertes par les preuves finales du [bilan141](../../.opencode/missions/team_mvo1md2x_0000_zlfadf/reports/141-livraison-terminee-avec-reserves-explicites.md), notamment la [verticale139](../../.opencode/missions/team_mvo1md2x_0000_zlfadf/reports/139-validation-verticale-reussie.md).

Borne caller5s de grâce/6s de stop et jointure conservée. Drop/abort n'est pas
une attestation d'async join ; toutes les erreurs de startup, panics et courses
ne sont pas prétendues forcées en runtime. Pas de validation de la plateforme
complète, de protocole arbitraire ou d'effets distants. Dettes Clippy/Sonar/CI
conservées selon le bilan141. Aucun nouveau Cargo/runtime exécuté pour cette
réconciliation documentaire ; ratification humaine du statut toujours distincte.
