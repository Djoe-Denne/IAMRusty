# Compagnon de clôture — ADR-0007 (Apparatus P3 / Lazaret)

- **Rôle :** inventaire actionnable. **Ne remplace pas** [0007-apparatus-p3-capability-boundary-after-accept.md](0007-apparatus-p3-capability-boundary-after-accept.md).
- **Date :** 2026-09-20
- **HEAD photographié :** `a27ea5b` (arbre sale : `IAMRusty/docker-compose.yml`, `Telegraph/docker-compose.yml`)
- **Convention :** `docs/adr/README.md` — `Accepted` = cible ; `Réalité` = ce que le dépôt réalise.
- **Ce tour :** A-DEC Done (2026-09-20). Flip Réalité 0004 et 0007 → Implemented. Pas de commit. Pas de SuperSède. `APP-05` reste ouvert.

Catégories :

| Code | Sens |
|---|---|
| **A** | Travail pour clôturer 0007 (`Réalité: Implemented`) |
| **B** | Ouvert *stale* — le code a fermé le trou ; les docs disent encore « ouvert » |
| **C** | Statut / note Réalité ADR faux ou sous-marqué |
| **D** | Vrai hors-jalon — **laisser ouvert** explicitement |

Gravités : `BLOQUE-0007` | `STATUT-FAUX` | `STALE-DOC` | `LEFTOVER` | `HORS-JALON`.

---

## 1. Verdict

**0007 Implemented autorisé** (A-DEC 2026-09-20). T1–T14b sont livrés. Les 4 holes (`APP-05`, 0006 G/E, K8s, 2e protocole) sont **hors-jalon** : ils ne bloquent plus `Implemented`. `D-APP05` **reste ouvert** (ne pas clôturer). Analogie 0006 (Implemented avec G/E encore en vigueur).

ADR-0004 `Réalité: Implemented` ce tour : L11/L63 ne citent plus OpenBao absent / `kv_purge` unwired / mTLS résiduel.

---

## 2. Matrice ADR 0001–0007

| ID | Statut déclaré | Réalité déclarée | Verdict | Pourquoi | Action |
|---|---|---|---|---|---|
| [0001](0001-apparatus-binding-owned-by-manifesto.md) | Accepted | Partial | **ALIGNÉ** | P1 SQL livré ; pas de tenants ; admission P4 = 0008 Partial ; backfill legacy **mis de côté** (0007 §9) | Laisser Partial |
| [0002](0002-apparatus-contract-first.md) | Accepted | Partial | **ALIGNÉ** | `apparatus-contracts` + `apparatus-reference-kv` ; Factory/host = P4 | Laisser Partial |
| [0003](0003-apparatus-untrusted-plugin.md) | Accepted | Partial | **ALIGNÉ** | Harness test-only ; moteur isolation = ADR-0008 **Partial** (`apparatus-operator`), **pas** « APP-01 encore ouvert » | Laisser Partial |
| [0004](0004-apparatus-capability-gateway.md) | Accepted | Implemented | **ALIGNÉ** (recast 2026-09-20) | Gateway+KV+secrets livrés T5–T14b. `APP-05` = **D** | Laisser Implemented ; ne pas clôturer `APP-05` |
| [0005](0005-apparatus-same-protocol-valid-verified.md) | Accepted | Partial | **ALIGNÉ** | Même wire ; 0 `VALID`/`VERIFIED` persistés ; pipeline admission = P4 | Laisser Partial |
| [0006](0006-apparatus-p2-reconciliation-in-process.md) | Accepted | Implemented | **ALIGNÉ** | T1–T7 + §D ; G/E en vigueur = **succès** du titre | Ne pas toucher ; **ne pas** lever G/E |
| [0007](0007-apparatus-p3-capability-boundary-after-accept.md) | Accepted | Implemented | **ALIGNÉ** (A-DEC 2026-09-20) | T1–T14b ; holes = **D** hors-jalon | Ne pas lever G/E ; ne pas clôturer `APP-05` |

README Vague 1 (`docs/adr/README.md` L24–L36) : 0004 et 0007 **Implemented** (A-DEC 2026-09-20). Pointeur closeout conservé.

Autre ADR citée (P3) : [0406](0406-crates-apparatus-p0-pas-le-host.md) Accepted / Partial — Partial **juste** (Factory/host absents) ; prose « gateway non livrée » **STALE** (**C-0406**). Pas d’audit 0100–0502 hors ce point.

**Aucune ADR 0001–0007 n’est SUR-MARQUÉE** (rien n’est `Implemented` sans preuves).

---

## 3. Preuves T1–T14b (déjà livrées — ne pas relivrer)

| Tranche | Preuve | Hole fermé |
|---|---|---|
| T1 | `Manifesto/tests/apparatus_p3_t1_absence.rs` | Pas d’`invoke` sur `ApparatusRuntime` ; 0 `gateway` Manifesto src |
| T2 | `Manifesto/tests/apparatus_p3_t2_gate.rs`, `apparatus_p3_t2_migration.rs` | Gate Lazaret P4+ ; `grant_revision` + `apparatus_capability_consents` |
| T3 | `Manifesto/tests/apparatus_p3_t3_identity.rs`, `Lazaret/tests/apparatus_p3_t3_identity.rs` | Identité hybride CSR → cert → session `iss`/`aud`=`lazaret` |
| T4 | `Manifesto/tests/apparatus_p3_t4_gateway.rs`, `Lazaret/tests/apparatus_p3_t4_grants.rs` | Consult live + intersection |
| T5 | `Manifesto/tests/apparatus_p3_t5_consent.rs`, `Lazaret/tests/apparatus_p3_t5_consent.rs` | Consent write ; close-at-commit |
| T6 | `Manifesto/tests/apparatus_p3_t6_kv.rs`, `Lazaret/tests/apparatus_p3_t6_kv.rs` | KV PG+Redis ; secrets-by-ref **wiremock** (volontaire) |
| T7 | `Manifesto/tests/apparatus_p3_t7_invoke.rs`, `Lazaret/tests/apparatus_p3_t7_invoke.rs` | `POST /invoke` ; proxy nommé ; `t7_public_project_without_consent_does_not_open_call` |
| T8 | `Lazaret/tests/apparatus_p3_t8_kv_purge.rs` | `kv_purge` ← `component_removed` / `lazaret-kv-events` |
| T9 | `Lazaret/tests/apparatus_p3_t9_invoke_prefix.rs` | `POST /lazaret/invoke` sur `prefixed_router` |
| T10 | `Lazaret/tests/apparatus_p3_t10_enrollment_persist.rs` | `apparatus_enrollments` ; revoke sur `component_removed` |
| T11b | `Lazaret/tests/apparatus_p3_t11b_session_mtls.rs` | `/session` HTTPS + client CA optionnelle |
| T12 | `Lazaret/tests/apparatus_p3_t12_openbao.rs` ; `docker-compose.yml` `openbao/openbao:2.6.2` | OpenBao **produit** ; T6 reste wiremock |
| T13 | `Lazaret/tests/apparatus_p3_t13_ca_persist.rs` | CA persist + TLS compose Lazaret |
| T14b | `Hive|IAMRusty|Telegraph/tests/https_mesh_optional_mtls.rs` ; compose racine `8443/8444/8445` | Mesh HTTPS + CA optionnelle, **pas** rustls required |

Artefacts : `Lazaret/migration` (`apparatus_kv_entries`, `apparatus_enrollments`) ; `Manifesto/migration` `m20260913_000014_apparatus_p3_grants.rs` ; `Lazaret/http` `POST /invoke` ; `apparatus-contracts` `KvStore::kv_put(..., expected_cas)`.

---

## 4. Catégorie A — clôturer 0007

### A-DEC — Décision de convention

| | |
|---|---|
| Gravité | **BLOQUE-0007** |
| Preuve | 0007 L11, L228 ; README L36 : holes « bloquent Implemented » **et** checklist 13/5/12 : ne pas les fermer |
| Action | Accord humain : les 4 holes sont **contraintes / hors-jalon**, pas des preuves manquantes. Analogie 0006 (Implemented avec G/E encore en vigueur). |
| Done | **Done (2026-09-20).** Phrase utilisateur (exact) : « Implemented autorisé sans fermer APP-05, sans lever G/E, sans K8s, sans second protocole. Ces items restent hors-jalon. » |
| Dépend | — |

### A-0007 — Canon 0007 (voir aussi C-0007)

| | |
|---|---|
| Gravité | **BLOQUE-0007** / **STATUT-FAUX** |
| Preuve | T1–T14b §3 ; L15 (contexte : « gateway réseau n’est pas livrée ») faux vs T7 |
| Action | `Réalité: Implemented`. Recast L11, L15, L209, L228 : holes = D, pas bloqueurs. Conséquences T1–T14b restent. |
| Done | En-tête Implemented ; plus aucune phrase « holes bloquent Implemented » (docs 2026-09-20). Digest Serena = orchestrateur. |
| Dépend | **A-DEC**. Même tour : digest Serena `architecture/apparatus-p3-adr-0007` (règle `adr-serena-digest`) |

### A-README — Index Vague 1

| | |
|---|---|
| Gravité | **BLOQUE-0007** |
| Preuve | `docs/adr/README.md` L31, L34–L36 |
| Action | 0007 (et 0004 si C-0004) → Implemented dans la table + paragraphe de ratification |
| Done | Table = fichiers ADR (2026-09-20). |
| Dépend | C-0007 ; C-0004 si flip simultané |

### A-C1 — Compose service-local IAM / Telegraph

| | |
|---|---|
| Gravité | **LEFTOVER** (T14b C-1) — pas un T15 |
| Preuve | Briefing `.cursor/review-briefings/20260920T1009Z-correctness-t14b-https-mesh-a27ea5b.md` ; `*/config/development.toml` `tls_enabled` ; diffs sales compose. Racine OK : IAM `8443:8443`, Telegraph `8444:8443`, Hive `8445:8443`. Hive **n’a pas** de `Hive/docker-compose.yml`. Diff Telegraph locale `8443:8443` = **collision**. |
| Action | Répliquer `platform-mesh-certs` + volume `../certs/platform-mesh:/app/certs` **sans** collision ports racine, **ou** documenter « unsupported — stack racine only ». Ne pas désactiver TLS du profil racine. |
| Done | `docker compose -f IAMRusty/docker-compose.yml` (et Telegraph) boot **ou** handbook explicite ; diffs commitées ou abandonnées |
| Dépend | — (peut précéder A-DEC) |
| Tests | `cargo test -p iamrusty-service --test https_mesh_optional_mtls` (+ Telegraph, Hive) ; smoke compose si réparation |

### A-LAZ-README — `Lazaret/README.md`

| | |
|---|---|
| Gravité | **STALE-DOC** |
| Preuve | L3 « Slice T2 : health / ready uniquement » vs T3–T14b. `docs/services/lazaret.md` déjà plus à jour. |
| Action | Aligner sur T3–T14b (enroll, session, invoke, KV, OpenBao). |
| Done | Plus de « T2 only » (2026-09-20). |
| Dépend | — |

---

## 5. Catégorie B — ouvert stale (fermer dans les docs)

### B-0004-NOTES — 0004 L11 / L63

| | |
|---|---|
| Gravité | **STALE-DOC** / **STATUT-FAUX** |
| Preuve | L11/L63 : « OpenBao produit absent », « `kv_purge` non branché », « mTLS rustls residual ». Contredit T8, T11b, T12 (`docker-compose.yml` `openbao:`), T14b |
| Action | Réécrire avec C-0004. Pointer transport/secrets vers 0007. |
| Done | Plus aucun de ces trois claims (2026-09-20). |
| Dépend | C-0004 |

### B-0004-NONDECIDE — 0004 L56–L57

| | |
|---|---|
| Gravité | **STALE-DOC** |
| Preuve | « Non décidé : produit secrets ; transport mTLS ». Tranché en 0007 (OpenBao T12 ; hybride T3/T11b). `APP-05`/`APP-03`/`APP-06` restent Non décidé (**D**) |
| Action | Remplacer L56–L57 par pointeur 0007 ; ne pas figer TTL/CA ici |
| Done | Plus de « Non décidé » sur secrets/mTLS (2026-09-20). `APP-05` A/B **non tranchées**. |
| Dépend | — |

### B-0007-L15 — Contexte 0004 dans 0007

| | |
|---|---|
| Gravité | **STALE-DOC** |
| Preuve | 0007 L15 : « La gateway **réseau** n’est pas livrée » |
| Action | Historiser (« au moment de 0004 ») ou pointer T7 |
| Done | L15 n’affirme plus l’absence (historisé 2026-09-20). |
| Dépend | A-0007 |

### B-0406 — Titre / L17 / L22 / L33

| | |
|---|---|
| Gravité | **STALE-DOC** |
| Preuve | « gateway ne sont pas livrés » / « pas une gateway ». Lazaret **est** la gateway P3. Factory/host restent absents |
| Action | Voir **C-0406**. Partial conservé |
| Done | Prose : gateway = Lazaret ; non-livré = Factory/host/K8s |
| Dépend | — |

### B-PLAN — Wiki plan P3-close

| | |
|---|---|
| Gravité | **STALE-DOC** |
| Preuve | `obsidian/AI FOR ALL/projects/manifesto/references/apparatus-implementation-plan.md` L117, L127, L138 : mTLS / OpenBao encore holes ; L35 « prochain = P3-close » alors que T8 est livré (`f0cf1d2`) |
| Action | État 2026-09-20 : T1–T14b ; holes restants = **D** seulement |
| Done | Plus de mTLS/OpenBao comme holes ouverts |
| Dépend | — |

### B-WIKI-0004 — Hub wiki 0004

| | |
|---|---|
| Gravité | **STALE-DOC** |
| Preuve | `obsidian/AI FOR ALL/projects/manifesto/decisions/0004-apparatus-gateway.md` (décalage OpenBao/`kv_purge`/mTLS) |
| Action | Aligner sur canon après C-0004 |
| Done | `feature_status` / résumé = Implemented (2026-09-20). `APP-05` A/B non tranchées. |
| Dépend | C-0004 |

### B-SERENA-0004 — Digest Serena 0004

| | |
|---|---|
| Gravité | **STALE-DOC** |
| Preuve | `.serena/memories/architecture/apparatus-p0-adr-0004` (claims OpenBao/`kv_purge`/mTLS) |
| Action | Même tour que C-0004 (`adr-serena-digest`) |
| Done | Digest = Statut + Réalité du fichier |
| Dépend | C-0004 |

### B-MESH-PORTS — Wiki mesh vs compose

| | |
|---|---|
| Gravité | **STALE-DOC** |
| Preuve | Wiki `https-platform-mesh` : Hive 8443 / IAM 8444 / Telegraph 8445. Compose racine : **IAM 8443, Telegraph 8444, Hive 8445** |
| Action | Corriger le wiki (le compose est la preuve T14b) |
| Done | Ports wiki = `docker-compose.yml` |
| Dépend | — |

### B-T11B-SHA — SHA T11b wiki

| | |
|---|---|
| Gravité | **STALE-DOC** |
| Preuve | Wiki attribue T11b à `d605377` (« chore: update subproject reference in rustycog ») = pin, pas le sujet métier |
| Action | Citer le commit du fichier `apparatus_p3_t11b_session_mtls.rs` / land HTTPS |
| Done | SHA = commit de preuve |
| Dépend | — |

### B-WIKI-0007-OK

Hubs `decisions/0007-apparatus-p3-lazaret.md` et `projects/lazaret/lazaret.md` : `feature_status: implemented` (2026-09-20). **Ne pas** « fermer » APP-05 dans le wiki.

---

## 6. Catégorie C — statuts ADR à corriger

### C-0007 — 0007 Réalité Partial → Implemented

| | |
|---|---|
| Gravité | **STATUT-FAUX** (sous-marqué) |
| Preuve | §3 ; Accept 1–14 ratifié 2026-09-13 |
| Action | Flip Réalité **seulement**. Recast L228. Digest Serena. |
| Done | `Réalité: Implemented` (docs 2026-09-20). Digest Serena = orchestrateur. |
| Dépend | **A-DEC** |
| Interdit | Lever G/E ; SuperSède 0006 ; clôturer APP-05 ; T15 inventé |

### C-0004 — 0004 Réalité Partial → Implemented

| | |
|---|---|
| Gravité | **STATUT-FAUX** (sous-marqué) — **plus gros mismatch** |
| Preuve | Jalon L7 = P0 contrat + P3 gateway. P3 = T5–T14b. L11/L63 raisons Partial **fausses**. `APP-05` = Non décidé (**D**), pas un motif Partial |
| Action | Flip Implemented ; réécrire L11, L37, L63 ; L56–L57 → B-0004-NONDECIDE |
| Done | Réalité Implemented ; plus de « OpenBao absent / kv_purge unwired / mTLS residual » (2026-09-20). |
| Dépend | Accord (peut aller avec A-DEC). Ne pas exiger rustls client-required (T14b a fermé autrement) |

### C-0406 — 0406 notes gateway

| | |
|---|---|
| Gravité | **STATUT-FAUX** (notes) — label Partial **juste** |
| Preuve | Titre + L17, L22, L33 |
| Action | Photographier Lazaret comme gateway P3 ; garder Partial pour Factory/host |
| Done | Titre/prose ne nient plus la gateway |
| Dépend | — |

**Alignés, pas d’action C :** 0001, 0002, 0003, 0005 (Partial honnête), 0006 (Implemented honnête).

---

## 7. Catégorie D — laisser ouvert (explicite)

| ID | Sujet | Pourquoi rester ouvert | Preuve / ancre |
|---|---|---|---|
| D-APP05 | `APP-05` lecture anonyme / ops publiques | Checklist 13 : **ne pas** clôturer. V1 privé. Comportement déjà testé | 0007 L187–L189 ; `t7_public_project_without_consent_does_not_open_call` |
| D-0006G | Pas d’`invoke` sur `ApparatusRuntime` ; 0 `gateway` Manifesto src | Accept point 12. **Invariant satisfait** | T1 ; gate P2 `apparatus_p2_t7_gate.rs` |
| D-0006E | 5 routes `/components` ; pas de 202 | Accept point 5 | `t7_components_route_count_stays_five` |
| D-K8S | Isolation K8s | Hors P3 ; `APP-01` / P4 | 0007 L211, L230 ; 0003 |
| D-PROTO2 | Second protocole / `trusted_skip_gateway` | Escalade ; 0005 | 0007 L211 |
| D-P4 | Factory, host UI, OCI, `VALID`/`VERIFIED`, iframe | 0002 / 0005 / 0406 | — |
| D-APP01 | Moteur isolation | 0003 | — |
| D-APP03 | Rétention / purge hors `kv_purge` binding | 0004 Non décidé | T8 ≠ politique rétention |
| D-APP06 | Quotas chiffrés / SLO | 0004 Non décidé | Quota P0 256 clés = impl, pas APP-06 |
| D-CA | Produit CA, TTL/rotation chiffrés, keypair injecté | 0007 « Non décidé » | Défauts T3/T13 non figés |
| D-REDIS | Layout clés Redis / TTL KV | Point 7 non figé | Adaptateur Redis existe |
| D-SM | Adaptateur Secrets Manager cloud | Point 6 : plus tard | V1 = OpenBao |
| D-T6WM | IT T6 wiremock | Volontaire ≠ T12 | Ne pas « unifier » |
| D-LEGACY | Backfill legacy→managed | Checklist 9 ; jamais déployé | 0001 L11 |
| D-S12 | Clés mesh `644` / volumes RW | Résidu compose-dev T14b accepté (S-1/S-2 MEDIUM) | Briefing security T14b |
| D-T2IT | `Err(_)` permissif cert étranger mesh | T-2 LOW hors merge | Briefing tests T14b |

---

## 8. Ordre d’exécution recommandé

**Fait 2026-09-20 (docs, pas de commit) :** A-DEC, C-0007, A-0007, A-README, B-0007-L15, A-LAZ-README, C-0004, B-0004-NOTES, B-0004-NONDECIDE, B-WIKI-0004, wiki 0007 `feature_status`. Digest Serena = orchestrateur.

**Encore ouvert :**

1. **A-C1** (indépendant, arbre sale) — hors ce tour docs.
2. **B-SERENA-0004** / digest 0007 — orchestrateur.
3. **C-0406**, **B-PLAN**, **B-MESH-PORTS**, **B-T11B-SHA** — SKIP ce tour.

Interdit en parallèle : lever G/E, SuperSède 0006, clôturer APP-05, K8s P3, rustls required, crate `lazaret-events`, API Manifesto « pour Lazaret ».

---

## 9. Checklist d’exhaustivité

| Sujet demandé | Couvert |
|---|---|
| T1–T14b | §3 tableau + « ne pas relivrer » |
| APP-05 / G / E / K8s / 2e proto | A-DEC, C-0007, **D-*** |
| C-1 compose IAM/Telegraph | **A-C1** |
| 0004 L11 | **C-0004**, **B-0004-NOTES** |
| Wiki / README stale | **A-README**, **A-LAZ-README**, **B-PLAN**, **B-WIKI-0004**, **B-MESH-PORTS** |
| Statuts 0001–0007 | §2 ; aucun sur-marqué |
| 0406 gateway | **C-0406** |

---

## 10. Références

- Canon : `docs/adr/0001` … `0007`, `0406`, `docs/adr/README.md`
- Prompt P3 : `docs/apparatus-p3-implementation-prompt.md`
- Wiki : `projects/manifesto/decisions/0007-apparatus-p3-lazaret.md`, `projects/lazaret/lazaret.md`, `projects/manifesto/references/apparatus-implementation-plan.md`, `projects/aiforall/concepts/https-platform-mesh.md`
- Briefings T14b : `.cursor/review-briefings/20260920T1009Z-correctness-t14b-https-mesh-a27ea5b.md`, `20260920T1015Z-security-t14b-platform-mesh-tls-a27ea5b.md`
- Inventaire session : `.cursor/review-briefings/20260920T1048Z-orchestrator-adr0007-remaining-a27ea5b.md`
