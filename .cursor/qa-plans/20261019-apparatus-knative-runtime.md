# Plan Q — Apparatus runtime HTTP Knative par binding

- Mission : `docs/missions/20261019-apparatus-binding-runtime.md` ; équipe mission-45b81c878e91e264, lot Q.
- Date : 2026-10-20. HEADbase : `28e07383e53c1a0810dac1433feb7bd4e7226d60` (Lot A documentaire).
- Canon : ADR-0012 Accepted, Réalité Unimplemented ; contrat `docs/contracts/apparatus-knative-runtime.md` v1 Accepted le 2026-10-20, §§11–15. Ce plan ne donne aucun GO B/C/D/T/runtime.
- Méthode : impact × probabilité séparés ; obligations explicites issues du canon, oracles indépendants du code ; états GAP/blocked/unassessed ne signifient jamais preuve exécutée. Réévaluer après incident, lock nouveau ou changement d'interface ; signaler au lead toute contradiction PM sans changer son board.

## Pin observé

AIForAll : ADR Accepted > contrat v1 > code (réalité). Workspace Cargo racine ; compilation/tests Windows natifs, un Cargo à la fois. E2E uniquement `kind-aiforall-local` ; aucune autorisation implicite du full profile ou d'un autre cluster. Agents métier : architecte (décision), qa-planner (plan), hard-implementer (production), test-engineer (tests séparés), test-reviewer indépendant (audit). Pin remplaçable si profil copié, pas stratégie portable imposée.

Stratégie écrite ADR 0200–0202 : IT avec vraie infrastructure testcontainers, SQL réel pour transactions ; wiremock uniquement collaborateur HTTP sortant, jamais contrôle interne remplacé pour une preuve de sécurité ; OpenFGA réel quand `has_openfga()==true` (OpenFgaMockService interdit). Queues restent opt-in, aucun topic runtime ajouté. Kind réel pour Serving/réseau/UID/sandbox. Un mock Kubernetes ne prouve jamais le confinement. Fixtures existantes operator `tests/fixtures/{mod,chain,canary,evidence,with_kind}.rs`, `fixtures/kind/{cluster.yaml,ensure.rs,mod.rs}` ; nouvelle fixture `knative.rs` réservée à T.

Historique réutilisé : plan `20261018-m6-proof-hardening.md` documente skip silencieux par required-features, preuves insuffisantes par simple nom/digest, remplacements transitifs de fixtures et preuve réseau exigeant canary/contrôle positif. Ce sont des catégories attestées par le ledger de durcissement, non des fréquences de défaut mesurées. Aucun churn quantifié dans Q ; probabilités ci-dessous estimatives, fondées sur nouveauté et concurrence, pas sur un historique inventé.

## Faits, ambiguïtés et décisions

Faits : A ratifié ; pins B5 absents et compatibilité non prouvée. Valeurs B5 acceptées selon §15 ; leurs mentions anciennes « à ratifier » dans §§5/12 restent du texte historique non harmonisé. §12 préambule dit encore « tous restent à ratifier », contrairement à l'en-tête et §15 : appliquer l'Accept documenté, signaler la contradiction sans éditer le canon.

Le brief appelle « §11 (1–9) » : le contrat §11 contient **six** scénarios ; les **neuf** scénarios sont mission §6. Les deux listes sont couvertes et croisées ci-dessous, sans inventer des items 7–9 dans le contrat.

Décision humaine restante : lock exact B5, accord/périmètre du spike séparé et campagne E. Tous scénarios dépendant pins, valeurs effectives ou capacité : **NON MESURABLES tant que le lock n'est pas ratifié** ; déclarer les intents maintenant n'autorise aucun PASS. Gel opérationnel §13 à constater avant auteurs C/D/T (DTO complets, auth/rôles, hash canonique, raisons HTTP, CAS/epochs, migrations et garde legacy).

## Matrice du lot

Notation : critique 15–25 = automatisation nominale et erreurs ; haut 10–14 = automatisation ; moyen 5–9 = nominal et erreurs clés. Scores QA distincts du routage PM/Ensemble ; aucun score PM numérique fourni ici.

- R1 Caller/identité et bypass legacy : I5 × P4 = **20 critique**. Nouvelle séparation source/cible/enrollment multi-instance.
- R2 Révocation et effets pendant buffer/exécution : I5 × P5 = **25 critique**. Ordres distribués et issues ambiguës.
- R3 Isolation réseau/transport/sandbox : I5 × P4 = **20 critique**. Nouveau dataplane cold/hot et TCB.
- R4 Fencing, ownership et migration : I5 × P4 = **20 critique**. CAS DB/API non atomique, writers et ressources partagées.
- R5 Capacité/deadlines/retries/zéro : I4 × P4 = **16 critique**. Pins non disponibles et cumuls multi-Revision.
- R6 État externe/secrets/retrait : I5 × P3 = **15 critique**. Rotation et scope inter-bindings.
- R7 Contrats/erreurs/préfixes et régressions : I4 × P3 = **12 haut**. DTO nouveaux et anciens chemins conservés.

Modes de défaillance (détection/mitigation **actuelles**, pas garanties futures) :
- R1 : preuve source réutilisée en cible/replay CSR → autorité inter-bindings ; suites legacy invoke/mTLS détectent leurs seuls anciens cas ; mitigation prévue B1 registre authoritative ; **gap** aucune preuve Serving multi-instance/enrollment runtime.
- R2 : revoke entre admission et begin ou crash après envoi → effet non autorisé/dupliqué ; aucune preuve buffer actuelle ; B4 impose verrou writer/consommation CAS ; **gap** barrières DB+effet externe à construire.
- R3 : route chaude contourne Activator/headers forgés → compute ou I/O croisée ; canary/anciens tests P4 ne qualifient pas Istio/gVisor ; B2 default-deny et mTLS proposés ; **gap** lock et preuve réelle cold/hot.
- R4 : lease perdu/apply tardif/objet tiers même nom → résurrection ou suppression croisée ; anciennes suites P4/m6 historiques seulement ; B3 tombstones et préconditions UID prévus ; **gap** CAS réel et API Serving réelle, rollback migré.
- R5 : max-scale=0, attente cumulée ou retry après ambiguïté → épuisement/effets multiples ; anciens runtime tests non Serving ; profil B5 borné accepté ; **gap** paramètres standards vérifiés au pin et mesures, pas de queue maison.
- R6 : état local perdu ou secret scope digest → perte/fuite ; invoke KV/secrets legacy partiels ; B6 état externe et références seules ; **gap** zéro/réveil/rotation/retrait réel et inspection logs/contrôle SQL.
- R7 : version/hash/préfixe ou erreur modifiée → contrat client brisé ; suites P0/invoke préfixes existantes ; §13 gel et mapping explicites ; **gap** nouveaux DTO/raisons/cas négatifs runtime.

Passe heuristique : états closed→pré-déclaré zéro→ouvert→drain→closed, zéro↔instances ; frontières source/cible/projet/Revision/epoch ; ordre revoke/accept/begin/complete ; idempotence lease/report/enroll distincte d'invoke (aucun exactly-once). Concurrence inter-processus, deux bindings même release et deux replicas obligatoires.

## Registre obligation → intent → oracle → preuve

Toutes les obligations G ci-dessous sont **explicites**. Toutes sont **GAP**, ou **blocked** quand le lock/gel manque ; aucun COVERED Serving revendiqué. Effort relatif S=0,5 j, M=1 j, L=2 j de test-engineer hors setup/E (estimations, pas engagement). Chaque ligne comprend nominal et négatif ; pas de GAP critique accepté implicitement.

### G-1 — B1 caller/enrollment (R1 ; M+L)
Cibles T : `services/Lazaret/tests/apparatus_runtime_caller.rs`, `apparatus_runtime_enrollment.rs` ; IT services réels, crypto réelle et PostgreSQL testcontainer ; preuve Pod réelle en E.
Scénarios : workload source vivant autorisé invoque cible à zéro sans session cible ; refus droits source/cible manquants, Background absent, admin seul, project/binding/generation erronés. Enrollment avant Ready, répétition même preuve+CSR ; deux replicas et nouveau réveil. Négatifs audience/issuer/expiration/signature/UID/SA/namespace/Revision faux, CSR substituée, ancien Pod/session et replay après retrait.
Oracle : refus 401/403 selon B5 et raison stable, zéro forward/scale/effet pour admission initiale refusée ; registre instance↔Pod UID↔Revision↔binding/génération/release unique, même CSR idempotente, autre CSR jamais identité transférée ; identité source jamais cible. Lecture writer à chaque invoke/enroll/session, panne refuse. Anciennes WorkloadIdentity/SessionClaims inchangées.

### G-2 — B2 transport/isolation (R3 ; L, blocked lock)
Cible `workers/apparatus-operator/tests/apparatus_knative_runtime.rs`, fixture `knative.rs` ; E2E Kind réelle, rendu statique B en complément seulement.
Scénarios : cold gateway→Activator→queue-proxy et hot gateway→queue-proxy permis depuis Lazaret ; refus autre principal/binding, externe, direct Pod/Service/Revision/tag/user/admin/metrics et egress plugin→plugin/Transit/API/Internet ; falsifier Host/Forwarded/XFCC/K-*/Apparatus. Probes/metrics et plugin→Lazaret/DNS stricts restent fonctionnels.
Oracle : canary réellement disponible et contrôle positif dans la même fenêtre ; hop de rejet identifié, pairs certifiés/trust domain/SA vérifiés ; pas d'effet ni réveil au refus initial, pas seulement timeout. Ticket hash/cible/deadline validé même à chaud. RuntimeClass runsc réellement utilisé pour tout Pod, pas de runc fallback, privilèges/hostNetwork/hostPath/token API absents, arrêt de tous sidecars à zéro. Compatible seulement après preuve E ; aucun code hostile avant qualification.

### G-3 — B3 autorité, projection et ownership (R4 ; L)
Cibles `services/Manifesto/tests/apparatus_runtime_control.rs`, operator `apparatus_knative_projection.rs`, `apparatus_knative_fencing.rs`. IT PostgreSQL réel ; API réelle E pour UID.
Scénarios : snapshots paginés actifs/closed/tombstones et reports CAS idempotents ; deux writers inter-processus, restart/failover, lease perdu, CAS perdu, apply retardé après close, événement/cursor ancien, timeout create puis objet tiers même nom ; mauvais rôle machine/projet. Deux bindings même digest, retrait ciblé.
Oracle : monotonie génération métier/epoch/version/fence et cinq compteurs distincts ; ancien writer ne publie aucun allow ; mutation UID/resourceVersion exacte, objet tiers intact et STOP, aucune adoption par label/nom. Snapshot writer sans cache allow ; panne/expiration/mismatch refuse. Résiduel apply en vol toléré uniquement fermé à zéro, détecté puis retrait UID prouvé ; pas promesse atomique DB/API. Aucun client/token Kubernetes Manifesto/Lazaret ni dépendance cluster dans types neutres.

### G-4 — B4 autorisation avant effet et drain (R2 ; L)
Cibles `services/Manifesto/tests/apparatus_runtime_revocation.rs`, `services/Lazaret/tests/apparatus_runtime_effects.rs`, E runtime réel pour buffer.
Scénarios : barrières explicites avant/après admission, attente Activator, accept et commit begin ; revoke source/cible/identité/grant de part et d'autre de chaque barrière ; nouvelle génération. Deux accept concurrents même ticket, deux begin même effect, hash différent/replay ; panne writer, crash après envoi, complete perdu. Essayer `/invoke` legacy et toute autre capacité avec session runtime sans contexte.
Oracle : SQL writer ordonne commit revoke/begin sous mêmes verrous ; revoke avant begin ⇒ aucun effet nouveau (compteur/KV du collaborateur), accept unique et execution liée instance ; begin avant revoke ⇒ cet effet seul peut finir dans deadline, suivant refusé. Issue ambiguë persistée sans retry/ré-exécution, aucune revendication cancellation/exactly-once. Buffer peut encore réveiller CPU après revoke ; cela n'est pas un défaut si aucun accept/effet nouveau. Drain ferme d'abord, conserve engagements/unknown, ne tue pas les enfants Serving.

### G-5 — B5 lock, budgets, erreurs/retries (R5/R7 ; L, blocked)
Cibles `services/Lazaret/tests/apparatus_runtime_errors.rs`, operator `apparatus_knative_runtime.rs` ; statique lock/rendu puis E réel. **NON MESURABLE tant que lock ratifié absent**, y compris mesures dépendant capacité/valeurs effectives.
Scénarios : sous chaque borne puis à limite et au-delà (payload/attente/concurrence/Pods cumulés/CPU/mémoire/temp/quotas journal/TTL), coexistence Revision active+drain ; image impossible, scheduling absent, readiness bloquée, ingress/writer/projection absent, deadline épuisée ; injection envoi ambigu et double livraison native.
Oracle : settings et images exécutés égaux au lock transitif approuvé, max-scale et containerConcurrency >0, quotas pool explicites ; aucune limite globale inférée d'un seul proxy. Deadline absolue non remise à zéro, pas retry métier/effet ambigu, accept/effet durable unique pour mêmes clés. DTO→400, preuve→401, autorité/replay→403, payload→413, indisponibilité/saturation/deadline→502, interne→500 ; `{error:String}`, raisons snake_case sans secrets/URL ; pas QUEUED ni nouveaux 503/504/409/429. Un 502 ne prouve jamais absence d'effet. Préserver mapping legacy séparément.

### G-6 — B6 état/secrets/éligibilité (R6 ; L, E blocked lock)
Cible `crates/apparatus-reference-kv/tests/apparatus_runtime_zero.rs`, Lazaret effects/enrollment et Manifesto migration.
Scénarios : écrire KV via capacité, perdre Pod/mémoire/filesystem puis relire par instance neuve ; deux projets et deux bindings même projet/release ; rotation référence à zéro puis à chaud, retrait d'un binding, réinstallation ; profils websocket/background/migration startup incompatible refusés.
Oracle : donnée persistée sous projet/binding, autre binding intact, référence/version seules dans snapshot/events/tickets/logs/SQL de contrôle (scans avec secret canary unique), aucun credential durable env/monté ouvrant egress. Rotation à zéro crée zéro Pod ; epoch/grant invalident nouveaux effets anciens, pas promesse d'effacer plaintext déjà remis. Retrait conserve données/références historiques sans droit d'accès, ni suppression Secret/release partagé ; consentement explicite pour reprise. Migration incompatible interdit ouverture/rollback automatique, pas de migration par replica.

### G-7 — DTO/ports et compatibilité (R7 ; M)
Cible `crates/apparatus-contracts/tests/apparatus_runtime_v1.rs` ; tests contrat/unit, IT HTTP pour routes/rôles.
Scénarios : round-trip de chaque DTO §13 et vecteurs de hash canonique indépendants, standalone/préfixe monolithe protégés ; versions inconnues, champ matériel absent, hash altéré, rôle operator essayant effets ou Lazaret essayant activation/VALID refusés.
Oracle : schéma v1 gelé et golden vectors dérivés du contrat, pas snapshot copié du sérialiseur ; source/projet/cible/génération/grant non facultatifs ; RS256/purpose/issuer/audience dédiés ; aucun changement events/P0 ni queue obligatoire, aucune confiance loopback implicite.

### G-8 — Transition, rollback et retrait (R4/R6 ; L)
Cible `services/Manifesto/tests/apparatus_runtime_migration.rs`, operator fencing/projection ; SQL réel et E API réelle.
Scénarios : backfill historique ready→closed, migration par binding et inventaire tous consommateurs digest ; coupure ancien locator avant ouverture nouvelle route, crash à chaque étape §15 ; rollback compatible puis ancien binaire ignorant tombstones, UID tiers/partagé, nouvelle génération non concordante.
Oracle : jamais double dispatch, ancien event n'ouvre rien ; anciennes sessions/epochs invalides, route digest/local inaccessible pour migré. Objet partagé conservé tant que consommateur restant ; aucune suppression enfants Serving/Secret/release/données. Sur mismatch/bypass : close writer avant drain, nouveaux tickets/accept/begin refusés, preuves conservées. Binaire rollback incapable de refuser migré interdit ; pas réouverture auto ni downgrade SQL destructif.

## Traçabilité scénarios d'acceptation (sans doubler les obligations)

Mission §6 1→G3/G5 (contrat §11.1), 2→G1/G4 (contrat §11.1/5), 3→G1/G2/G6 (contrat §11.2), 4→G3/G4/G5 (contrat §11.3), 5→G1/G5/G6 (contrat §11.4), 6→G2 (contrat §11.2/5), 7→G3/G4 (contrat §11.5), 8→G3/G5 (contrat §11.6), 9→G3/G8 (contrat §11.6).
Activation : observer aussi toute création transitoire depuis activation jusqu'au premier invoke, après fenêtre de réconciliation et restart operator ; zéro snapshot final ne suffit pas. Retour zéro : Pods et sidecars absents, aucune demande de nœud plugin, route/état/references conservés, workload de contrôle système non compté comme plugin. Concurrence : plusieurs callers au même instant depuis zéro, un objet logique courant et replicas bornés ; mesurer les effets distinctement de l'idempotence de projection.

## Régressions existantes §14 : registre inverse et oracles à préserver

État de chacune : **unassessed pour exécution au nouveau HEAD**, pas COVERED Serving ; cibles réelles repérées, aucun run Q. T doit rattacher fichier:symbole et pourquoi aux assertions conservées avant PASS. Ces suites ne sont ni orphelines ni remplaçables par les nouvelles ; leurs preuves pré-Knative sont historiques et périmées pour une affirmation Serving, pas invalidées pour leur propre contrat.

- G-9 Manifesto `services/Manifesto/tests/{apparatus_p2_t7_gate,apparatus_p3_t2_gate,apparatus_p4_t1_absence}.rs` : gates d'absence inchangées ; oracle zéro `k8s`/`kubernetes` dans surfaces src prescrites et absence client/token cluster. Positif contrôle de détection sur corpus synthétique si fourni ; ne pas retargeter/exclure nouvelle production. S.
- G-10 Lazaret `services/Lazaret/tests/apparatus_p3_t7_invoke.rs` : symboles repérés `t7_invoke_kv_ops_require_live_grant_and_session`, `t7_stale_revision_and_foreign_binding_and_ungranted_op_denied`, `t7_secret_reference_and_named_connector` ; préserver KV permis et refus grant/session/scope/URL brute, payload et enveloppe historiques. `apparatus_p3_t9_invoke_prefix.rs` : `t9_public_lazaret_invoke_on_prefixed_router_is_200`, `t9_unprefixed_invoke_on_prefixed_router_is_404`, `t9_run_wiring_unauthenticated_invoke_is_401` : préfixe public existant et refus sans preuve. `apparatus_p3_t11b_session_mtls.rs` : session liée au pair certifié, refus substitution/absence ; ne pas confondre session caller et cible. M.
- G-11 Operator `workers/apparatus-operator/tests/{apparatus_p4_t10_platform_workload,apparatus_p4_t11_runtime}.rs` : workload plateforme/admission runtime autorisé et refus identité/admission invalide ; extensions ciblées vers Serving, séparation des responsabilités privilégiées et aucun accès API plugin/publisher. Historique Pod≠preuve Serving. M.
- G-12 Operator `apparatus_p4_t12_gate_regression.rs`, admission T7–T9 et `crates/apparatus-contracts/tests/apparatus_p01_micro.rs` : invariants P0, admission/signatures/digests et barrières d'absence restent valides ; bon artefact accepté, altéré/non admis refusé, activer ne produit jamais VALID/VERIFIED. T revalide noms exacts admission avant sélection, aucun renommage silencieux. M.
- G-13 Operator `apparatus_m6_e2e_chain.rs`, `apparatus_m6_fixture_oracles.rs`, `apparatus_m6_proof_oracles.rs` : chaîne réelle, oracles contrôles positifs/négatifs/ownership et preuves H1–H4 non affaiblis ; faux UID/artefact/policy ne donne pas PASS ; aucun remplacement tiers ou ancien cluster IT protégé. Relever required-features et comptage réel ; skip silencieux n'est pas succès. L.

Les nouvelles cibles §14 sont toutes attribuées G1–G8 (12 fichiers tests plus fixture knative). Tous fichiers tests/fixtures sont T, tests inline restent au propriétaire production ; aucun deuxième écrivain. Inventaire des doublons faibles : G2/G13 peuvent partager fixture/evidence mais pas compter deux fois la même observation ; G5/G6 zéro partagent campagne mais ont oracles capacité et persistance distincts. Un test sans obligation précise reste unassessed, pas automatiquement supprimé.

## Entrées / sorties par lot

- **A fait** : ratification et attribution acquises au HEADbase ; pins exclus de cette acquisition, aucune preuve runtime.
- **Q** : entrée contrat gelé Accepted ; sortie présent registre couvrant B1–B6, six scénarios contrat et neuf mission, régressions et décisions restantes explicites. Lead commite ce fichier avant worktrees auteurs ; Q ne commite pas.
- **B** : entrée Q+humain+ownership et pins/valeurs ratifiés ; exception seulement préparation strictement statique du lock autorisée séparément. Sortie lock complet/rendu sans illimité/public, graphe TLS/SA/ports/probes, compatibilité à prouver E et absence double writer ; pas installation live implicite.
- **C** : entrée A/Q, gel §13 effectif, profil/lock B disponible, ownership exclusif (Cargo global C seul). Sortie migrations additives/backfill fermé, writer+CAS/fencing/tombstones, Service bindé et UID protégés, absence cluster Manifesto ; inline tests et T coordonnés. Pas d'autorité VALID modifiée.
- **D** : mêmes gates et interfaces C publiées ; préparation parallèle seulement après gel. Sortie caller/ticket/enrollment multi-instance, aucun cache allow, B4 et bypass legacy fermés, locator bindé, deadline/mapping/retries conformes ; pas de DTO local divergent.
- **T** : entrée Q+gel et propriété tests ; build ciblé vert/fixtures/données décidées, slot Cargo Windows libre avant exécution. Sortie tous G actionnables implémentés avec oracles, SQL réel et réseau E prévu, cibles compilables nominatives ; bug production reste ROUGE renvoyé B/C/D, pas healer rendant le test artificiellement vert. Audit test-reviewer PASS ou findings traités, fmt/clippy sans régression. Ne pas promettre vert sans run.
- **E** : entrée code intégré corrigé, reviews indépendantes, gates Windows statiques, autorisation campagne séparée ; skills evidence/lifecycle et préflight remplacements transitifs, inventaire/baux/IDs Docker/UID, fixtures canary et writer disponibles. Windows ciblé puis IT finales puis seulement Kind. Sortie verdict par scénario/G, pins/config/UID/horodatages et effets, cold/hot/zéro/revoke/rollback ; zéro ressource possédée running après IT et cleanup possédé vérifié, pas prune/force/ressource inconnue. 0012 Implemented et 0009 nécessitent actes/revues explicites, jamais fermeture automatique de 0010/APP-xx.

Commandes futures : `cargo test --locked -p <package réel> --test <target ci-dessus> [--features requises] -- --test-threads=1`, mapping package/required-features à vérifier au HEAD T ; jamais `--workspace`, `--all-features` ni skip interprété PASS. La cible m6 historique nécessite `--features admit,controller` selon son plan ; T revérifie Cargo réel et liste/comptage. Aucun Cargo/runtime exécuté par Q.

## Gaps accept/defer et budget

- **GAP-defer B5** : humain/architecte+B doivent ratifier lock après spike explicitement autorisé, **avant B publication/C/D** ; tous seuils/compatibilité/mesures dépendants restent NON MESURABLES. Pas gap caller/effet/identité/fencing/isolation/ownership accepté.
- **GAP-defer décision humaine** : tout besoin browser autonome, cancellation forte, interdiction absolue de résiduel apply, workload non HTTP, migration destructive/purge et rollback données revient à A/owner avant extension. Restrictions workload caller/effet déjà engagé/unknown/résiduel fermé sont explicitement acceptées §15, pas des trous de tests.
- **GAP-accept hors tranche** : fermeture 0010/APP-xx, capacité production/SLO, exactly-once et coût système zéro non revendiqués ; la présente campagne n'en constitue pas une preuve. Toute modification de ces frontières impose nouveau contrat.
- **GAP-defer inventaire preuves** : T doit lire assertions réelles et nommer symboles/commandes des G9–G13 au HEAD futur avant run ; présence constatée n'est pas preuve. Gate de sortie E bloquée si obligation matérielle sans oracle ou couverture réelle. Aucun abandon implicite de haut risque.

Budget indicatif de capacité à confirmer par lead : 40 unités totales ; setup fixtures/données/barrières et environnement = 10 (25 %), implémentation des intents automatisés = 15, audits/triage/preuves = 5, buffer **10 (25 %)** nommé intégration pins/mesh-gVisor, courses revoke/CAS et fixtures. 75 % planifié ; setup inclus, non caché. Efforts G sont relatifs et chevauchent les campagnes, pas addition brute de jours. Tous risques étant hauts/critiques, capacité insuffisante ⇒ réduire le jalon fonctionnel/amender avec humain, pas supprimer un contrôle sécurité ni consommer buffer. Programmer d'abord G1/G4/G3 et contrats peu coûteux ; aucune coupe haute pour garder une moyenne. E estimé après lock, pas promis dans ce budget documentaire.

## Journal

2026-10-20 : plan initial sur HEADbase ; lecture contrat/mission/ADR 0009/0012/0200–0202 et handoff exécution, réutilisation du plan m6 ; inventaire ciblé des suites existantes (symboles sans exécution). Aucun code/test/runtime/cargo/commit ; seule écriture : ce plan. Aucun ID Docker créé, registre sans événement requis.
