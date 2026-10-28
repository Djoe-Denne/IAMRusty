# Diagnostic de reprise prêt à être exécuté une fois

## Travail vérifié

Après l'enquête fresh à raisonnement high, le même test-engineer a été réaffecté nativement et a lui aussi confirmé high avant édition. Sa préparation est terminée : formatage et compilation sans exécution réussis, aucun test ou runtime lancé par l'auteur.

Le lead a lu l'intégralité du patch temporaire et vérifié les empreintes. Il ne modifie que le test vertical, dans l'arbre isolé de l'auteur. Les assertions métier et la correction temporelle restent présentes ; le sampler et les scénarios de replay et d'accès sont conservés. La fixture et le SDK restent inchangés.

Le test relèvera les états déjà lus avant arrêt, émettra les marqueurs d'arrêt/reconstruction, puis suivra les états pendant la même attente de publication de 25 secondes. Une seule lecture supplémentaire projette des champs SQL sûrs après l'arrêt ; elle avance en parallèle du même futur de restauration de la destination. La restauration gagne les simultanéités et ne doit pas attendre une observation encore en cours. La mesure peut donc être annulée : ce sera signalé comme inconclusif, pas comme un succès ou un nouvel échec métier.

La boucle observée conserve le retour, le poll100ms et le même message d'échec ; au timeout elle imprime son dernier échantillon en mémoire, sans nouvelle lecture au-delà de l'échéance. Pas de payload, JWT, erreur brute ou modèle complet dans les nouvelles traces. Aucun correctif de production, délai étendu ou nouvelle infrastructure.

## Préflight et étape autorisée

Préflight frais : 12 conteneurs préexistants, un seul étranger actif ; aucun adopté. Aucun Cargo, aucun réglage de jobs contradictoire ; configuration de 24 jobs et target E:. SDKd60 propre, AI1f2, source diagnostic exacte. Le lanceur existant de la campagne précédente est repris avec uniquement un nouveau nom de campagne ; toujours une seule cible verticale et registre historique TI, syntaxe contrôlée.

Le lead prend le bail pour **un seul diagnostic110 autorisé**. Les identités seront inventoriées de nouveau avant création, inscrites au registre et contrôlées après teardown, y compris sur échec. Seules les nouvelles fixtures possédées pourront être arrêtées gracieusement, avec perte de mémoire volatile. Ressources préexistantes et volumesASK préservés, aucun arrêt global Docker/WSL.

Un résultat vert ne suffira pas à expliquer l'échec109 ; une mesure manquante reste inconclusive. Aucune relance automatique, correction de production, publication ou pin autorisé. Le source temporaire n'est pas intégré au produit : la version corrigée/revue6ac7 reste dans l'arbre du lead.

## Annexe technique

- Source exécutée : pool14/services/Telegraph/tests/notification_created_routing_test.rs, SHA256 `5f02f067f00c08a285b9cd7e32001af3c3c95ad5800a9464416cced00a8bc9f6`.
- Patch depuis correction6ac7 : `511a9d0e4bb0a54d338d30a9664b83cdabf72f417fc0cb8f757f4ca9c8d5c3ea` ; depuisAI1f2 : `09d3cd055db48709b2c7e466f891a1d91ed1a2c438f70e6439b605cff4eaef1f`. Tous recalculés par lead. Fixture identique àTI, SDKpropre.
- Neuf artefacts source/patches/manifeste/fmt/no-run/bail archivésTI/.tmp/recovery-diagnostic110-approved/, copies aux empreintes identiques. no-run status0,31,42s ; aucun test exécuté à cette étape. Diffcheck0.
- Lanceur pool14/.tmp/recovery-diagnostic110-inputs/run-recovery-diagnostic110.cjs, SHA256 `0d74c2a5eca52a75a364bf5dd82575ae2eeb436d4a398dee1f1987ed4b6a0bf7`, syntaxe0 ; seul préfixe diffère du lanceur unique105 inspecté.
- Commande attendue : `cargo test --locked -p telegraph-service --test notification_created_routing_test user_email_verified_routes_same_event_through_sentinel_and_real_authorization_after_publisher_recovery -- --exact --test-threads=1`.
- Localisation : helpers privés du seul test et lectures autourstop/rebuild. Impact : instrumentation passive temporaire. Limites : I/O synchrone et lecture concurrente peuvent modifier l'ordonnancement ; causalité à interpréter suivant le plan112, pas déduite d'un seul vert.
