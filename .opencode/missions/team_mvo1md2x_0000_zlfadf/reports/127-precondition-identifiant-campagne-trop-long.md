# Précondition du lanceur rejetée, preuve de reprise toujours absente

## Ce qui s'est passé

La tentative de preuve après expiration122 s'arrête immédiatement à la création de la fixture : le message sûr est «122 fixture setup failed». Libtest rapporte0passé/1échoué/0ignoré/6filtrés et0,00s, Cargo101. Aucun marqueur de la preuve, aucun claim, aucune expiration ou publication n'est observé. **Ce n'est pas un rouge fonctionnel de production : la précondition de lancement est rejetée.** La preuve122 reste absente, pas réussie.

La cause est identifiée par les paramètres réellement exécutés et le chemin source : **le lead a choisi un préfixe donnant un identifiant de campagne de41caractères, alors que le harness impose40maximum**. Le contexte des fixtures est validé avant de créer OpenFGA, puis la DB et les autres services. Le rejet intervient donc avant toute création de fixture. L'erreur interne détaillée n'était pas imprimée, par le wrapper de test qui protège les traces ; elle n'est pas inventée comme log observé.

## Correction préparée, sans exécution

Le lanceur emploie maintenant un préfixe court, donnant31caractères. Une validation identique à la contrainte existante est ajoutée avant Cargo/création : identifiant non vide, au plus40caractères, uniquement caractères alphanumériques ou tiret. **La limite du SDK n'est pas modifiée.**

Vérifications statiques : ancien nom41rejeté, nouveau31accepté, nomvide/caractèreinterdit/41rejetés et40accepté ; syntaxeNode0. Source test445f729a inchangée, revue indépendante et compilation encore valides. Aucun test, runtime, compilation ou nouveau run effectué pour cette correction de paramètre.

Les logs, résumé et marqueur d'admission de la tentative échouée restent conservés. Une comparaison inverse en mémoire de la correction retrouve exactement l'empreinte du lanceur original ; aucune ancienne trace n'est réécrite et aucune permission n'est remise à zéro.

## Ressources vérifiées

Zéro conteneur créé, zéro ledger de fixture, zéro possédé actif et zéro Cargo. Les quatorze conteneurs préexistants gardent identités et états ; inventaires de volumes identiques. Le workload étranger actif reste préservé, sans arrêt global Docker/WSL ni suppression. Vérification indépendante et diagnostic de précondition persistés à côté du résumé.

## Décision demandée

**Recommandation : autoriser une seule exécution de remplacement du même test revu, avec le nom valide.** Ce n'est ni une répétition jusqu'au vert ni une nouvelle preuve/harness. La source445f, les oracles et les bornes restent strictement identiques : t0entrée de reconstruction,Rmesuré<=30s,Dfixet0+R+25<=55s,cleanup6s/phase61s,IAM25s inchangé. Nouveaupréflight/bail et marqueur distinct avant exécution. Le résultat fonctionnel peut encore échouer ; durée totale non garantie. Aucun autre run, verticale ou publication.

**Autre choix : conserver le blocage.** Aucun nouveau lancement. Les acquis117 et le correctif de lanceur sont conservés, mais la reprise après expiration reste non prouvée, publication et pin bloqués. Aucun défaut produit déduit de cette précondition et aucun risque accepté tacitement.

La première commande a bien été exécutée ; le lead n'en déduit pas une autorisation automatique de relancer. L'utilisateur décide explicitement du remplacement après cette explication.

## Annexe de preuve

- Tentative `component-post-expiry122-72aba6ce2cf643b2`, longueur41 ; fixture_runtime.rs173–177 rejette run.len()>40, avant ledger186–190.
- Chaîne : Telegraph/tests/common.rs118–125 → TestFixture::new SDKdatabase.rs264–322 (OpenFGA avant DB) → OpenFga::new → get_or_create_test_openfga_container548–565 → fixture_runtime::endpoint/context. Endpoint validé avant containerstart. Sources exécutées TI/SDKd60 lues, pas index seul.
- Campagne privéeTI/.tmp/component-post-expiry122-72aba6ce2cf643b2/ :summary.json,outbox_runtime_tests.log,before/after,volumesbefore/after,lead-cleanup-verification.json,precondition-diagnosis.json.
- Lanceur ancienSHA7edcd014cd7c2e3c4e61c1f81d3837475ff0d08757c6f3c997c53ad01c483a07 ; corrigéSHAa77dba1db77708532c4d381c5ab3ec2b8cca4bacceb788c0f541bdaed576554a. Delta : préfixe component-post-expiry122→post-expiry122 et garde de métadonnées avant fs.mkdir/Cargo. Inverse en mémoire=SHAoriginal vérifié.
- Aucun changementSDK/production/test/revue/garantie/ADR. Ce rejet de précondition ne constitue pas une tentative technique de correction produit et ne déclenche pas une escalade de débogage production.
- Localisation : paramètre runID du lanceur et validation du contextefixture. Impact : témoin non exercé. Limites : preuve après expiration toujours absente, aucun autre verdict métier révisé.
