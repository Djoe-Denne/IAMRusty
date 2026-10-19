# Review briefing — P6a correctness (chaine m6, diff 43c3804..e66b736)

    reviewer: correctness
    written_at: 2026-10-18T10:25Z
    branch: master
    head_sha: e7e6630 (docs) ; diff code revu : 43c3804..e66b736 (HEAD code e66b736)
    dirty: false
    scope_slug: m6-e2e-chain
    verdict: GO_WITH_COMMENTS
    related_paths:
      - workers/apparatus-operator/tests/fixtures/kind/mod.rs
      - workers/apparatus-operator/tests/fixtures/kind/cluster.yaml
      - workers/apparatus-operator/tests/fixtures/chain.rs
      - crates/apparatus-reference-kv/Dockerfile.http
      - workers/apparatus-operator/tests/apparatus_m5_invoke_isolated.rs
      - ops/deploy/apps/overlays/kind-demo-monolith/schedule-reference-kv.ps1
      - ops/deploy/README.md
      - ops/deploy/kind/cluster.yaml

## Verdict

Le diff fait exactement ce qu'il annonce : re-ciblage borne non destructif (D-1/D-2), kubeconfig derive du point unique (D-4), apply server-side (D-6), fix chain.rs, revert Dockerfile.http — prouve en runtime par run4 (1 passed / 0 failed, 311.79 s) plus negatif bogus-cluster fail-loud. Aucun BLOCKER/HIGH ; deux MEDIUM preexistants (dormants, hors GO), deux ecarts documentaires. Les findings sont des lots futurs candidats, pas des corrections de ce lot.

## Findings

### C-1 — MEDIUM (defaut preexistant, dormant)

- Location: workers/apparatus-operator/tests/apparatus_m5_invoke_isolated.rs:168-177 (manifeste staging m5) et ops/deploy/apps/overlays/kind-demo-monolith/schedule-reference-kv.ps1:124-133 (New-M5HttpStaging).
- What: les stagings m5 et ps1 n'ont ni reqwest ni rcgen dans leur bloc [workspace.dependencies], alors que crates/apparatus-reference-kv/Cargo.toml:24-25 herite reqwest = { workspace = true, optional = true } / rcgen = { workspace = true, optional = true }, et que Dockerfile.http:8 construit --features http (binaire reference-kv-http exige required-features http, Cargo.toml:16).
- Expected: les 3 producteurs de staging (chain.rs, m5, ps1) fournissent un workspace embarque ou toutes les deps heritees de la crate sous feature http sont presentes.
- Observed: seul chain.rs:202-203 (E-14) a les 2 entrees ; m5 et ps1 les manquent — le build staging echoue au stage cargo avec le meme parse error prouve par run3 sur m6.
- Why: tout run futur de m5 (Kind-defer G-17.10) ou du gold path 0605 (GO separee) serait ROUGE au stage build meme apres le revert Dockerfile. La justification ledger du E-13 « restaure le contrat declare pour TOUS les consommateurs » est incomplete : vraie uniquement pour la couche COPY. Dormant en plus d'une porte : m5 appelle ensure() (:452) avant le build staging (:456).
- Classification: preexistant — aucun des 2 fichiers est dans le diff 43c3804..e66b736.
- Suggested correction: futur petit lot test-only : ajouter verbatim reqwest = { version = 0.12.0, features = json + rustls-tls } et rcgen = { version = 0.13.2, features = x509-parser } aux deux blocs (pattern chain.rs:202-203), ou factoriser le manifeste staging en constante partagee pour eviter la triple derive.
- How to validate: inspection statique des 2 manifestes (grep reqwest/rcgen) ; jour J (GO dediee) : docker build d'un staging assemble comme m5, exit 0 attendu.

### C-2 — MEDIUM (preexistant, expose par le re-ciblage)

- Location: workers/apparatus-operator/tests/fixtures/kind/mod.rs:1046-1051 (bin_ok) + 1039-1044 (resolve_kubectl).
- What: bin_ok(kubectl) execute kubectl version SANS --client, ce qui contacte l'API serveur du contexte du kubeconfig PAR DEFAUT (pas le kubeconfig exporte par le fixture). Kubeconfig par defaut perime (port re-mappe apres restart Docker Desktop) -> exit 1 -> « kubectl not found in PATH » : diagnostic faux.
- Expected: la resolution de binaire teste la disponibilite client (kubectl version --client), jamais la joignabilite serveur.
- Observed: le probe serveur est traite comme un test de presence binaire ; run1 ROUGE (183 s) avec cause environnementale rapportee comme absence de binaire.
- Why: tout re-run dans un shell frais sans KUBECONFIG pre-exporte reproduit le faux « not found » sur le cluster partage (ports variables) ; le re-ciblage rend ce chemin plus expose qu'avant (cluster fixture dedie stable). Les kubectl internes au fixture sont immunises (.env KUBECONFIG = &self.kubeconfig :394/:411) — seul le probe de resolution est vulnerable.
- Classification: preexistant (fonction hors du diff, correctement notee « hors lot » au triage P5) — pas une regression du diff.
- Suggested correction: lot fixture 1 ligne : probe dedie kubectl_ok() avec version --client dans resolve_kubectl ; laisser bin_ok(kind) tel quel (kind version est client-only, correct).
- How to validate: kubectl version --client exit 0 avec kubeconfig perime ; puis run m6 (GO) en shell frais sans KUBECONFIG -> ensure() franchit le resolve.

### C-3 — LOW (introduit par le lot ; fichier mort devenu trompeur)

- Location: workers/apparatus-operator/tests/fixtures/kind/cluster.yaml (name aiforall-local, podSubnet 192.168.0.0/16).
- What: fichier devenu mort (unique consommateur create_cluster supprime par le diff ; grep repo : zero reference code, seulement docs/plan QA et ADR-0603:96) mais renomme vers le nom du cluster partage reel en conservant podSubnet 192.168.0.0/16, divergent du canon ops/deploy/kind/cluster.yaml:11 (10.244.0.0/16) pour le MEME nom.
- Expected: deux descriptions du meme cluster nomme doivent concorder, ou le fichier mort doit disparaitre.
- Observed: collision de nom avec divergence de subnet — un usage futur (manuel/agent) pour recreer aiforall-local produirait un cluster non conforme (subnet different du canon, semantique host.docker.internal differente, aucune install Calico dans le fixture pour rattraper : ensure() fail-loud ensuite).
- Why: invite a la confusion exactement pour l'operation la plus risquee (creation du cluster partage).
- Suggested correction: supprimer le fichier mort (choix propre), ou l'aligner : podSubnet 10.244.0.0/16 + commentaire « fichier de reference ; la creation du cluster est une operation runtime (deploy-j2.ps1), jamais un effet de test ».
- How to validate: grep cluster.yaml sous workers/apparatus-operator = 0 apres suppression.

### C-4 — LOW (ecart documentaire introduit par le lot)

- Location: ops/deploy/README.md:52,55 + ops/deploy/kind/cluster.yaml:5.
- What: « Cluster IT apparatus-p4-it != aiforall-local (ne pas retarget) », « Fixture ... Intouchable ; Calico + nom apparatus-p4-it », « tests/fixtures/kind/* intouchable » — contredits par l'etat commite (retarget borne autorise par resolution de mission, nom derive cluster_name(), Calico jamais installe par le fixture).
- Why: un futur agent lisant « intouchable » pourrait s'y bloquer ou « restaurer » le comportement destructif supprime. e7e6630 n'a amende que le plan QA (+3 lignes) ; ces 3 sites restent pre-mission sans la carve-out mission (ledger 20261017-g17 :44-49, plan QA section 0).
- Suggested correction: mise a jour des 2 sites README + commentaire cluster.yaml:5 : fixture cible par defaut aiforall-local (override APPARATUS_KIND_CLUSTER), ensure reutilise sinon fail-loud, jamais delete/create/install-Calico.
- How to validate: lecture des 3 sites amendes.

### C-5 — INFO

- Location: mod.rs:19-24 (cluster_name).
- What: le filtre teste !name.trim().is_empty() mais retourne la valeur brute — APPARATUS_KIND_CLUSTER avec espaces de padding produit un nom avec espaces -> mismatch kind get clusters -> Err « cluster absent » (fail-loud, non destructif).
- Suggested correction: retourner name.trim().to_owned() au prochain passage sur ce code.

### C-6 — INFO (protection procedurale, pas appliquee)

- Location: mod.rs:12-24 + doc-comment :16-17.
- What: l'override env n'est jamais filtre contre les clusters proteges (kind-apparatus-p4-it, rancher-desktop) ; la protection est procedurale (doc + GO), pas appliquee en code.
- Why: consequence bornee par le nouvel ensure() non destructif : pire cas = apply server-side additif + kind load + lectures ; aucun chemin delete/create/install ne survit (grep 0). rancher-desktop n'est pas un cluster kind -> cluster_exists false -> Err. Durcissement optionnel futur : rejeter explicitement les noms proteges dans cluster_name().

## Reasoning (deja tranche — ne pas re-explorer)

- ensure() non destructif verifie (mod.rs:51-75) : Docker -> resolve kind -> cluster_exists (absent -> Err explicite, jamais create) -> export kubeconfig -> wait_api_server -> assert_cni_ready (kindnet -> Err ; calico-node absent -> Err explicite « operation runtime deploy-j2.ps1 » ; waits CNI avec re-check kindnet final :279-284) -> apply_operator_manifests SSA --field-manager=apparatus-it-fixtures + wait CRD 90 s. Zero delete_cluster/create_cluster/install-Calico/FELIX_* restant (grep : 0 hors doc-comment :17).
- Point de nommage unique verifie : cluster_name() derive aux 8 sites (ensure :58, load image :307, load_and_pin :428, mirror :449, cri-absent :500, eprintlns :196/:206, temp kubeconfig :1070, export :1072). Temp kubeconfig = {name}.kubeconfig (D-4 ok) ; l'ancien apparatus-p4-it.kubeconfig en TEMP ni lu ni supprime (hors perimetre D-4 assume).
- Rupture d'API : aucune. La pub const CLUSTER_NAME supprimee n'a zero consommateur (grep tests/ + repo-wide KindCluster/fixtures::kind : seuls m3/m4/m5/t10/t11/m6, tous via ensure() signature inchangee). Les items supprimes (delete_cluster, create_cluster, install_calico_and_wait, set_calico_kind_env, CALICO_MANIFEST_URL) sont tous prives. Les 15 Kind-defer ne perdent que le comportement delete/recreate — voulu (D-2/D-7, resolution mission).
- Noeud {name}-control-plane coherent kind (1 control-plane) ; prouve runtime : aiforall-local-control-plane Ready (recovery :25).
- chain.rs verifie : repo_root() = ../.. depuis CARGO_MANIFEST_DIR workers/apparatus-operator = racine repo ; reference_kv_dir() = crates/apparatus-reference-kv ; staging deps reqwest/rcgen verbatim identiques au root (Cargo.toml:88/:142) ; staging layout top-level (staging/apparatus-contracts, staging/apparatus-reference-kv) compatible Dockerfile.http:16 COPY apparatus-reference-kv/apparatus.toml — prouve run4 vert.
- SSA sans --force-conflicts : conflit = erreur = fail-loud, pas d'ecrasement silencieux des champs d'autres managers (J2/J3) ; runtime : labels J3 intacts, ns crees avec labels fixture (recovery :21-32). daemonset_exists reste fail-loud sur erreur non « not found ».
- m6 inchange : apparatus_m6_e2e_chain.rs n'est pas dans le diff (assertions conservees — E-3 ok).
- Messages « forbidden for T11 » (kindnet :188/:281) : cosmetique — m6 depend aussi de la netpol pour l'oracle canary ; non compte comme finding.

## Do not redo

- Ne pas re-prover le mecanisme workspace-inheritance reqwest/rcgen (prouve run3 ROUGE + run4 vert).
- Ne pas re-grepper les 16 sites ensure() (inventaire plan QA section 1.2 ; signatures verifiees inchangees ici).
- Ne pas re-executer le negatif bogus-cluster (fait : FAILED 0,86 s, Err explicite, kind get clusters inchange — ledger :369-371).
- Ne pas rejuger run4 : « test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 311.79s » verifie lead (recovery :9).
- Ne pas re-auditer wait_api_server/wait_kubectl/daemonset_exists (preexistants inchanges, fail-loud).
- Ne pas relire ADR-0008/0603/0604 pour ce lot : le nouveau contexte de preuve est documente (plan QA section 1.5) ; pas de promotion ADR demandee.

## Anti-goals

- Ne jamais re-introduire delete/create/install-Calico dans le fixture (barriere D-7, clusters proteges et partage).
- Ne pas « miroiter » reqwest/rcgen cote test m6 pour contourner les stagings m5/ps1 (refus worker valide lead — masquerait la derive).
- Ne pas changer le probe kind en durcissant kubectl (kind version est client-only, correct).
- Ne pas toucher INDEX.md / plan / ledger (reserves au lead) ; pas de .cargo/config.toml, pas de gitlink rustycog.
- Aucune correction C-x dans ce lot : lots futurs candidats, arbitrage lead.

## Open questions

Aucun bloquant. Consolidation des manifestes staging (C-1) et durcissement kubectl (C-2) = petits lots futurs a arbitrer.

## Tests

- Executes (preuves datees, non re-lances) : run4 = cargo test --locked -p apparatus-operator --features admit,controller --test apparatus_m6_e2e_chain -- --test-threads=1 -> 1/1, 311.79 s (log worktree preserve, recovery :7-9) ; negatif bogus-cluster -> Err explicite sans mutation cluster.
- A executer pour valider les fixes futurs : cf. chaque finding (C-1/C-2/C-3).

## Staleness

Lie a e66b736 (code) / e7e6630 (docs, arbre propre). Si HEAD bouge, reverifier les fichier:ligne (surtout mod.rs, volatile).
