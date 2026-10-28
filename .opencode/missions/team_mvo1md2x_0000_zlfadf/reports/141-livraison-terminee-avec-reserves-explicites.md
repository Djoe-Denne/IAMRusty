# 141. Livraison du périmètre terminée, réserves qualité explicites

## Ce qui est livré

La notification et son intention de publication sont durables dans la même
transaction. Le transport désactivé ne produit pas de faux succès. Les tâches
HTTP possédées, la reprise et les compatibilités corrigées sont intégrées.
Le produit conserve ses leases, retries et budgets d'arrêt : aucun unlock,
raccourcissement de lease ou drain nouveau n'a été ajouté pour obtenir un vert.

Le nettoyage et le plan révisé sont livrés. Les deux témoins complémentaires
sont conservés : réservation maintenue avant expiration, puis reprise après
expiration. La verticale finale prouve le vrai parcours SQL, SQS, handler,
OpenFGA, replay et accès HTTP, au nominal et après panne/reconstruction.

Le SDK exact testé et revu est publié sur la branche dédiée. Le pin et les tests/
documents sont enregistrés dans un commit application local. L'arbre application
et le SDK sont propres. Les30 tâches natives sont complétées, aucune tâche active.

## Vérifications et preuves réutilisées

- Composant post-expiration132 :1 PASS/0 échec,30,244s d'observations avant
  échéance54,990s, vraie nouvelle prise en charge et vrai message SQS.
- Revue indépendante134 : aucun défaut bloquant statique sur délai/ownership,
  source exacte, pas un résultat expérimental de toutes les branches d'erreur.
- Verticale finale139 :1 PASS/0 échec/0 ignoré/6 filtrés,25,39s, nominal et panne.
  Vraie corrélation/replay à ledger neuf/tuple unique,403 sans mutation et200 du
  destinataire. Résiduel0 dans ce run, observations266736µs sousD25s ; le cas de
  lease encore active reste la preuve distincte du composant, pas un fait inventé
  pour la verticale. La cause historique109 reste inconnue et son rouge conservé.
- Fmt ciblé et compilation verticale --no-run :0. Hook normal du commit
  `cargo fmt --all -- --check` :0, sans --no-verify ni formatage global automatique.
- Diff whitespace, hashes des tests, SDK propre, mode160000 et SHA distant
  recontrôlés après commit. Aucun Cargo actif.

Les tests et revues HTTP/transaction/NoOp acquis antérieurement sont réutilisés
avec leur portée, pas additionnés artificiellement en taux global de réussite.

## Ressources

Chaque campagne a son registre contemporain et son propre bilan. Campagne finale
verticale :4 créations/3 absentes/1 SMTP Exited(2) ASK/2 volumes opaques ASK,
0 possédée active ;15 conteneurs préexistants et138 volumes préexistants conservés.
Le lead a recoupé le lanceur avec un inventaire et un inspect frais. Les volumes
et SMTP de diagnostic restent conservés, sans prune ou suppression additionnelle.
Desktop/WSL restent actifs pour un workload étranger, pas pour une campagne
terminée ; RAM non mesurée. Le backend MCP résident n'a pas été arrêté.

Les worktrees et archives historiques sont préservés. Aucun cleanup destructif
de l'équipe ni commit forcé de ses anciennes instrumentations n'a été engagé.

## Réserves — ce que «terminé» ne signifie pas

Clippy strict antérieur reste non vert, Sonar/CI distante n'ont pas été exécutés
pour cette livraison. Pas de PASS qualité global sans dette, ni de tests forcés de
toutes les erreurs de startup, panics ou annulations externes. Drop n'est pas une
preuve de jointure asynchrone générale. Pas d'exactly-once, de fencing multi-worker,
de déduplication IAM entrante ou de backfill promis. ADR0313/0503 restent Proposed,
0301 Partial : aucun statut canonique n'a été promu automatiquement.

Le dépôt application n'a pas été poussé à distance par cette livraison ; son
commit est local. Le SDK, lui, est effectivement disponible à distance. Aucun
déploiement ou lancement de plateforme complète n'est revendiqué.

## Annexe de livraison et commandes réellement exécutées

- Commit application local : `39024b2b445ae9e35f9ad148ea754995f8baab18`.
- SDK publié et gitlink committé : `d60a88b2f2f499768a807941a1b4c80c9a366b3f`,
  branche `mission-telegraph-ac7-zlfadf`, vérifiée par ls-remote après publication
  et après commit. Push fast-forward sans force ; branche principale SDK inchangée.
- Fichiers du dernier commit : docs/contracts/telegraph-notification-outbox.md,
  docs/platform/events-outbox.md, les deux tests Telegraph et gitlink rustycog.
- Source verticale SHA80cce027, composant445f inchangés après commit.
- Commandes : rustfmt ciblé --check ; cargo test --locked -p telegraph-service
  --test notification_created_routing_test --no-run ; même cible avec filtre exact
  user_email_verified_routes_same_event_through_sentinel_and_real_authorization_after_publisher_recovery
  -- --exact --test-threads=1 ; git push du SHA SDK exact vers branche dédiée ;
  git commit des seuls cinq chemins de mission via hook normal ; git status,
  ls-tree, ls-remote, hashes et diff --check finals. Logs privés sous
  TI/.tmp/vertical-final134-inputs et TI/.tmp/vertical134-06bd9d7e772c4370.

Localisation : Telegraph outbox, propriétaire HTTP partagé et preuves ciblées.
Impact : correction intégrée, preuve fonctionnelle et SDK/pin livrés.
Limites : réserves ci-dessus, pas de publication application ou quitus global.
