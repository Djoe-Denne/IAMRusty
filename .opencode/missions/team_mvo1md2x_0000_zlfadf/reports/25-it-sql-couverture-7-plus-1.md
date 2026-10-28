# Rapport 25 — huit scénarios SQL couverts par deux runs

## Résultats recoupés par le lead

Run ciblé t2-fault-7c6186fd65f74005/cargo.log : running1test,
real_delivery_and_outbox_sql_faults_rollback_the_entire_attempt_then_recover OK ;
1passed/0failed/0ignored/7filteredout,3.91s. summary.json confirme cargo_exit0.
Le scénario conserve les fautes SQL delivery et outbox sur deux chemins,
catégories infrastructure/recoverable et deltas0/0/0 puis rétablissement.

Couverture obtenue : sept scénarios verts du premier run plus ce scénario
corrigé, pas un premier run8/8vert. La production et les sept autres tests sont
annoncés inchangés après correction du nom de table ; à contrôler au diff final.
Check/no-run/fmt5packages annoncés verts par auteur ; pas rejoués par principe.

## Ressources

Registre et summary du second run recoupés : container possédé
021b68b59f35a9ad550df4ebe1e4ff036b5c103260d553dd1d08680e573a484f,
created→stopped→reclassifiedyes→removedExited0, sansforce ni volume deletion.
owned_running[], issues[], before_count1/after_count1, seul node40201...running
préexistant intact. Fin du bail SQL ciblé, aucune fixture retenue pour T3.

Deux volumes opaques conservés ask :
-29d7354c0713da89eb516799e62681282c46a568a96b54797bfe5e504538400b ;
-07eacdcf2cdcaa608c97950679fa4ede6bfd941d5be9c730605221793a44f78c.
La seconde ligne observée utilisait Name comme id malgré création opaque :
correction de provenance append-only demandée (UNKNOWN+Name+ask), sans réécrire
l'historique ni demander de suppression. Rien n'autorise l'adoption par nom.

## Suite et limites

T2 prépare son commit normal sans exemption Rust, puis remise au lead pour
diff/provenances et intégration avant T3. Aucun GO T3 ni PASSglobal. AC7runtime,
preuve SQS/FGA/HTTP et revues finales restent à livrer. Aucun autre Cargo ou
runtime lancé par le lead. Aucun shutdownglobal du workload préexistant.
