# Tranche A — Builds Linux depuis l'arbre courant

2026-11-01. Base `master@f60910589e3867236c71beacb3ddbce447db74f7`, SDK
`93b890ff7b821e9cb2e7d0aeb81957d098778a2f` inchangé.

## Livré dans cette tranche

- Entrée commune `ops/deploy/build/images.py` : plan hors ligne, build Linux
  explicite, profil fonctionnel `dev-image`, réutilisation des images conformes.
- Snapshot public du working tree et du SDK consommé, fichiers nouveaux et
  Git-ignorés admissibles inclus ; empreintes par cible et dépendances locales.
  Sources privées, `.env`, clés et caches exclus avant lecture/hash.
- Catalogue exact packages/binaires/migrations, contexte de features stable ;
  un Cargo reçoit la sélection complète et décide des recompilations. Seules
  les images périmées sont emballées. Aucun `find` d'exécutables dans le cache.
- Cache target Linux historique conservé et exclusif ; journal content→mtime
  remplacé atomiquement/fsync **avant Cargo**, y compris s'il échoue ensuite.
  Dates relues, postérieures aux fingerprints conservés ; horloge/précision
  inadéquates refusées sans purge. Les 24 jobs et le target Windows E: restent
  inchangés ; aucun partage Windows/Linux du target.
- Recettes packaging-only ext-authz/controller, Cosign conservé ; anciennes
  recettes standalone maintenues. Provenance vérifiée pour tous les consumers
  Rust sélectionnés, pas seulement les trois frontières du lot précédent.
- Reçu `aiforall.image-build/v1` : **IDs Docker locaux**, pas digest OCI.
  Compatibilité kind-load explicite via `local-full-images.py --build-manifest`.

Usage et limites : [guide](../guides/linux-image-builds.md).
Architecture : [ADR-0607](../adr/0607-images-source-snapshot-registre-zot-hors-kind.md)
et [contrat](../contracts/images-local-release-registry.md), toujours **Proposed / Partial**.

## Arbitrages et revue

La revue initiale a bloqué le journal écrit après succès (B1), l'omission des
inputs Git-ignorés (B2), l'oracle de mutation transitoire (B3) et la qualification
release incomplète (R). B1/B2 sont corrigés. `--policy release`, `--revision`
et `--cold` sont explicitement refusés avant capture/Docker : export automatique
master/gitlink et pins complets restent à livrer. `--profile release` local ne
qualifie pas une livraison cloud.

**Décision utilisateur du 2026-11-01 : `snapshot-final-v1`.** Un aller-retour
A→B→A est acceptable si le snapshot compilé est A et que toutes les entrées
finales sont identiques A, SDK compris. Toute différence finale est refusée.
Pas de promesse de détection des transitoires ni de watcher/daemon ajouté.
Cette politique figure dans le plan, le reçu et les labels.

L'architecte a relu les corrections et autorisé la publication de **A locale
sous ces limites** ; aucun Accept global ni PASS Linux/OCI/Kind n'en découle.

## Preuves exécutées

- Plan hors ligne Hive/ext-authz : exit 0, politique de fraîcheur et contexte
  de features explicites, sans Docker.
- Batch Python ciblé : **52 PASS**, 29,186 s ; 53 cas chargés, l'oracle Cargo
  natif opt-in étant ignoré dans ce batch. Couvre source/SDK dirty, nouveaux
  inputs ignorés, filtres spécifiques, fichiers privés exclus, cache partiel,
  réutilisation d'image, refus de différence finale et régressions OpenBao/OAuth.
- Oracle Cargo **Windows natif : 1/1 PASS**, exécuté séparément en 2,473 s.
  Deux crates minimales sans dépendance tierce : A + asset build.rs A ⇒ binaire
  exécuté `A:A` ; B et asset B compilés, puis échec intentionnel du consommateur
  confirmé ; rollback A en restaurant les anciennes dates host ⇒ binaire
  exécuté `A:A`. La dépendance B fraîche et l'asset B sont confirmés par le JSON
  Cargo, pas seulement déduits d'un label.
- Oracle natif : 24 jobs, trois builds et une génération de lock séquentiels,
  target E: unique ; aucune modification Rust applicative/SDK. Preuves locales
  non versionnées : `.tmp/image-cache-tmphtoi3t7u/` (`proof.json`, logs A/B/A).
  SHA256 du helper effectivement testé :
  `0d1dd6fe96d3cd26275e1a49a1995e2e3d4c883a66b9f947e2333cbb5e1bb9f5`.
- Whitespace vérifié ; hook normal de publication `cargo fmt --all -- --check`
  conservé, sans contournement.

## Non qualifié / suite

**NOT RUN** : images Linux réelles, cache BuildKit/Cargo Linux, publication et
lecture OCI Zot, trust/pull Kind, rollout et E2E OpenBao/OAuth/données. Aucun
chiffre de vitesse globale n'est déduit du petit oracle natif.

B reste à implémenter : Zot applicatif indépendant, PKI distincte, Skopeo mTLS,
push puis relecture du digest OCI et reçu `aiforall.image-release/v1`. C raccorde
Kind via l'autorité leasev2 ; D fournit un refresh ciblé sans reprovisioning.
Le full deploy n'est pas présenté comme refresh rapide. L'export commit/cloud
reste une capacité distincte gated, pas un checkout propre promu en release.

Aucune ressource Docker/Kind/WSL créée ou démarrée dans A ; Zot/PV P4 inchangés.
Sources temporaires de l'oracle retirées par unittest, cache E: conservé.
Aucune branche/worktree de mission créée ; les 82 branches/5 worktrees
préexistants et l'UI Obsidian étrangère sont préservés. Pas d'équipe Ensemble
créée ; l'architecte ciblé a terminé. Le lot global reste ouvert après A.
