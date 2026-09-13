# Prompt — Implémentation Apparatus P3

Tu travailles dans le dépôt `C:\Users\djden\source\repos\AIForAll`. Ne commite rien.

## Contrainte modèle

Décision utilisateur **2026-09-12** (encore en vigueur sur Apparatus, sauf consigne contraire de l’utilisateur) : **seul Grok 4.6 Extra High** pour toute délégation, résolveur, agent, contre-agent et review de cette exécution P3.

- Slug Cursor obligatoire : `cursor-grok-4.6-xhigh` (paramètre `model:` de **chaque** Task / sous-agent).
- **Interdit nommément** : Muse Spark, `muse-spark-1.3-max`, Composer, tout autre modèle, et `inherit` s’il peut dévier vers un autre modèle.
- Lance **tous** les sous-agents de cette exécution P3 avec `model: cursor-grok-4.6-xhigh`. Un agent lancé sans ce slug = non conforme ; arrête et relance.

## Mission

Implémente **P3 — frontière de capacités et données** en TDD strict RED-GREEN-REFACTOR, **après** une ADR P3 (`0007+`) **Accepted** pour tout code identité workload / certificat / gateway / grants / consentement / KV plateforme / secrets / proxy / `invoke` **serveur**. Livre code, migration additive si l’ADR la nomme, tests et mises à jour documentaires **factuelles**. Ne dépasse pas P3.

**Ancre (2026-09-13, commit `8f37a0b`)** : ADR-0006 **Accepted**, Réalité **Implemented**. T1–T7 P2 + ticker `/ready` + CAS update + writer backoff §D livrés. `invoke` **serveur Manifesto absent**. `/components` gelé (5 registrations). Zéro type FGA `apparatus`. Port `ApparatusRuntime` **sans** `invoke`. Gate `Manifesto/tests/apparatus_p2_t7_gate.rs` **interdit encore** `kubernetes`/`k8s`, `wasm`/`wasi`/`wasmtime`, `iframe`, `messagechannel`, `gateway`, `apparatus_host`, `ui_host` **partout** dans `Manifesto/*/src` (y compris l’allowlist P2).

Conduis la tâche de bout en bout : Phase 0 ADR, exploration du code P2, implémentation tranche par tranche, tests ciblés, **retarget du gate P2 avant tout token gateway en prod**, formatage, diagnostics, compte rendu. Ne demande une clarification que si une décision matérielle absente des ADR **Accepted** bloque réellement l’implémentation. N’invente **aucune** décision `Accepted`.

## Accepted vs Réalité

`Accepted` = cible ratifiée. `Réalité` = code présent. Ne les confonds pas. Ne réécris pas les décisions ADR **Accepted** `0001`–`0006`.

Le wiki (plan, capabilities, platform) est **conception** (`^[inferred]`), **pas** une ADR, **pas** un contrat RED. Interdit de traiter le wiki comme Accepted, et d’inventer colonnes SQL, URLs, mTLS ou un microservice « parce que le wiki les nomme ».

ADR-0004 est **Accepted** (cible : toute I/O par une gateway de capacités ; KV plateforme ; pas de bearer IAM) et **Réalité Partial** (taxonomie P0 + port `KvStore` + harness ; **gateway réseau = P3**). Elle **ne fige pas** le mécanisme P3 (mTLS, rotation, produit secrets, transport d’identité).

### Contradictions ADR / code / wiki (ne pas lisser)

1. **ADR-0004 Accepted vs gate P2** : la cible ratifiée est la gateway ; `t7_p3_tokens_forbidden_even_on_p2_allowlist` interdit encore le token `gateway` dans tout `Manifesto/*/src`. P3 **doit retargeter ce gate avant** d’introduire le mot `gateway` en prod (même logique que P2 vs gate P1 qui interdisait `desired`/`observed`/`lease`…).
2. **ADR-0006 G** : « Pas d’`invoke` (P3). **Zéro** identifiant `gateway` sous `Manifesto/*/src`. » Toujours vrai dans le code. L’ADR P3 **Accepted** devra **lever explicitement** G pour le périmètre qu’elle nomme ; ne pas « oublier » G en codant.
3. **Le gate P2 n’interdit pas le token `invoke`** (absent de la liste `wasm`…`gateway`…`ui_host`). `grep invoke` sur `Manifesto/**/*.rs` = **0**. `invoke` existe comme **DTO P0** (`apparatus-contracts/src/protocol.rs`, `INVOKE_PATH = "/invoke"`) et dans le **harness in-process** ; ce n’est **pas** l’`invoke` serveur lié au binding. L’ADR P3 doit dire si `invoke` (et d’éventuels autres identifiants) deviennent des tokens de gate + allowlist. **Ne pas inventer** cette liste avant l’Accept.
4. **ADR-0001** : Manifesto propriétaire du **consentement** ; preuve P1/P2 : consentement **non ajouté**. ADR-0006 **M — Consentement : Hors P2**. Le consentement (capacités / révocation) est **dans P3**, pas une dette à bricoler sans ADR P3. Le paragraphe Réalité de `0001` cite encore « ADR-0006 **Proposed**, pas Accepted » — **texte périmé** (0006 est Accepted + Implemented). **Ne pas réécrire 0001** pour « corriger » ça pendant P3, sauf champ `Réalité` factuel après preuves, sans changer la décision Accepted.
5. **ADR-0004 Réalité Partial** vs wiki plan P3 `^[inferred]` (identité, certificats, grants, KV, secrets, proxy). Le wiki **informe** les noms de tranches ; il ne nomme pas le SQL.
6. **Wiki** `apparatus-implementation-plan.md` matrice d’acceptation : preuves worker P2 encore annotées « Partial » alors que le canon 0006 est **Implemented**. Pointer, ne pas promouvoir la matrice en contrat.
7. **Wiki** `apparatus-capabilities-and-isolation.md` (`status: proposed`) recommande mTLS + jeton de session gateway `^[inferred]`. **ADR-0004 Non décidé** : transport mTLS / rotation / durée de vie. Interdit de copier ce paragraphe dans un RED ou une ADR auto-Acceptée.
8. **Prompt P2** (`docs/apparatus-p2-implementation-prompt.md`) décrit P2 comme non implémenté — **obsolète**. L’état réel = 0006 Implemented / `8f37a0b`. S’en servir comme **modèle de discipline**, pas comme photo du dépôt.
9. **ADR-0003** : plugin hors processus privilégiés ; harness in-process ≠ production. P3 = refus **côté serveur** + identité workload ; **pas** le runtime d’isolation K8s/WASM (P4). Compatible avec une gateway **privilégiée** (Manifesto) sans charger le binaire communautaire.

Si une contradiction **bloque** le contrat (ex. vouloir un second protocole, `trusted_skip_gateway`, un bearer IAM, ou K8s comme isolation P3) : **escalade humaine**, n’invente pas.

## Sources de vérité à lire avant de coder

1. `AGENTS.md` et les règles du workspace.
2. `docs/adr/README.md` (prochain entier libre plage **0001–0099** = **0007** ; pas de fichier `0007-*` aujourd’hui).
3. ADR `0001`–`0006` dans `docs/adr/` (ne pas réécrire `Accepted`). Canon P3 cible : `0004`. Canon P2 livré : `0006`.
4. `docs/adr/template.md` pour la Phase 0. `docs/apparatus-p0-implementation-prompt.md`, `docs/apparatus-p1-implementation-prompt.md`, **`docs/apparatus-p2-implementation-prompt.md` entier** (esprit TDD + retarget de gate).
5. Canvas lecture seule : `C:\Users\djden\.cursor\projects\c-Users-djden-source-repos-AIForAll\canvases\apparatus-P0-audit-P1-TDD.canvas.tsx`. L’E2E « Contrôleur, workload, gateway, Factory » n’autorise pas K8s / Factory / host en P3.
6. QMD CLI, collection `aiforall-wiki` — **conception `^[inferred]` seulement** :
   - `projects/manifesto/references/apparatus-implementation-plan.md` (section **P3 — Frontière de capacités et données** + preuve de sortie)
   - `projects/manifesto/concepts/apparatus-capabilities-and-isolation.md`
   - `projects/manifesto/concepts/apparatus-platform.md`
   - `projects/manifesto/concepts/apparatus-bindings-and-lifecycle.md`
   - `projects/manifesto/concepts/apparatus-p2-reconciliation.md` (état P2, pas le mécanisme P3)
7. Code réel P2 (à étendre, pas à réécrire) :
   - `Manifesto/tests/apparatus_p2_t7_gate.rs` (**retarget obligatoire** avant tokens `gateway`)
   - `Manifesto/tests/apparatus_p2_t1_events.rs` … `apparatus_p2_t7_cleanup.rs` (régression)
   - `Manifesto/migration/src/m20260912_000013_apparatus_p2_runtime.rs` ; `Manifesto/infra/src/apparatus_runtime/`
   - `Manifesto/http/src/lib.rs` (5× `/api/projects/{project_id}/components`) ; `Manifesto/http/src/handlers/components.rs`
   - `Manifesto/infra/src/transaction.rs` ; `Manifesto/infra/src/apparatus_outbox.rs`
   - `Manifesto/tests/common.rs` ; `openfga/model.fga` (`grep apparatus` = 0)
   - Contrats P0 : `apparatus-contracts/src/protocol.rs` (`/bind` `/configure` `/unbind` `/invoke`) ; `apparatus-contracts/src/ports.rs` (`KvStore` + `ApparatusRuntime` **sans** `invoke`)
   - Harness / KV de référence : `apparatus-contracts` feature `test-harness`, `apparatus-reference-kv` — **double de test**, pas le KV plateforme persistant

Skills : `.agents/skills/rustycog/SKILL.md` (et jumeau `.cursor/skills/rustycog/SKILL.md`) si API RustyCog. Fixtures DB : `.agents/skills/creating-testcontainer-fixtures/SKILL.md`. HTTP live **si** l’ADR P3 ajoute des routes : `.agents/skills/creating-wiremock-fixtures/SKILL.md`. **Nouveau service** : `.agents/skills/aiforall-new-service/SKILL.md` **seulement si** l’ADR P3 Accepted l’exige. **Défaut Phase 0 : NON** — réutiliser Manifesto / rustycog (P2 a rejeté un microservice).

## Périmètre P3 obligatoire (TDD, une tranche après l’autre)

Invariants **toujours vrais** (sauf ADR Accepted qui les lève) : pas de second UUID public ; identité = `project_components.id` ; tuples `component:{id}` ; routes `/components` (5) ; events `ComponentAdded` / `ComponentRemoved` inchangés ; 1:1 ; `source` ∈ `legacy|managed` ; pas de nouveau type FGA sans escalade ; `component_type` non réécrit ; pas de `VALID` / `VERIFIED` persistants ; harness P0 ≠ runtime de production ; ticker P2 in-process conservé ; `desired_generation` **≠** `grant_revision` (0006 J) ; pas de `trusted_skip_gateway` (0005) ; pas de bearer / JWT / HMAC IAM dans le plugin (0004).

Cible wiki/0004 à **trancher dans l’ADR P3**, pas à copier comme spec : identité workload ; certificat/rotation ; gateway ; grants interactifs **et** de fond ; consentement/révocation ; KV plateforme namespacé par binding ; secrets par **référence** ; proxy réseau (connecteurs **nommés**) ; `invoke` côté serveur lié au **binding courant**. Projet public ≠ storage/invoke public (`APP-05` ouvert ; V1 privé).

Wiki preuve de sortie (**tests**, pas spec SQL) : deux projets + deux bindings adverses ; plugin incapable de changer de tenant, lire un secret, réutiliser un grant révoqué, joindre l’infra interne, invoquer une op non accordée ; suspension membre **immédiate** malgré projection FGA périmée.

Nomme les tests `Manifesto/tests/apparatus_p3_t*.rs` (esprit `t1`…`t7`).

### Défauts prudents (ce prompt, **pas** une ADR Accepted)

À ratifier ou écarter **dans l’ADR P3**, pas à coder avant Accept :

- **Pas** de nouveau microservice (P2 a rejeté ; 0001 V1 : pas un nouveau service RustyCog).
- **Pas** de Kubernetes / WASM / host / iframe / MessageChannel comme runtime d’isolation.
- **Pas** de `trusted_skip_gateway`.
- **Pas** de bearer IAM (0004 déjà Accepted : identité workload ≠ HMAC IAM).
- **Pas** de second protocole wire (réutiliser DTO P0 `/invoke`).
- **Pas** de HTTP `202` ni de levée du gel `/components` **sauf** si l’ADR P3 Accepted le dit (0006 E toujours en vigueur tant que non superSédé).

### Phase 0 — ADR P3 (bloquant avant T2–T7 code identité / mTLS / KV / proxy / invoke / consentement)

- RED : `docs/adr/` n’a **aucune** ADR `0007+` Accepted qui fige le mécanisme P3 (gateway réseau, identité workload, grants, consentement, KV persistant, secrets, proxy, `invoke` serveur). (État actuel : **aucune ADR P3** ; 0004 = cible, pas le mécanisme.)
- GREEN : rédiger `docs/adr/0007-….md` depuis `docs/adr/template.md` en **Proposed**. Titre = **la décision**, une phrase. L’implementer **n’accepte pas** lui-même. **STOP T2–T7** (et tout code identité/mTLS/KV/proxy/invoke/consentement) jusqu’à Accept **explicite** (utilisateur / PR). Puis ligne `docs/adr/README.md` + index wiki. Wiki reste conception. Digest Serena `architecture/apparatus-p3-adr-0007` **dans le même tour** que le fichier ADR (règle `adr-serena-digest`) ; pas de `*-proposed` orphelin une fois Accepted.
- Colonnes SQL, chemins d’allowlist gate, URLs, produit secrets, mTLS : **nommés seulement à l’Accept**. Checklist **sans préempter** les réponses.
- Sortie : ID d’ADR **Accepted** ; T1 peut avancer sans attendre. T2–T7 code **interdit** tant que non Accepted.
- Interdit : promouvoir le wiki en `Accepted` ; inventer SQL/URLs/mTLS « parce que le wiki les nomme ».

Checklist Phase 0 (questions **ouvertes** — ne pas y répondre dans le code ni dans une ADR auto-Acceptée) :

1. Gateway P3 : **in-process Manifesto** (comme le contrôleur P2) ou **nouveau** service RustyCog ? (skill `aiforall-new-service` seulement si nouveau.)
2. Transport d’identité workload : mTLS, rotation, TTL, autorité émettrice ? (0004 : **non figé**.)
3. Quelles tables/colonnes (grants, consentement, identité, KV) ? **Aucun nom tant que non Accept.**
4. `invoke` serveur : méthode sur `ApparatusRuntime`, **nouveau port**, ou handler HTTP réutilisant uniquement les DTO P0 ?
5. L’ADR P3 **lève-t-elle 0006 E** (nouvelles routes) tout en **gelant** les 5 `/components` ? Quelles routes, si oui — **pas d’URL inventée ici**.
6. Produit secrets (0004 Non décidé) : quoi, où, référence opaque seulement ?
7. KV plateforme persistant : quel adaptateur ? (harness P0 / `apparatus-reference-kv` **≠** cette décision.)
8. Tokens de gate + **chemins** d’allowlist : `gateway` est déjà gated ; `invoke` ne l’est pas. Que gater ? Où ?
9. Consentement **capacités / révocation** vs consentement **legacy→managed** (0001) : même ADR P3, ou legacy→managed **hors** P3 ?
10. Confirmer **zéro** nouveau type FGA (0001, 0004, 0006 J) ou **escalade**.
11. Connecteurs réseau nommés : qui les admet, comment, sans domaine arbitraire V1.
12. SuperSède / levée partielle de **0006 G** (zéro `gateway` sous Manifesto, pas d’`invoke` runtime) : libellé exact.
13. `APP-05` (invoke/KV publics) reste **ouvert** — ne pas le préempter.
14. Deux writers / lease P2 : l’`invoke` s’appuie-t-il sur le **binding courant** (génération + `grant_revision`) sans inventer un second fencing ?

### Tranche 1 — Preuves d’absence `invoke` serveur / `gateway` Manifesto (sans attendre l’ADR P3)

Autorisé **uniquement** par ADR **déjà Accepted** : 0006 G (pas d’`invoke` sur `ApparatusRuntime`, zéro identifiant `gateway` sous `Manifesto/*/src`) + 0004 Réalité **Partial** + 0005 (pas de `trusted_skip_gateway`). Analogie P2 T1 (0001 isolation events) : **geler** l’existant, pas introduire P3.

- RED : `Manifesto/tests/apparatus_p3_t1_absence.rs` (unit/contract) : trait `ApparatusRuntime` sans `invoke` ; aucun `invoke` dans `Manifesto/**/*.rs` ; gate P2 toujours rouge sur `gateway` en prod ; DTO sans `trusted_skip_gateway` ; `grep apparatus openfga/model.fga` = 0. **Ne pas** retargeter le gate. **Ne pas** ajouter de colonne SQL.
- GREEN : tests de caractérisation seulement.
- Sortie : P2 T7 inchangé ; baseline verte. Interdit : identité, mTLS, KV persistant, proxy, consentement, handler `invoke`.

### Tranche 2 — Retarget gate P2 → P4+ (APRÈS Phase 0, **avant** tokens `gateway` en prod)

- RED : retargeter `Manifesto/tests/apparatus_p2_t7_gate.rs` **avant** tout fichier prod contenant `gateway` (et tout autre token que l’ADR P3 **nomme**). Allowlist = **chemins figés par l’ADR P3 Accepted**, pas inventés. P4+ reste interdit **y compris** sur l’allowlist P3 : `kubernetes`/`k8s`, `wasm`/`wasi`/`wasmtime`, `iframe`, `messagechannel`, `apparatus_host`, `ui_host`. `factory` : allowlist inchangée (`ManifestoCommandRegistryFactory`). `VALID`/`VERIFIED` persistants toujours interdits. `/components` = 5.
- Si l’ADR nomme une migration / persistance (identité, grants, …) : tests DB via `Manifesto/tests/common.rs`, additive, `down` propre, **colonnes = ADR**, 1:1 et pas de second UUID public.
- GREEN : `apparatus_p3_t2_gate.rs` (+ `t2_*` persist si ADR). T7 P2 ne casse plus T2+.
- Sortie : gate P4+ vivant aussi dans `apparatus_p3_t2_*.rs` / T7 ; ne pas laisser `gateway` interdit à jamais **ni** l’autoriser partout.

### Tranche 3 — Identité workload + certificat/rotation (selon l’ADR)

- RED : identité = instance, binding, release, génération, `grant_revision`, audience gateway — **jamais** l’utilisateur, **jamais** le HMAC IAM (0004). Transport (certificat, rotation, TTL) **seulement** si l’ADR P3 Accepted le fige. Les phrases wiki mTLS sont des **exemples**.
- GREEN : pas de K8s token, pas de secret IAM dans le workload, pas de nouveau microservice sauf ADR.
- Sortie : `apparatus_p3_t3_identity.rs` (et `t3_certs.rs` si l’ADR sépare). Si l’ADR laisse mTLS en Non décidé : **ne pas** l’implémenter ; noter le trou.

### Tranche 4 — Gateway + grants interactifs et de fond

- RED : chaque opération (interactive **ou** fond) autorisée **à l’appel** par l’intersection 0004 : principal actif selon l’état **transactionnel DB** ∩ ACL projet/instance ∩ release admise ∩ capacités **déclarées** ∩ capacités **consenties** ∩ `desired_state` ∩ `grant_revision` courant ∩ politique opérateur. Capacité inconnue = refus. **Pas** `enforce_world_read_or_principal` comme autorisation d’invoke.
- GREEN : grants = contrat **distinct** d’OpenFGA (0004). Zéro nouveau type FGA sauf escalade.
- Sortie : `apparatus_p3_t4_gateway.rs`. Interdit : Factory, host, `trusted_skip_gateway`.

### Tranche 5 — Consentement, révocation, fermeture DB immédiate

- RED : l’état DB ferme l’accès dès le commit d’une suspension, révocation ou retrait de membre. Projection OpenFGA / cache périmé **insuffisant**. Grant révoqué non réutilisable. Distinguer consentement **capacités** et (si l’ADR l’inclut) passage legacy→managed.
- GREEN : selon l’ADR P3. AuthZ OpenFGA **seulement si** tuples/grants **utilisateur** changent ; sinon preuve d’absence FGA apparatus.
- Sortie : `apparatus_p3_t5_consent.rs`. Preuve suspension membre + FGA périmée (wiki → **test**, pas SQL copié).

### Tranche 6 — KV plateforme + secrets par référence

- RED : KV namespacé par binding (quota, CAS selon **ADR**). Pas d’accès SQL Manifesto/Hive/IAM pour l’extension. Secrets : référence opaque ; injection seulement dans l’opération **accordée** ; le plugin ne lit pas le secret.
- GREEN : pas le harness P0 comme « KV production ». Produit secrets = **ADR**, pas le wiki.
- Sortie : `apparatus_p3_t6_kv.rs`. Projet public n’ouvre pas le KV.

### Tranche 7 — Proxy réseau + `invoke` serveur lié au binding + preuves adverses

- RED : réseau sortant uniquement via proxy/gateway et connecteurs **nommés** admis (0004) ; pas d’URL interne / `fetchInternal` ; pas de domaine arbitraire V1. `invoke` **serveur** lié au **binding courant** (DTO P0, pas un second protocole). Preuves : deux projets + deux bindings adverses ; pas de changement de tenant ; pas de join infra interne ; op non accordée refusée.
- GREEN : pas de K8s NetworkPolicy mockée comme isolation ; E2E Factory/host/K8s = **P4–P6**.
- Sortie : `apparatus_p3_t7_invoke.rs` (+ gate P4+ vert). Cleanup P2 / ticker inchangés sauf ADR.

## Stratégie de tests (couches pertinentes P3 seulement)

- **Unitaires** : intersection de grants, révocation, `grant_revision` vs génération, refus capacité inconnue, identité sans HMAC.
- **Contract** : DTO P0 bind/configure/unbind/**invoke** ; pas de second protocole ; pas de `trusted_skip_gateway` ; pas de credentials dans les DTO.
- **Intégration DB** (Postgres testcontainer, `Manifesto/tests/common.rs`, skill testcontainers) : T2 persist si ADR, T5 fermeture à commit, T6 KV si SQL/CAS plateforme.
- **Intégration AuthZ** (OpenFGA réel) : **seulement** si tuples/grants utilisateur changent. Sinon `grep apparatus openfga/model.fga` = 0. Suite FGA « pour faire joli » = interdit.
- **Intégration HTTP** : seulement si l’ADR P3 ajoute des routes. Skill wiremock. Live + fixtures typées. `/components` gelé.
- **Pas** de harness parallèle. Les tests P0 in-process restent des tests Cargo, pas un runtime de production.
- **E2E Factory / host / K8s / WASM** : hors P3 (P4–P6).

## Hors périmètre strict

Ne pas ajouter : Factory, CLI, proc-macro, Git checkout, OCI, registry, signature, admission `VALID`/`VERIFIED` **persistants** ; host UI, iframe, CSP, MessageChannel, serveur de bundles ; Kubernetes / Docker runtime / WASM/WASI comme **runtime d’isolation** ; N instances du même Apparatus par projet ; `shared` / `organization` ; `APP-01` … `APP-07` préemptés ; second UUID public ; `trusted_skip_gateway` ; second protocole ; broker réel / binaire de contrôleur séparé (sauf si l’ADR P3 Accepted crée **explicitement** un service — défaut **NON**).

Préempter P4–P6 = interdit. Si un besoin P4+ apparaît, note-le en suivi.

## Contraintes de qualité

- Respecter les lints workspace (`unsafe_code = forbid`, Clippy pedantic/nursery/cargo).
- Éviter `unwrap`/`expect` dans le code de production ; documenter types/erreurs publics (`# Errors`, `# Panics` si requis).
- Dépendances minimales, versions du workspace ; migration additive, réversible, sans perte.
- Préserver les modifications utilisateur ; aucune commande Git destructive.

## Vérification

Exécuter au minimum :

1. `cargo fmt --all -- --check`
2. `cargo check` ciblé sur crates/migration/HTTP/runtime modifiés
3. `cargo test` ciblé (P0/P1/P2 non régressés + tranches P3, distinguées unit/contract/DB/AuthZ/HTTP)
4. `cargo clippy` niveaux workspace si le temps reste raisonnable
5. Diagnostics IDE sur les fichiers modifiés

Corriger toute régression introduite. Ne pas élargir aux défauts préexistants sans rapport. Le gate P2 retargeté doit rester **plus strict** que « tout Manifesto ».

## Documentation de sortie

Mettre à jour **uniquement les faits** après preuves : champ `Réalité` de 0004 (et 0007 une fois Accepted) `Partial`/`Implemented` **sans** modifier les décisions `Accepted` de `0001`–`0006` ; preuve P3 dans `apparatus-implementation-plan.md` ; index ADR + pointeur wiki. Canvas = lecture seule.

## Compte rendu final

Fournir : Phase 0 (ADR ID **0007+** ou **blocage** si non Accepted) ; où vit la gateway ; identité / grants / KV / proxy / `invoke` (selon l’ADR, pas le wiki) ; fichiers / migrations / tests ; critères par tranche TDD ; commandes + résultats (unit/contract/DB/AuthZ/HTTP) ; retarget gate (tokens + allowlist **réels**) ; éléments volontairement différés à P4–P6 ; risques ou questions réellement bloquantes (`APP-xx`, mTLS encore ouvert, secrets sans produit).
