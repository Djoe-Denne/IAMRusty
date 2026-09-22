# Compagnon de clôture — ADR-0008 (Apparatus P4 / Kind V1)

- **Rôle :** inventaire actionnable. **Ne remplace pas** [0008-apparatus-p4-k8s-isolation-outside-manifesto.md](0008-apparatus-p4-k8s-isolation-outside-manifesto.md).
- **Date :** 2026-09-22
- **Convention :** `docs/adr/README.md` — `Accepted` = cible ; `Réalité` = ce que le dépôt réalise.
- **Ce tour :** A-DEC Done (2026-09-22). Flip Réalité 0002, 0003, 0005, 0008 → Implemented. Pas de commit. Pas de SuperSède. `APP-05` reste ouvert. G/E non levées.

Catégories :

| Code | Sens |
|---|---|
| **A** | Travail pour clôturer 0008 (`Réalité: Implemented`) |
| **D** | Vrai hors-jalon — **laisser ouvert** explicitement |

Gravités : `BLOQUE-0008` | `STATUT-FAUX` | `HORS-JALON`.

---

## 1. Verdict

**0008 Implemented autorisé** (A-DEC 2026-09-22). Phrase utilisateur (exact) : « Je veux définitivement clôturer ces ADR. La fonctionnalité soit complète. Je n’ai pas de production. Si ça doit rester en dette/TODO, ça me va. » Même force que 0007 « Implemented autorisé sans fermer APP-05 ».

T2–T12 + chaîne Kind M1–M6 sont livrés. Kind = **l’environnement V1**. Les holes Transit `-dev` / Adm-B / pas de cluster prod sont **hors-jalon** : ils ne bloquent plus `Implemented`. Analogie 0006 (Implemented avec G/E encore en vigueur) et 0007 (Implemented avec `APP-05` ouvert).

[0007-closeout.md](0007-closeout.md) = photo **2026-09-20**. Ses lignes 0002 / 0003 / 0005 / 0008 sont **historiques**. **Ne pas** réécrire 0007-closeout. **Ne pas** rouvrir 0007.

---

## 2. Matrice ADR 0001–0008

| ID | Statut déclaré | Réalité déclarée | Verdict | Pourquoi | Action |
|---|---|---|---|---|---|
| [0001](0001-apparatus-binding-owned-by-manifesto.md) | Accepted | Partial | **ALIGNÉ** | P1 SQL livré ; pas de tenants ; backfill legacy **mis de côté** | Laisser Partial |
| [0002](0002-apparatus-contract-first.md) | Accepted | Implemented | **ALIGNÉ** (recast 2026-09-22) | Contrats + KV de référence ; pin descripteur M1–M6. Factory/host = P5/P6 **après** | Laisser Implemented |
| [0003](0003-apparatus-untrusted-plugin.md) | Accepted | Implemented | **ALIGNÉ** (recast 2026-09-22) | Isolation Kind/Calico + 4 SA sur chemin invoke. Harness test-only **peut rester** | Laisser Implemented ; pas de SuperSède |
| [0004](0004-apparatus-capability-gateway.md) | Accepted | Implemented | **ALIGNÉ** | Gateway Lazaret T5–T14b. `APP-05` = **D** | Ne pas toucher ; ne pas clôturer `APP-05` |
| [0005](0005-apparatus-same-protocol-valid-verified.md) | Accepted | Implemented | **ALIGNÉ** (recast 2026-09-22) | Adm-A = source VALID (persisté / Kind). Adm-B jamais source. VERIFIED = P6 **après** | Laisser Implemented ; pas de SuperSède |
| [0006](0006-apparatus-p2-reconciliation-in-process.md) | Accepted | Implemented | **ALIGNÉ** | T1–T7 + §D ; G/E en vigueur = **succès** du titre | Ne pas toucher ; **ne pas** lever G/E |
| [0007](0007-apparatus-p3-capability-boundary-after-accept.md) | Accepted | Implemented | **ALIGNÉ** (A-DEC 2026-09-20) | T1–T14b ; holes = **D** hors-jalon | Ne pas toucher ; ne pas clôturer `APP-05` |
| [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md) | Accepted | Implemented | **ALIGNÉ** (A-DEC 2026-09-22) | T2–T12 + M1–M6 ; Kind = V1 ; dette = **D** | Ne pas lever G/E ; ne pas SuperSéder 0003/0005 |

**Aucune ADR 0001–0008 n’est SUR-MARQUÉE** (rien n’est `Implemented` sans preuves Kind V1).

---

## 3. Preuves M1–M6 (déjà livrées — ne pas relivrer)

| Jalon | Preuve | Hole fermé |
|---|---|---|
| M1 | `apparatus-operator/tests/apparatus_m1_reference_kv_pin.rs` | Pin descripteur 0002 (64 hex, jamais `latest`) ; CRI ≠ enveloppe OCI |
| M2 | `apparatus-operator/tests/apparatus_m2_valid_persist.rs` | Adm-A seule écriture VALID ; store JSON produit |
| M3 | `apparatus-operator/tests/apparatus_m3_install_fail_closed.rs` | Install fail-closed si non-VALID |
| M4 | `apparatus-operator/tests/apparatus_m4_desired_state_bridge.rs` | Pont Manifesto HTTP 5 routes ; ticker P2 inchangé |
| M5 | `apparatus-operator/tests/apparatus_m5_invoke_isolated.rs` | Invoke Lazaret → Pod isolé ; T11 canary bloqué |
| M6 | `apparatus-operator/tests/apparatus_m6_e2e_chain.rs` (`m6_e2e_0002_0008_chain`) | Chaîne unique Kind bout en bout |

Usine déjà livrée (ne pas relivrer) : P4 T1–T12 ; Calico v3.29.7 ; Cosign fail-closed. Contrat : `docs/apparatus-0002-0008-next-milestones.md` (photo pré-flip **historique** pour la section B TCB).

---

## 4. Catégorie A — clôturer 0008

### A-DEC — Décision de convention

| | |
|---|---|
| Gravité | **BLOQUE-0008** |
| Preuve | Chaîne M1–M6 verte ; utilisateur sans production ; analogie 0006 / 0007 |
| Action | Accord humain : Transit `-dev`, Adm-B, cluster prod = **dette / hors-jalon**, pas des preuves manquantes. Kind = environnement V1. |
| Done | **Done (2026-09-22).** Phrase utilisateur (exact) : « Je veux définitivement clôturer ces ADR. La fonctionnalité soit complète. Je n’ai pas de production. Si ça doit rester en dette/TODO, ça me va. » |
| Dépend | — |

### A-0002 / A-0003 / A-0005 / A-0008 — Canon

| | |
|---|---|
| Gravité | **STATUT-FAUX** (sous-marqué) |
| Preuve | §3 ; A-DEC |
| Action | Flip Réalité **seulement**. Recast notes L11-style. Digest Serena même tour. |
| Done | `Réalité: Implemented` (docs 2026-09-22). |
| Interdit | SuperSède 0003/0005 ; lever G/E ; clôturer APP-05 ; ADR-0009 ; code Transit prod / webhook Adm-B / cluster cloud |

---

## 5. Catégorie D — laisser ouvert (explicite)

| ID | Sujet | Pourquoi rester ouvert | Preuve / ancre |
|---|---|---|---|
| **D-TRANSIT-TCB** | Transit encore `-dev` (≠ TCB release) | INTERDIT de prétendre que Transit `-dev` EST le TCB | 0008 L11 ; OpenBao T12 `-dev` |
| **D-ADMB** | Adm-B **non tranché** | Webhook optionnel ; **jamais** source VALID | 0008 Décision §2 ; 0005 |
| **D-PROD** | Pas d’environnement / cluster prod | N/A, pas un trou de code. Kind seulement = **l’environnement V1** | A-DEC 2026-09-22 |
| **D-APP05** | `APP-05` lecture anonyme / ops publiques | Checklist 0007 : **ne pas** clôturer. V1 privé | [0007-closeout.md](0007-closeout.md) §7 |
| **D-0006G** | Pas d’`invoke` sur `ApparatusRuntime` ; 0 `gateway` Manifesto src | Accept 0006/0007. **Invariant satisfait** | T1 P3 ; gate P2 |
| **D-0006E** | 5 routes `/components` ; pas de 202 | Accept 0006/0007 | `t7_components_route_count_stays_five` |

Hors-jalon déjà (rester) : K8s-as-P3 ; 2e protocole. **Après** 0002–0008 : P5 host/iframe/CLI ; P6 catalogue / `VERIFIED` ; PSS ; APP-03 / APP-06.

---

## 6. Références

- Canon : `docs/adr/0001` … `0008` ; closeout historique `docs/adr/0007-closeout.md`
- Méthode (non canon) : `docs/adr/0008-app01-reconciliation.md`
- Contrat jalons : `docs/apparatus-0002-0008-next-milestones.md` (preuves M1–M6 ; section B TCB = photo pré-A-DEC)
- IT : `apparatus-operator/tests/apparatus_m6_e2e_chain.rs`
