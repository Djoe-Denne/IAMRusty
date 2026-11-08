# Reconstruction locale bornée pour OAuthConnect

Version : 1 — 2026-11-08. Décision : [ADR-0608](../adr/0608-recreer-local-full-pour-oauthconnect.md), **Proposed / Unimplemented** à la rédaction. Autorisation opérationnelle acquise ; aucun résultat live implicite.

## Périmètre et invariants

- Seul cluster `aiforall-local-full` / contexte `kind-aiforall-local-full` : un control-plane, deux workers. Perte DB/PVC/coffre de ce lab acceptée, **sans migration/restauration obligatoire** ; aucune promesse de récupération des données détruites.
- Préserver legacy/autres clusters, PostgreSQL opaque/volumes inconnus, registre OCI externe, images/caches/builder, credentials et PKI privés. Aucun prune global, force SSA ou skip TLS. Propriété inconnue ⇒ préserver et STOP si elle empêche l'opération.
- Réutiliser les helpers Ops existants : `ops/deploy/bootstrap-local-full.ps1`, `local_full_target.py`, `local-full-images.py`, `deploy-local-full.py` et lifecycle. **Pas de nouveau pipeline, port/API applicative, mode TLS/réseau ou chantier production.** L'architecte n'édite aucune de ces sources.

## Séquence opérateur minimale

1. **Avant destruction — parent.** Relire le ledger et les registres Docker pertinents ; fixer l'inventaire privé des IDs exacts des trois nœuds, volumes/montages et baux. Recouper le bail v2 avec l'identité réelle ; vérifier l'absence de workload/bail étranger incompatible. Attester séparément que credentials, bundle PKI complet, manifeste de build et neuf images/artefact sont disponibles hors des ressources à supprimer. Ne jamais journaliser de secret.
2. **Destruction ciblée.** Supprimer uniquement le full dont l'identité est vérifiée et ses ressources de stockage attestées ; préserver les volumes inconnus et tous les éléments hors périmètre. Conserver les préimages/diagnostics privés et rendre l'ancien bail non réutilisable comme autorité active. Ne pas lancer de réconciliation/force-conflicts préalable.
3. **Installation neuve.** Recréer la topologie 1+2 avec les paramètres full existants. Créer nouveau bail v2, kubeconfig fixe privé et registration lifecycle distincte. Vérifier mapping/endpoint avant GET, puis nouveau `kube-system` UID avant mutations ; ne pas retargeter l'ancien kubeconfig ni toucher au contexte global.
4. **Images et PKI.** Vérifier la provenance du build déjà acquis contre les entrées effectives courantes ; changement pertinent ⇒ STOP et décision ciblée, pas réemploi silencieux. Recharger explicitement les neuf images sur les nouveaux nœuds via le parcours `--build-manifest` existant et conserver son nouveau reçu. Réutiliser le bundle PKI complet courant et son extension OAuthConnect validée, sans rotation ; vérifier CA/SAN/validité et cohérence des références. Les anciens Secrets/montages ne sont pas une entrée de reconstruction.
5. **Activer.** Premier `deploy --oauth-connect` avec nouveau bail, reçu d'images, PKI courante et fichiers privés GitHub + simulated ; **sans `--preserve-platform-tls`, ancien profil OAuth ni nouveau mode réseau de compatibilité**. Ne pas contourner les gates existantes. Réenregistrer explicitement les deux sources privées pour le lifecycle du nouveau lab ; pas de copie de credentials dans Git/logs.
6. **Vérifier et rendre.** Inventaire avant/après, reçus et résultats bornés dans le ledger parent. Respecter le keepalive demandé pour le nouveau lab ; ne pas arrêter/supprimer les ressources étrangères. Chaque ressource Docker créée est enregistrée à la création selon le registre du dépôt.

## Validation proportionnée — sans nouveau chantier

- Réutiliser les preuves acquises au même contenu ; si les helpers sont modifiés, rejouer seulement leurs tests Ops pertinents, notamment `ops/tests/test_oauthconnect_ops.py`, et les tests des gardes effectivement touchées. Aucun « tout cargo » ou build répétitif pour une modification documentaire/Ops seule.
- Preuves live attendues : topologie 1+2, nouvel ancrage valide, images correspondant au reçu ; foundation prête et déploiements OAuthConnect + simulateur présents/prêts ; IAM configuré `github` + `simulated`, secrets seulement référencés et accès TLS vérifié. Pas de connecteurs legacy ajoutés au profil neuf ; cluster legacy extérieur inchangé.
- Distinguer installation/readiness, entrée HTTPS et parcours navigateur OAuth. Un parcours réel GitHub ou simulé non exécuté reste **NOT RUN**, jamais PASS déduit des Pods. Utiliser les checks existants, sans inventer une campagne supplémentaire préalable à l'activation.
- Comparer l'inventaire hors scope et les empreintes PKI pour vérifier leur préservation ; absence de fuite de credentials et absence de suppression étrangère. En cas d'échec : pas de boucle d'apply, pas de forçage ; conserver les preuves et annoncer activation incomplète avec le blocage précis.

## Dette / disposition

La reconstruction n'établit pas une procédure de convergence prod, upgrade avec données, HA/cloud, GitLab ou reprise à expiration Docker OFF. Les conflits SSA et dérives manuelles de l'ancien lab deviennent des constats historiques, pas des corrections démontrées. Le parent consigne ce qu'il garde/remplace/écarte parmi les correctifs testés, sans rollback global ni exécution de `recover_apply_receipts.py` ou du nouveau mode réseau pour cette opération.

Suivi d'exécution : [ledger parent](../../.cursor/handoffs/20261108-ops-oauthconnect-reprise.md). Aucun runtime, test ou cleanup exécuté par ce livrable documentaire.
