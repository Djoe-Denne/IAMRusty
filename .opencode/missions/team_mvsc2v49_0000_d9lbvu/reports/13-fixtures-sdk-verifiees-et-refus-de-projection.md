# Fixtures SDK vérifiées, refus de projection à fermer

La mission poursuit la correction des frontières d'authentification avant les tests d'intégration et le navigateur réel.

Le complément SDK ne change que deux fichiers de tests. L'auteur rapporte19 tests RS256 réussis et le formatage SDK réussi ; la revue indépendante fraîche a confirmé que les assertions cryptographiques restent intactes et que la compatibilité historique n'est activée que dans son scénario dédié. Le lead a également lu le diff exact. La publication de ce nouveau commit test-only est retenue sur la même branche de mission, sans modifier le commit déjà publié ni la production SDK.

Un défaut de bout en bout a été identifié dans la projection des droits : le traducteur refuse bien les anciens événements sans provenance, mais son appelant peut avaler l'erreur et marquer l'événement terminé. Le refus visible n'est donc pas encore garanti. La correction est attribuée au même auteur : réutiliser l'échec/retry existant, sans nouvelle file ni écriture de droits. Ce point reste bloquant pour la livraison applicative.

La création d'un job de synchronisation ordinaire reste autorisée ; son exécution avec auto-invitation vérifie le propriétaire avant tout appel ou mutation. Aucun nouveau champ public ou persistant n'est introduit pour cette garde interne.

Le laboratoire poursuit ses configurations sur les valeurs d'autorité ratifiées. Aucun Docker, Kind, WSL ou véritable navigateur n'est démarré. Pas de clôture, de PASS global ou de GO production.

## Preuves et portée

- Nouveau SDK : `77672b8a305b5205db37c2b6ab80327f9a3cb16d`, parent72845d9, deux fichiers27+/4−. Test-only constaté par diff, snapshot lead propre. Résultats auteur distincts des preuves de revue ; pas d'exécution indépendante revendiquée.
- Revue native `msg_18463f51f001m0VXeID34hyX5d`, rapport [persisté](../../../../.cursor/review-briefings/20261028T2121Z-tests-sdk-rs256-fixtures-77672b8.md), aucune assertion retirée/skip ni production modifiée. Tâche acceptée après lecture et persistance.
- Module TLS remote exécuté par le lead après intégration IAM+SDK728 et import explicite :2PASS/0ignored,133 units hors filtre. Ce run ne prouve pas le comportement d'Envoy.
- `handler.rs:44–52,203–211` et trait Translator64–71 relus ; Err revendiqué doit suivre fail_delivery, Ok(None) inconnu conserve sa sémantique.
- Préconditions de bascule0314 persistées dans `.cursor/handoffs/20261028-authz-rollout-preconditions.md`, aucune requête SQL/FGA ou migration exécutée. L'outil de réconciliation Manifesto n'est pas un outil Hive.
- Publication du nouveau SDK pas encore exécutée à cette écriture ; son résultat sera ajouté après vérification distante. Gitlink canonique toujours72845d9.

Actualisation : publication **réussie en avance rapide728→776**, identifiant
distant vérifié égal au commit autorisé ; aucun force-push/réécriture/branche
protégée. Le pin canonique776 est intégré au2b0ea47. La configuration Serena
étrangère conserve la même empreinte ; elle n'est pas committée. L'auteur
applicatif a reçu l'autorisation de sélectionner ce seul nouveau pin publié.
