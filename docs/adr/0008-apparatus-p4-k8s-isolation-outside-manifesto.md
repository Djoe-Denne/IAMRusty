# ADR-0008 : Le moteur d’isolation Apparatus P4 est Kubernetes hors de Manifesto, avec Cosign et OpenBao Transit, une admission worker-only, un operator plus Jobs sans nest HTTP, une enveloppe OCI distincte de l’image CRI, et quatre ServiceAccounts

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-09-20
- Décideurs : Architecture AIForAll — ratification chat « Je valide tout. Je ratifie tout. » 2026-09-20 (même force que 0007 « accepté » 2026-09-13). README L17 / L158 : `Accepted` typiquement après PR ; **écart documenté** comme 0006 (chat 2026-09-12) et 0007. A-DEC Réalité Implemented 2026-09-22 (holes = hors-jalon).
- Jalon concerné : P4
- SuperSède : aucune
- SuperSédée par : —

`Accepted` ratifie la cible ci-dessous. `Réalité : Implemented` (A-DEC 2026-09-22) : mécanisme P4-core dans `apparatus-operator` (T2–T12) + chaîne Kind M1–M6 (`m6_e2e_0002_0008_chain`) : pin descripteur 0002 → VALID JSON Adm-A → install fail-closed → pont Manifesto HTTP 5 routes → invoke Lazaret → Pod isolé, T11 canary bloqué. Preuve usine : `apparatus_p4_t6_sign_registry` (zot+Transit), `apparatus_p4_t7_admission` / `t8_refus` / `t9_malicious_worker`, IT Kind `apparatus-p4-it` (`apparatus_p4_t10_platform_workload`, `apparatus_p4_t11_runtime`), gates `apparatus_p4_t12_gate_regression` + T1/P2 T7/P3 T2 Manifesto + Lazaret `apparatus_p3_t7_invoke`. Kind IT **Calico v3.29.7** (`disableDefaultCNI: true`, kindnet interdit) ; T11 `t11_plugin_exec_cannot_reach_transit_or_system` **vert** ; Cosign `verify` **fail-closed** (présence du tag `.sig` ≠ preuve d’admission). Kind = environnement V1. T1 reste une preuve d’**absence** dans Manifesto (pas de `Factory/`, pas de `KubernetesAdapter`, zéro token `k8s`/`kubernetes` sous `Manifesto/*/src`). Cette ADR **ne SuperSède pas** 0003 ni 0005. 0002 / 0003 / 0005 / 0008 = **Implemented** (ce tour). 0004 / 0006 / 0007 restent **Implemented**. 0001 reste **Partial**. A-DEC 2026-09-20 **inchangé** (APP-05 ouvert ; G/E restent ; pas de second protocole). Hors-jalon (ne bloquent **pas** Implemented, A-DEC 2026-09-22) : **D-TRANSIT-TCB** (Transit `-dev` ≠ TCB release — INTERDIT de prétendre que Transit `-dev` EST le TCB) ; **D-ADMB** (Adm-B webhook optionnel, jamais source VALID) ; **D-PROD** (pas d’environnement / cluster prod — N/A, pas un trou de code).

Le compagnon `docs/adr/0008-app01-reconciliation.md` est la **méthode** de réconciliation α/β (2026-09-20), **pas** le canon. Inventaire de clôture (faits, pas nouvelle cible) : `docs/adr/0008-closeout.md`.

## Contexte

ADR-0003 **Accepted** exige quatre identités OS distinctes (plugin hors processus privilégiés ; workers Factory, admission/signature, contrôleur, gateway). ADR-0005 **Accepted** exige une admission indépendante du publisher, du plugin, du harness et de `VERIFIED` ; register Manifesto ≠ admit ; harness / `apparatus dev` jamais `VALID`/`VERIFIED`. ADR-0002 **Accepted** : identité d’installation = digest du descripteur, jamais `latest` ; Factory = P4. ADR-0004 / 0007 : `invoke` = Lazaret `POST /invoke` ; zéro identifiant `gateway` sous `Manifesto/*/src`.

Jusqu’ici `APP-01` (moteur, registry, signature, admission, processus) restait un arbitrage ouvert. README L18 interdisait d’en faire une ADR tant que le jalon n’ouvrait pas. **P4 ouvre ce jalon.** A-DEC interdit Kubernetes **comme isolation P3** ; les gates Manifesto (`apparatus_p2_t7_gate.rs`, `apparatus_p3_t2_gate.rs`, T1 P4) interdisent les tokens `k8s`/`kubernetes` sous `Manifesto/*/src`. Un gel humain avait déjà nommé **K8s en P4, hors Manifesto**. Le paquet registry / admission / BC / package / processus a été réconcilié le 2026-09-20 puis ratifié en chat.

Le wiki (plan L152 `KubernetesAdapter`, factory `status: proposed`) reste **conception**. Interdit de le copier comme contrat, d’inventer un répertoire `Factory/`, ou de retargeter les gates Manifesto.

## Décision

Le moteur d’isolation Apparatus **P4** est **Kubernetes hors de Manifesto**. K8s-as-P3 reste **interdit**. **Zéro** token `k8s` / `kubernetes` sous `Manifesto/*/src` (gates P2 T7, P3 T2, P4 T1 **non retargetés**).

Le paquet ratifié 2026-09-20 est :

1. **Reg-A+D-ACL portable** — Cosign + OpenBao **Transit** (distinct du KV plugin Lazaret T12) + registre OCI **portable** + push **signer-only**. ACL **sur le registre**, pas sur Kubernetes.
2. **Adm-A** — worker OS privilégié, **sans** BC HTTP, **seule** source de `VALID` (digest 0002 + politique + rapport). **Adm-B** (webhook) = enforceur runtime **optionnel**, **jamais** source.
3. **BC-A** — operator + Jobs ; **pas** de nest HTTP dans le monolithe ; **pas** de nom `Factory` ; skill `aiforall-new-service` **non**. Repli **BC-D** seulement plus tard **si** APP-02 / P5 impose un POST hors-cluster (hors P4-core).
4. **Pkg-B** — enveloppe OCI/ORAS **non** CRI + blob image CRI pinée. Identité d’installation = digest descripteur **0002** seulement. Le kubelet n’exécute que `image@sha256` issu de l’enveloppe **admise**.
5. **Run-A** — **quatre ServiceAccounts** Kubernetes (les quatre identités 0003). Job de build **sans** token API. Plugins dans **un autre namespace**. Contrôleur P4 ≠ ticker P2 Manifesto.

A-DEC 2026-09-20 **inchangé** : `APP-05` ouvert ; 0006 G et E restent ; pas de second protocole ; `invoke` = Lazaret. WASM n’est **pas** le premier runtime V1 (0003).

Photo post-M6 (A-DEC 2026-09-22) : 0002 / 0003 / 0005 / 0008 = **Implemented** ; 0008 **ne lève pas** G/E, **ne SuperSède pas** 0003/0005.

## Conséquences

- T2–T12 du prompt P4 sont **canoniquement débloqués** par cet Accept. Preuves T2–T12 + M1–M6 livrées sous `apparatus-operator` (Réalité **Implemented**, A-DEC 2026-09-22). Pas de `KubernetesAdapter` Manifesto, pas de `Factory/`, pas de nest HTTP.
- T1 absence **déjà livrée** : ne pas relivrer. T1 ≠ TCB prod (**D-PROD** / **D-TRANSIT-TCB**) : ne bloque **plus** Implemented. Kind = environnement V1.
- L’operator et les Jobs vivent **hors** `Manifesto/*/src` et **hors** Lazaret. Zéro token `k8s`/`kubernetes` dans Manifesto même si l’operator existe ailleurs.
- Isoler OpenBao Transit du mount KV plugin (T12 `-dev` ≠ TCB de release). Seul le SA signer pousse et appelle Transit.
- Le registre d’IT doit servir **artifacts et** pull d’image. Ne pas présumer `registry:2` suffisant.
- Indépendance 0005 = processus / SA + rapport signé, **pas** un GET HTTP.
- Catalogue : stocker le digest **0002**, pas le digest d’enveloppe comme identité. `spec.image` du Pod ≠ digest d’enveloppe.
- Skill `aiforall-new-service` **non** pour ce BC (pas de 6ᵉ hexagone HTTP).
- Wiki et canvas restent conception / lecture seule.

## Alternatives rejetées

| Option | Pourquoi pas (maintenant) |
|---|---|
| K8s-as-P3 / tokens K8s dans Manifesto | A-DEC + 0007 ; gates restent |
| WASM premier runtime V1 | 0003 |
| Reg-B KMS cloud | TCB extra ; OpenBao T12 existe |
| Reg-C unpack / Reg-D in-cluster | TCB / surface |
| ACL Kubernetes plutôt que registre | builder hostile |
| Transit = KV Lazaret | forge de signatures |
| Admission HTTP / nest / 6ᵉ hexagone | surface IAM ; skill new-service |
| Adm-B / C / D comme source `VALID` | 0005 indépendance |
| Consumer d’events register → `VALID` | Adm-D furtif |
| Pkg-A (paquet = image) | kubelet tourne des octets non admis |
| Run-B (jeton CI = publish) / Run-D un hôte | 0003 quatre identités |
| SuperSède 0003 / 0005 | canon P4 |
| `KubernetesAdapter` dans Manifesto / wiki L152 | conception, pas contrat |

## Non décidé ici

- `APP-02` (politique catalogue / publishers)
- `APP-05` (options A/B dans 0004 — **reste ouvert**, A-DEC)
- `APP-03`, `APP-06` (dont **budget / CPU numériques**)
- Drain / destruction des bindings déjà `ready` (0005, README) ; **ne pas shipper en prod** la limite pod-H24 / partage-par-digest — gate pré-prod [0009](0009-gate-preprod-scale-on-demand-isolation-instance.md) (scale on demand + isolation d’instance explicite)
- Split admission vs signer (colocation 0003 possible, **pas** tranchée)
- Produit registry IT concret (zot / ORAS vs `registry:2`) — contrainte : artifacts **et** pull d’image
- Adm-B webhook comme complément **après** cette ADR, jamais source `VALID`

## Références

- Canon voisin : ADR-0001 … 0008 (`docs/adr/`) ; closeout historique `docs/adr/0007-closeout.md` ; closeout 0008 `docs/adr/0008-closeout.md`
- Méthode (non canon) : `docs/adr/0008-app01-reconciliation.md`
- Prompt : `docs/apparatus-p4-implementation-prompt.md`
- Code (absence Manifesto) : `Manifesto/tests/apparatus_p4_t1_absence.rs`, `Manifesto/tests/apparatus_p2_t7_gate.rs`, `Manifesto/tests/apparatus_p3_t2_gate.rs`
- Code (mécanisme operator, Implemented) : `apparatus-operator/tests/apparatus_p4_t2_scaffold.rs` … `apparatus_p4_t12_gate_regression.rs` ; IT Kind `apparatus-p4-it` (T10–T11) ; chaîne M1–M6 `apparatus_m6_e2e_chain.rs`
- Wiki (faits de tests, pas L152) : `obsidian/AI FOR ALL/projects/manifesto/decisions/0008-apparatus-p4-k8s.md`
- Preuve d’implémentation : **Implemented** (T2–T12 + M1–M6 verts, A-DEC 2026-09-22). Dette hors-jalon : **D-TRANSIT-TCB**, **D-ADMB**, **D-PROD**.
