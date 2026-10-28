# Rapport 51 — T4 bloque HTTP en vol, T5 prête mais non intégrée

## Retours distincts

T5 a rédigé/stagé deux documents (+51/−1), diffcheck annoncé réussi ; commit normal unique échoue Windows206, sans bypass/retry. Sources et index conservés, aucune nouvelle preuvefonctionnelle/runtime. La méthode checkout court connue résout la mécanique, mais la photographie finale devra tenir compte de l'issueT4 ; pas d'intégration ni complétionT5 prématurée.

T4 a levé le faux blocage de manifeste SDK et examiné l'interaction réelle axum0.8.9. Nouveau finding statique bloquant transmis : joindre/annuler le serveur parent ne suffit pas à joindre/annuler un handler async en cours dans une tâche de connexion détachée. La borne6s et l'absence d'orphelinHTTP peuvent être violées malgré drain des parents réussi. Paquet final en préparation, pas témoin exécuté ni cause de production reproduite à déclarer.

## Persistance et décision

Finding préliminaire persisté avant correction : `.cursor/review-briefings/20261026-correctness-http-inflight-aa400f3.md`, lié au snapshotfigé. Le lead ne réduit pas les garanties du contrat97/115/117. Consulter architecte sur surface minimale et test discriminant ; si SDK/publicAPI/harness supplémentaire requis, revenir à la frontière de périmètre avant implémentation. Aucun patch de production autorisé par cette note.

T3 reste auteur de continuation pour la correction si elle est autorisée ; T4 reste le même reviewer indépendant. Pas de nouveau pipeline global, test/runtime ou quarto essai historique. Les preuves vertes acquises restent dans leur portée et ne ferment pas ce finding. Dernières ressources possédées connues arrêtées, node non possédé préservé ; aucun nettoyage global.
