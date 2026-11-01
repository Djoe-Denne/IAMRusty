# Images locales/release — ADR-0607

Statut : Proposed. Réalité : Partial (tranche build locale codée ; Linux/BuildKit réel, Zot/pull Kind et livraison cloud non qualifiés). Voir le fichier ADR `docs/adr/0607-images-source-snapshot-registre-zot-hors-kind.md` et `docs/contracts/images-local-release-registry.md`.

- Direction humaine acquise : code courant/SDK dirty local, commit master exact pour future release, caches Linux conservés, Zot applicatif indépendant de Kind ; aucun Accept global.
- Arbitrage humain explicite 2026-11-01 : snapshot-final-v1 accepte A→B→A si compilation depuis A immuable et entrées finales byte-identiques A, SDK compris ; différence finale refusée. B3 clos, aucun watcher/daemon ni détection des transitoires prétendus.
- Correctifs B1/B2/R relus : journal mtime atomique/fsync avant Cargo avec floor fingerprints et refus clock/précision ; filesystem allowlist publique sans Gitignore/prune privé ; policy release/revision/cold refusées avant capture/Docker. Tranche A locale publiable sans bloqueur architectural restant identifié sur ce périmètre.
- Preuves parent non réexécutées : 52 Python offline PASS ; oracle Cargo natif Windows 1/1 PASS, B fraîche/build.rs/échec partiel/rollback mtime conservé → A:A. Pas de PASS Linux/BuildKit/OCI/E2E ni de changement Rust prod/SDK déduit.
- B reste à réaliser : build manifest aiforall.image-build/v1 + freshness_policy → Zot 2.1.8/mTLS/Skopeo → aiforall.image-release/v1 OCI réel. C trust/pull via leasev2 ; Zot/PV P4 préservés. Index/wiki/publication au parent, aucune supersession.