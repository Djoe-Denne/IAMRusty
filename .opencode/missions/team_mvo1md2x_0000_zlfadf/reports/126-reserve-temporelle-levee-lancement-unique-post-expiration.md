# Réserve temporelle levée, lancement de la preuve après expiration

Le même reviewer indépendant conclut au **PASS borné du recheck** : le pointT-1 est résolu sur la source corrigée. La date de fin est prise après le dernier await, contrôlée strictement avant le succès, puis transportée sans recalcul après cleanup. Les cas de fin exactement àD ou aprèsD sont rejetés par le code. Ce sont des constats source et des contre-scénarios inférés, pas des tests de frontière exécutés.

Le lead a persisté la revue et l'index. La source exacte, le formatage et le diffcheck sont recontrôlés avec succès. SDKd60 propre, AI1f2, aucun Cargo concurrent,24jobs configurés/targetE:/aucun réglage contradictoire. Docker déjà actif ; inventaire frais de quatorze conteneurs préexistants, un seul étranger actif, aucun adopté.

Le lead lance **l'unique preuve composant122 autorisée** : la reconstruction et toutes observations partagent D=t0+R+25s, avecR mesuré au plus30s, donc55s maximum depuis entrée en reconstruction. Aucun reset ; nettoyage des propriétaires sous6s/phase au plus61s. Les clausesIAM25s et le produit sont inchangés.

Le lanceur existant ne sélectionne que ce nouveau filtre, recapture l'inventaire avant création, vérifie les images déjà présentes, inscrit les identités et termine le cleanup même en échec. Seules les nouvelles fixtures possédées pourront être arrêtées gracieusement, avec perte de leur mémoire volatile. Les anciennes ressources/volumesASK et le workload étranger restent préservés, sans arrêt global Docker/WSL.

Pas de test de frontière, nouveau117, verticale, suite globale, correctif de production ou publication. Un futur PASS122 ne couvrira que la brancheAC3 contrôlée, sans reconstituer109 ni valider droits/replay/wholeapp. Aucune relance automatique si le run échoue.

## Annexe

- Recheck persisté `.cursor/review-briefings/20261027T1423Z-tests-post-expiry122-t1-recheck-1f2dd30.md` +INDEX ; tâchetask_mvqryt8t_000f_7cwn31 livrée.
- SourceSHA `445f729af1b350dc0e702cb3aa0fbec9715bbe1c2a8e7ae793d309a99fffd1ea`, delta20961fe9, fullpatchdc5b2c7c, source identique à la compilation10,60s sans exécution.
- LanceurTI/.tmp/component-post-expiry122-run-inputs/run-component122.cjs, SHA `7edcd014cd7c2e3c4e61c1f81d3837475ff0d08757c6f3c997c53ad01c483a07`, syntaxe0 ; seuls préfixe et filtre diffèrent du lanceur117 vérifié.
- Commande : `cargo test --locked -p telegraph-service --test outbox_runtime_tests committed_claim_recovers_same_event_after_measured_expiry_with_fixed_rebuild_deadline -- --exact --test-threads=1`.
- Admission unique marquée avant lancement ; bail leadCargo+fixtures jusqu'à sortie puis cleanup. Aucun autre consommateur de ce bail. Aucun runtime redémarré au préflight.
