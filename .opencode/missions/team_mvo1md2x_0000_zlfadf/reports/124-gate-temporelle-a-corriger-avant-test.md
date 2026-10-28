# Corriger une limite du test avant d'exécuter la preuve

La revue indépendante a identifié un seul point bloquant dans la preuve122 : après le dernier comptage SQL, le test ne vérifie pas explicitement que l'observation a fini avant sa deadline. L'enveloppe timeout ne suffit pas à attester strictement l'heure de complétion d'une future déjà prête. Un résultat tardif pourrait donc être accepté comme dans le budget.

C'est un défaut de l'oracle du test, **pas un défaut de production ni un test réellement échoué**. Le contre-scénario de frontière est une inférence de revue, non exécutée. La production, le budget autorisé et les autres garanties ne changent pas.

Le briefing et son index sont persistés avant correction. Le lead route le findingT-1 au même test-engineer : ajouter un contrôle explicite après le dernier await et avant le succès, éventuellement conserver l'instant de complétion pour l'attestation après cleanup. Correction locale seulement, puis formatage/compilation sans exécution. Une vérification indépendante bornée du finding fermé suivra l'intégration ; pas de nouvelle campagne ou run de frontière.

L'unique exécution122 reste **non consommée** et bloquée à cette gate. Aucun délai étendu, nouvelle infrastructure, changementSDK ou preuve verticale/pub/pin autorisé. Les autres constats de la revue restent réutilisables si leurs sources ne changent pas.

Annexe : findingT-1MEDIUM/HIGHconfidence, `outbox_runtime_tests.rs:1075–1080,1093`, source0d50bb5d/patchbcbaba79. Brief `.cursor/review-briefings/20261027T1413Z-tests-component-post-expiry122-1f2dd30.md`. Correction dans le mandat122, sans nouvelle décision humaine requise ; aucun test additionnel n'est lancé pour reproduire la frontière, faute de budget autorisé pour un tel run.

Admission effective : `task_mvqrs8zf_000d_10753y` réaffectée nativement au même pool14/session/branche/arbre, sansfresh. BriefT-1 local fourni avec empreinte identique avant admission ; sourceauteur0d50 vérifiée inchangée. Confirmation d'admission transmise avant high/édition. La revue d'origine est terminée comme livrable, pas requalifiéePASS ; son consommateur de recheck n'est pas publié tant que le correctif exact n'est pas intégré.
