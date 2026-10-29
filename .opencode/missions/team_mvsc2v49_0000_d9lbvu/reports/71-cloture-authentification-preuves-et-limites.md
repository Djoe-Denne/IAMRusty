# Clôture : frontières d'authentification corrigées et preuve locale qualifiée

La mission d'authentification est terminée dans son périmètre approuvé. Les revues
indépendantes ne laissent aucun défaut de sécurité bloquant connu dans le diff
examiné. Cela ne constitue pas une qualification globale de production.

## Résultats

- L'identité conserve son émetteur avec l'identifiant utilisateur jusqu'aux
  permissions, caches et projections concernées ; pas de repli vers un UUID nu.
- Le signataire distant refuse les transports non autorisés, redirections et
  réponses hors borne, sans secours vers une autre clé ou PEM.
- Envoy accepte le bon certificat, refuse le mauvais nom signé par la même
  autorité, puis retrouve le certificat correct :303→503→303, refus de nom corrélé.
- Le défaut OAuth après inscription venait d'un jeton intermédiaire construit
  avec l'ancienne autorité. Le constructeur reçoit désormais l'autorité et
  l'audience configurées dans ses deux assemblages de production. Les contrôles
  de signature et du registre restent fermés ; le double mint préexistant n'est
  ni supprimé ni utilisé comme secours si le jeton final échoue.
- Chrome réel, contexte neuf : connexion réussie, réassociation par callback
  exact sans Bearer, trois chemins sans authentification refusés403. Compteurs
  du même run corrélés et oracles revus indépendamment.

Les tests antérieurs rataient la succession inscription terminée→nouvelle
connexion : ils vérifiaient l'inscription seule ou construisaient directement
des claims avec la bonne autorité. Une régression avec vrai codec puis une IT
de second login ferment ce trou, sans assouplir les assertions.

## Validation réellement exécutée pour le dernier correctif

- `cargo test --locked -p iam-domain --lib service::token_service` :19PASS,
  zéroignoré,34autresfiltrés.
- `cargo test --locked -p iam-infra --test rs256_jwks_roundtrip` :3PASS,
  zéroignoré/filtré ; signature et mauvaise audience vérifiées.
- `cargo check --locked -p iam-service` et `cargo fmt --all -- --check` :exit0.
- Sélection PS/AST du lanceur : rouge avant ajout, PASS après ; aucun Cargo ou
  Docker réel dans ce gate. Lanceur/registre/cleanup opérationnels inchangés.
- Trois suites IT par le lanceur existant : OAuth9, transactionsPostgreSQL6,
  cleanup2 :**17PASS, zérofailed/ignored/filtered**. Windows natif, un Cargo,
 24jobs/E:, RUN_ENV=test, sans Kind pour ces IT.
- Construction Linux de livraison IAM/migration puis image IAM :deuxexit0.
  Déploiement limité à IAM/inittrust, puis parcours Chrome et corrélation réussis.
- Revues sécurité core/sélection/oracles et infra documentaire bornée : aucun
  nouvel écart bloquant. Documentation/liens/diff contrôlés ; preuves inchangées
  réutilisées plutôt que relancer toute la liste de référence.

La reproduction préalable1PASS prouve le défaut, pas une correction. La simple
compilation de la nouvelle IT n'est pas comptée comme son exécution. Les autres
commandes de référence non sélectionnées n'obtiennent aucun PASS implicite.

## Code, décisions et documentation

Application exécutable7b8c1f95 ; SDK02e5 publié sur la branche de mission canonique,
sans push application.8606173 ajoute seulement sélectionQA/docs ; arbre applicatif,
SDK et Cargo identique à l'image testée. Les empreintes revues sont conservées.

ADR0412 : décisions Accepted inchangées, RéalitéImplemented pour ce laboratoire
après proposition de l'architecte lue puis ratification explicite du lead sous
délégation utilisateur. Historique Partial et échec401 conservés. Digest Serena,
contrats OAuth et handbooks réconciliés ; aucun autre état ADR promu par transitivité.

## Nettoyage et limites

Trois fixtures créées/retirées. Les trois nœuds Kind possédés sont arrêtés
gracieusement, sans suppression. Les trois autres conteneurs et leurs états sont
préservés, dont le legacy actif. Chrome/forward fermés ; aucun runtime possédé
encore actif. Images/caches/volumes et worktrees étrangers ou sales conservés.
DockerDesktop/WSL restent actifs pour le travail étranger ; RAM résiduelle non
mesurée, modèle8B laissé résident. Aucun prune, arrêt global ou publication supplémentaire.

Restent explicitement hors qualification : production/HA/deuxrépliques déployées,
toutes projections runtime, enfantSDKWindows réel ; suite Manifesto complète
échouée en environnementLNK1102/E0463, jamais présentée PASS ; contraintes
Apparatus/InspektorGadget distinctes de l'authentification. Quatrecompteurs5xx sont
MISSING, pas zéro ; pas mesureSQLdirecte, phase négative isolée ou config_dump
byteexact. Les inférences agrégées ciblées sont recevables selon la revue.

Localisation/impact/limites détaillés :
[dossier qualifié](../../../../docs/contracts/20261029-auth-security-qualified-state.md).
Registre :`.tmp/docker-resources-20261029.log`. Ledger de continuité :
`.cursor/handoffs/20261028-auth-security-closure.md`. Toutes les30tâches admises sont
complétées ; aucun pending/in_progress. Pas de nouvelle décision utilisateur requise.
