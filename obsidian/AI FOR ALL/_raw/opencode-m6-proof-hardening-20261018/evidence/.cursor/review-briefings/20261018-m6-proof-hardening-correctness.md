---
written_at: 2026-10-18T15:45Z
reviewer: correctness-reviewer
scope: m6-proof-hardening
head_sha: 04172a4
dirty: true
verdict: GO_WITH_COMMENTS-static-live-PENDING
---

# N6a — correctness indépendante, statique

## Delta final live N4/N5 — même reviewer, fd9c412

- Verdict reçu pool6 : **N4 PASS ; N5 PASS pour la preuve observable**, sans nouveau runtime ni modification par reviewer. Sources brutes des trois phases, runner, snapshots et diff relus directement ; 9/9 +1/1 +1/1, exit0, zéro skip/filtered, cible Windows E: démontrée.
- C1 LOW body BusyBox et PENDING-1/2/3 runtime fermés : H4 n'est émis qu'après HTTP404 avec body attendu, GNU check et classification du timeout borné inchangés, ainsi que PodAPI indépendant aux trois phases. Ce pass prouve les assertions exécutées, pas une capture autonome des octets du body/stderr non imprimés. Aucune causalité exclusive NetworkPolicy/DNS/dataplane Service déduite.
- H1–H4 réels cohérents : même catalogueH1/H2, enveloppe et CRI distincts, AdmissionRecord nommé par catalogue, hostname plugin lié aux32premiershex ; VALID/Scheduled,403/200,hops0/1,canary404/timeout. H1 préfixé par libtest sur la même ligne : extraction au marqueur puis fullmatch et unicité, log brut inchangé.
- Docker : fixtures créées puis arrêtées par ID, cleanup/witness sans erreur, node/volumes/réseaux préexistants conservés ; nouveaux volumes/images restent ask et ne sont pas supprimés.
- **Correction de portée par le lead** : le rendu initial qualifiait les deux objets Kubernetes remplacés d'« owned par nom dérivé » et concluait à leur remplacement autorisé/idempotent. Ces inférences **ne sont pas retenues** : nom/digest ≠ propriété, snapshots montrent Podplugin et AdmissionRecord préexistants supprimés/recréés (UID changés). Le PASS fonctionnel ne vaut ni autorisation ni préservation de l'état antérieur. Réserve sécurité distincte, clôture opérationnelle ouverte ; aucun cleanup supplémentaire engagé.
- G1 dossier tentative t6-170236Z vide : la cause préflight Unicode/CP1252 est déjà persistée au ledger ; pas d'exécution Cargo/Docker ni de log fabriqué. G2 fichier kubeconfig privé : credentials réels, exclu du manifeste allow-list, pas d'archive récursive ; suppression du seul fichier créé par mission après fin des consommateurs RO. G3 ancien PENDING QA rafraîchi après runvert.
- Dette strictClippym6 production différée par utilisateur toujours ROUGE ; T2/T4/G-A/G-B acceptés et résidus credentials historiques hors mission. Audit publication security et vérification infra séparés, pas implicitement PASS par ce verdict correctness.

## Revue statique initiale — provenance conservée

- Auteur : pool-6, fresh RO, tâche `task_mvdzohrt_001p_jloxg4`.
- Diff `0ca9b05..04172a4`,16fichiers (2463+/447-) relus intégralement, contrat §1–8 et plan QA amendé §5. HEAD04172a4 vérifié. Aucun Cargo/runtime/gitwrite ; userwork intact ; conclusions développeur non prises pour preuves.
- **GO_WITH_COMMENTS** :0bloqueur/HIGH,1LOW et4INFO. Dominante CROSS_FILE. Confiance distincte de sévérité ci-dessous.

## Findings persistés avant correction

### C-1 — LOW, confiance LOW, LOCAL — réponse404 réelle BusyBox à établir
- Sources : canary.rs:170,05-system-canary.yaml:22-24 et Dockerfile plugin.
- Positif exige status404 et body contenant «404 Not Found» ; canary busybox:1.36.1-uclibc, httpd8080, racine vide. Le body réel n'est pas établi par lecture seule ; si vide => faux ÉCHEC «canary positive HTTP unexpected response», jamais faux succès.
- **Ne rien affaiblir avant preuve**. Préflight N5 via forward dédié et requête autorisée après phaseIT ; ajustement oracle éventuel sous GO lead, conserver résultat observé sûr.
- Gap PENDING nommé, pas bloqueur statique.

### C-2 — INFO, confiance HIGH, CROSS_FILE — course port éphémère
- chain.rs:447-464 : bind0/drop puis kubectl forward ; tiers peut prendre le port. wait_bound30s détecte échec puis kill+wait => Err fail-loud, pas faux succès. Flake potentiel N5 ; protocoleSTOP, aucune action demandée.

### C-3 — INFO, confiance HIGH, CROSS_FILE — spawn-fail readyz immédiat
- ensure.rs:234-246 : échec spawn kubectl propagé immédiatement ; ancien code retentait jusqu'au deadline180s. Exitnonzero reste retenté jusqu'au délai borné. Délibéré, conforme fail-loud ; oracle Spawn fige trace at+1. Tracer, pas corriger.

### C-4 — INFO, confiance HIGH, CROSS_FILE — rejet cluster_name
- mod.rs:20-25 : panic «Kind target rejected» remplace défaut silencieux sur vide/invalide conformément contrat. Callers build/loadplugin, load/pinimage, mirrorzot, registryCRI : ensure a déjà validé même env via ensure::target_name. Aucune action demandée.

### C-5 — INFO, confiance HIGH, CROSS_FILE — stdout forward pipé
- mod.rs:190-202 et chain.rs:359-393/515-520 : stdoutnull→piped requis pour wait_bound ; faible sortie enfant, pas blocage statique identifié. Drop kill+wait toutes sorties/unwind. Deux forwards contrôlés ; plugin wait_tcp existant et h3hostnamebackstop, canary wait_bound/enfantmort. Aucune action demandée.

## Invariants vérifiés directement — do not redo sans delta

1. Garde avant effets du vrai ensure_with ;15refus trace et file_readsvide, nonUnicode distinct de None avec var_os ; catégories sans écho et padding normalisé.
2. Ordre argv exact : Dockerinfo→kindversionclient→getclusters→exportkubeconfig unique atomique/Drop→readyz→CNIignore-not-found distinguant API/RBAC/absence/présence→6waits inchangés600/300/600/300/300/180→SSA field-manager sansforce→CRD90. KUBECONFIGdédié partout, kubectlversion--client.
3. Tempsvirtuel readyz : deadline inclusif +pause2s, now dans(180000,182000], polls=sleeps+1, aucun apply.
4. ConflitSSA :1apply exact puis erreur sans CRDwait/retry/force, objets simulés inchangés (pas file de réponses).
5. Spawn/exit parcommande, erreurreadyzexit correctement non terminale ; tracefigée sans remédiation, diagnosticsclos.
6. PH16autoriséabsent via port,3commandes ; aucun re-run réel bogus. Overridefichier sansprobe, overrideabsentErr aprèsDockerseul.
7. Canary mêmeorchestration : ready→endpoints→positif404bodyPodexact→revalidate→wget→classify→revalidate→marqueur.9préconditions bloquentwget ; UID incluse dans comparaison cible et remplacement/disparition avant/après. Tout HTTP/=atteint=échec ;19diagnostics nonprobants ; seul nominalGNUexit4+marqueursexacts sansdoublons+ligne timeout IPobservée+ciblestable. GNU contrôlé script et image Debianbookworm-slim+aptwget ; execPodplugin, IPnumérique,--no-proxy,LC_ALL=C,tries1,wget5s,requesttimeout20s. Observation bornée seulement, pas certification globale DNS/Service/netpol/mesh.
8. Forwards possédés kill+wait à toutes sorties. H1–H4 schémas clos, valeurs observées, h3plugin-32hex conforme pod_name_for controller.rs:825-829, secretsformesrejetées sansécho, H4 aprèspréconditions et panicsassainies.
9. N2configeffective racine+crate et sentinelle tokio/net nonvacue ; reqwest/rcgenverbatim,21retraits+dérives ; TOMLsinglelinebornéfail-loud. Ajouts stagingm5/PS nécessaires et identiquesracine.
10. Docs etYAML référence seuls fidèles, subnet10.244/16canon ; m6 exactement1test ; zéro sourceproduction modifiée. DetteClippy27srcdifférée user, pas bloqueur comportement sansbugconcret.

## PENDING explicites et anti-goals

- PENDING-1 : C1body404BusyBox réel au préflightN5, direction fauxéchec seulement.
- PENDING-2 : ligne GNUwget réelle à N5 ; ne pas élargir diagnostic pour obtenirvert sanspreuve.
- PENDING-3 : IT/chaîneréelle/E2E/capturesH1–H4/auditfinal N4/N5 ; infra-verifier aprèsruntime. Aucune de ces preuves attestée ici.
- Ne pas refaire C3/C4/C5 pour style/préférence : comportements explicitement délibérés.
- Dette strictClippy et exception unique--no-verify sont deux décisions utilisateur distinctes déjà tracées, pas permissions générales de correctionprod/harness.
