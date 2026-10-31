# Clôture de la chaîne d'authentification supportée — plan proposé

## Actualisation superviseur — reprise du 28 octobre 2026

**Dernière correction utilisateur :** préautoriser les opérations nécessaires plutôt que redemander des accords de principe au fil du long run. Le [rapport 06](reports/06-preautorisations-long-run.md) décrit la délégation au lead pour ratifier les contrats d'architecte du périmètre, publier le SDK validé et opérer le laboratoire full identifié. Les conditions techniques, preuves, intégrations et protections restent obligatoires. Cette révision de politique doit être présentée puis approuvée par le start utilisateur avant exécution.

**État courant final de préparation :** le PM a maintenant rendu un document natif ready sans question ouverte. Quatre racines ont été importées et restent pending, sans worker ; les dix consommateurs restent dans l'annexe du candidat et ne sont pas importés. Le [plan natif retenu](prepared-plan.md) et le [rapport de revue de l'import](reports/05-preparation-ready-et-import-controle.md) priment sur les états antérieurs ci-dessous. Import n'est pas approbation d'exécution ; le start utilisateur reste requis.

**Historique avant la dernière reprise :** le PM avait livré un candidat de quatorze tâches, déclaré blocked, sans import. Son blocage était la clarification encore présente dans le brouillon, remplacée par la confirmation explicite du périmètre authentification. Les autorisations différées ont été conservées. Le [rapport de clarification](reports/04-candidat-pm-et-perimetre-authentification.md) conserve cette étape ; il ne décrit plus l'état courant.

Le candidat natif de quatorze lots devient la proposition de travail privilégiée pour la prochaine reprise. Le découpage détaillé de quinze fiches ci-dessous reste une alternative documentaire antérieure, pas le résultat du PM ni le preview d'un import. Le graphe complet reste au dossier ; seules les branches prêtes doivent être exposées à l'import/publication initial, conformément à la barrière d'intégration locale.

Le [découpage actionnable](task-breakdown.md) complète ce plan avec critères de réussite, ownership, contexte, prérequis d'architecture et barrières d'intégration. Il reprend les éléments utiles de l'historique du superviseur ; ce n'est pas un résultat final du PM natif.

Les corrections de politique du rapport 02 priment sur la photographie initiale : OpenAI uniquement, long_run proposé mais non actif, QA proportionate et classification désactivée. La réparation de contexte conserve l'objectif enregistré : sécurité de l'authentification, sans développement implicite du plugin Ensemble.

L'API native a confirmé le superviseur `ses_e7cd58424ffeqZrWRWzJWh763f` / `orchestrator` et le PM distinct `ses_e7c953b4dffeSr2aLcciLsCZxQ` / `project-manager`. Le PM n'avait pas encore de document final exploitable ; son dernier tour était marqué interrompu. La commande native `prepare` a repris la préparation existante, avec confirmation de mise en file dans la même session. Résultat ou erreur attendus, sans polling ni remplacement.

Intégration documentaire seulement : aucun résultat blocked/incomplete promu ready, aucune tâche ajoutée manuellement, aucun worker, Cargo, runtime ou `start` lancé. L'import natif éventuel devra être vérifié séparément du présent dossier.

## Objectif et état d'autorisation

Corriger les frontières de confiance effectivement exposées, démontrer le parcours d'authentification supporté et réconcilier les ADR avec les preuves. Ce n'est ni une homologation de production ni une livraison de tous les fournisseurs de clés envisagés.

Préparation seulement. Politique native version 2 en brouillon ; aucun worker, classificateur, correctif de production, Cargo ou runtime autorisé avant le `start` utilisateur. La préparation n'accepte aucune nouvelle architecture. Après modification de `verification`, présenter à nouveau la politique par `config` avant `start`.

Le lead coordonne directement Ensemble, sans orchestrateur intermédiaire. Pas de scores de difficulté, pas de classificateur, pas d'override de modèle. Les consommateurs de branches isolées restent dans ce plan jusqu'à intégration vérifiée de leurs entrées.

## Base technique et preuves réutilisables

- Base finale de l'exploration : application `c72ae362b950c63d6736a6818c68babc5a8d6f40`, SDK sélectionné et présent `d60a88b2f2f499768a807941a1b4c80c9a366b3f`.
- Aucun delta `services/`, `workers/`, `runtime/`, `ops/`, Cargo ou gitlink entre le début de l'exploration `9f1238020c6dd4f2d574a03874cb86d9af9b738a` et cette base finale.
- Changements étrangers à préserver : état de travail Obsidian, `.serena/project.yml` du SDK et worktrees existants. Aucun commit, stash, reset ou nettoyage automatique.
- Rust Code MCP : recherches hybrides Qwen3-8B, graphe HIR et consommateurs recoupés avec Serena et les sources. Le SDK est une crate `rustycog-framework`, ses anciens dossiers ne sont pas des crates distinctes. Nom qualifié HIR : `rustycog_framework::http::…`.
- Rendu du profil local-full via la fonction existante `render`, en mémoire : 112 objets ; listener 10000 TLS avec certificat client obligatoire ; cinq destinations vérifient la CA sans matcher SAN explicite. Aucun fichier de manifest généré, aucun appel à un cluster.
- Les tests cités ci-dessous existent et leurs corps pertinents ont été lus. Aucune nouvelle exécution Rust ou preuve d'exploitation pendant l'exploration. L'attestation utilisateur du 8 octobre reste identifiée comme telle, pas transformée en artefact de cette mission.
- Handoff canonique : `C:/Users/djden/source/repos/AIForAll/.cursor/handoffs/20261028-auth-security-closure.md`.

## Canon et invariants

- `docs/adr/0304-jwt-acces-plateforme-rs256-jwks.md` : Accepted / Partial, jetons et confiance ; amendement privé par 0311.
- `docs/adr/0305-account-identity-trust-domain.md` : Accepted / Implemented sur son périmètre ; principal `(iss, sub)` ; schéma FGA multi-émetteur non décidé dans la photographie. Ne pas en déduire une preuve de séparation des permissions.
- `docs/adr/0306-hive-iam-configuration-signature.md` : Accepted / Partial, administration et transports Hive-IAM ; fournisseurs cloud et événements de profil encore hors livré.
- `docs/adr/0307-workload-identity-port.md` : Accepted / Implemented, port d'identité de service, pas obligation SPIFFE.
- `docs/adr/0308-mesh-authn-jwt.md` : Accepted / Partial ; confiance expirée à 60 secondes même en panne ; passerelle comme autorité des en-têtes, sans double validation JWT en mode mesh.
- `docs/adr/0309-remote-signer.md` : Accepted / Partial ; adaptateur HTTP existant, pas d'équipement matériel qualifié.
- `docs/adr/0310-admission-epochs-budget-jwks.md` : Accepted / Partial ; calcul/capacité partiellement remplacés par 0312, pas admission entièrement absente.
- `docs/adr/0311-crypto-privee-deleguee-aux-providers.md` et `0312-admission-jwks-par-slots-bornes.md` : Accepted / Implemented ; préserver délégation et bornes ratifiées.
- `docs/adr/0412-oauth-transaction-persistante-liee-navigateur.md` : Accepted / Partial ; contrat secondaire `docs/contracts/0412-oauth-exact-route-envoy-proof.md`, vrai navigateur et cible local-full. HTTP local décrit par ce contrat, TLS client obligatoire observé dans le rendu : contradiction à résoudre explicitement.
- ADR 0200–0202 : IT réelles, mocks HTTP sortants seulement, files opt-in. Aucun nouveau harness partagé implicitement autorisé.
- `docs/adr/0104-outbound-overrides-composition-root.md` reste Proposed / Partial ; pas de refactor global des injections de dépendances.
- Garder rotation et révocation distinctes, autorité writer après signature, état vide sans résurrection, quotas/réserve, refus sans mutation et confidentialité. Pas de nouvelle deny-list globale des jetons d'accès.

## Registre borné des constats et de leur preuve manquante

### Séparation des identités et des permissions

Observé : `middleware_auth::with_principal` insère `principal.sub` en UUID ; `middleware_permission::permission_middleware` construit `Subject::new(user_id)` sans émetteur ; `Subject::Display` produit `user:{uuid}`. Hive protège les routes de signature par cette permission puis ses handlers utilisent `AuthUser`, sans garde d'émetteur supplémentaire. Le traducteur Hive de sentinel-sync conserve également le UUID seul. IAM possède une garde `PlatformUser` distincte.

Conséquence inférée : une clé d'organisation reconnue pourrait signer un principal portant le UUID d'un administrateur d'une autre autorité et hériter de ses permissions. Ce n'est pas une exploitation exécutée. Le test Hive actuel exerce un même émetteur HS256 explicite, pas cette collision.

Preuve à produire avant correction si le harness le permet : clé d'organisation réellement reconnue, UUID identique à celui du témoin plateforme autorisé, OpenFGA réel, refus attendu avant RPC/mutation ; conserver un témoin positif légitime. La solution persistée et le traitement des droits existants doivent être décidés par architecture.

### Signataire distant et mode sécurisé

Observé : `JwtConfig::validate_signing_security` appelle seulement `remote_http_endpoint` pour le remote ; cette dernière vérifie l'activation explicite et l'adresse non vide. `RemoteSigningProvider::new` n'interdit pas HTTP, ne désactive pas les redirections et lit JSON/texte sans borne de taille explicite. Transit possède une politique plus stricte.

Correction locale proposée : durcir l'adaptateur supporté, avec exceptions de test/local explicites et bornes écrites. Ne pas désactiver le produit ni choisir un nouvel équipement matériel par défaut. Vérifier aussi les configurations historiques effectivement admises en mode sécurisé ; ne pas assimiler les bons défauts RS256 à un rejet de toutes les configurations faibles.

### Identité des serveurs joints par Envoy

Observé dans les sources et le rendu : CA de confiance et SNI, pas de correspondance explicite SAN pour ext-authz, IAM, Hive, Telegraph et Manifesto. Les tests reqwest de mauvais SAN n'exercent pas Envoy.

Preuve attendue : bonne CA et bon nom acceptés ; même CA mais mauvais nom refusé par le vrai Envoy. Vérifier chaque identité de certificat réellement utilisée, pas recopier aveuglément le SNI. Pas d'interception réseau démontrée à ce stade.

### Parcours navigateur

Observé : le contrat de preuve décrit HTTP local ; le rendu conserve TLS et certificat client obligatoire ; le profil contient `connectors=[]`. Les tests `browser_flow` utilisent reqwest et ne sont pas un moteur de navigateur.

À décider avant scénario : exposition publique locale, confiance/certificats du navigateur, origine/redirects/cookie, connecteur GitHub ou GitLab activé, fournisseur simulé ou réel. Préférer une simulation de fournisseur et un navigateur existant pour une preuve reproductible sans credentials personnels. Aucun changement silencieux du contrat de preuve Accepted.

## Lots et ordre d'intégration proposés

### Départs indépendants après start

1. **Contrat de séparation des identités et de preuve navigateur — architecte.** Propriété : nouveaux ADR Proposed et contrat ciblé dans `docs/contracts/`, amendements proposés aux références concernées. Réutiliser l'exploration ; exposer les choix de modèle des permissions et d'entrée navigateur. Aucune production, aucune nouvelle ratification automatique. Livrable : alternatives, recommandation, interfaces/stockage, migration/compatibilité, ownership et critères exacts. Les branches touchant les décisions ouvertes attendent leur ratification humaine puis leur intégration.

2. **Tests discriminants sur les risques constatés — test-engineer.** Travail isolé, production interdite. Préparer la collision d'émetteurs et les refus du signataire distant à partir des fixtures existantes ; auditer les témoins TLS/navigateur réutilisables. Nouveaux fichiers de tests dédiés, pas de deuxième auteur dans les fichiers de production. Le lead fige l'attribution exacte avant écriture. Le test révélant un défaut produit reste ROUGE, sans correction par son auteur. Une panne de fixture reste un problème d'environnement. Aucun Kind ; exécution native seulement sous bail Cargo et préflight convenus.

Ces deux lots sont parallèles car le premier écrit la conception et le second reproduit des invariants déjà ratifiés, sans dépendre du format de la solution future.

### Corrections libérées après intégration des tests et des entrées utiles

3. **Durcissement du signataire distant — implementer.** Propriété prévue : `services/IAMRusty/infra/src/signing/remote.rs`, configuration du provider dans `services/IAMRusty/configuration/src/lib.rs`, seuls consommateurs setup requis après examen des références. Tests externes du lot précédent intégrés avant correction. Durcir sans modifier d'autre provider ou refondre le setup. Toute signature publique modifiée exige examen des consommateurs et mise à jour du contrat si nécessaire.

4. **Vérification de l'identité des serveurs — implementer.** Propriété prévue : validation TLS dans `ops/deploy/mesh/envoy.yaml`, `ops/deploy/apps/overlays/kind-mesh/envoy-mesh.yaml`, rendu local-full uniquement si les noms l'exigent. Préserver les filtres et exemptions d'authentification. Ne pas changer en parallèle l'entrée navigateur. Le lot reste indépendant du signataire distant ; absence de test Envoy exécuté déclarée tant que la campagne n'a pas eu lieu.

5. **Préservation de l'autorité jusqu'aux permissions — hard-implementer.** Dépend du contrat ratifié et intégré, des tests de collision intégrés, et de l'attribution des chemins éventuellement partagés avec IAM. Périmètre potentiel, non autorisation de tout réécrire : modules SDK auth/permissions, modèle FGA et projections, gardes des consommateurs Hive/Manifesto/Telegraph, migration ciblée si retenue. L'architecte ferme l'allowlist avant publication. Préserver les comportements légitimes et les deux runtimes. Aucun bannissement global des identités d'organisation ni migration de données inventée.

Les lots 3, 4 et 5 peuvent avancer sur des arbres isolés et fichiers disjoints une fois leurs vraies dépendances intégrées. Si le contrat attribue un même chemin à plusieurs lots, le lead impose l'intégration du producteur avant le consommateur, plutôt qu'une mutation concurrente.

### Intégration, preuve et clôture

6. **Intégration et vérifications natives — lead, test-engineer pour les corrections de tests.** Arrêter proprement et intégrer les livrables, inspecter le diff, puis vérifier le snapshot intégré. Un seul Cargo Windows, jobs 24 et cible E:, arbres compilés gelés. Les mutations du SDK passent par une branche isolée ; publication exacte autorisée avant mise à jour du gitlink. Aucun push sous-entendu. Les autres branches peuvent continuer leur travail indépendant sans Cargo concurrent.

7. **Preuve intégrée bornée — test-engineer.** Dépend de l'architecture ratifiée, des corrections intégrées et des tests natifs pertinents. Préparer la commande exacte et les scénarios en réutilisant le laboratoire et les cas existants : vraie passerelle, navigateur réel, un connecteur activé, refus de contournement, fraîcheur/révocation et mauvais nom de serveur. Pas de nouveau framework ni stack de test générale. Ne pas exécuter les scripts historiques sans les gardes et le ciblage convenus. Avant démarrage Kind : paquet opérationnel, inventaire, baux et autorisation utilisateur ciblée sur la cible retenue ; aucune reconstruction destructive ou usage d'un contexte protégé.

8. **Revues et photographie finale.** Revue de sécurité indépendante, contexte fresh et briefing factuel persisté sans verdict de l'auteur. Revue test-reviewer bornée aux nouveaux oracles adversariaux et à leur caractère discriminant. Infra-verifier après changement infra significatif ; un rendu seul donne au mieux une vérification statique, pas un PASS réseau. Les lectures indépendantes sont parallélisables sur le diff gelé. Correctifs bloquants puis revalidation ciblée seulement. Architecte réconcilie les ADR et contrats effectivement couverts ; le lead actualise handbook et ledger après preuves.

## QA proportionnée proposée

Les commandes de référence et les critères sont dans `model-policy.json.verification`. Ce n'est pas une campagne à lancer en bloc. Après contrat et diff, le lead sélectionne les suites utiles, enregistre les nouveaux tests par nom, réutilise les preuves non affectées et rapporte exactement ce qui a été exécuté.

- Collision d'autorités : vrai JWT d'organisation accepté par l'authentification, témoin plateforme positif, OpenFGA réel, refus avant RPC/écriture, parcours légitime d'organisation selon le contrat retenu. Pas de faux refus dû à un jeton invalide.
- Signataire distant : HTTPS requis dans le mode sécurisé ; exceptions locales explicites ; mauvaise adresse et redirection refusées ; timeout et plafond de réponse ; contrôle positif cryptographique ; aucun secret transmis à une destination alternative. Tests wiremock seulement aux frontières HTTP sortantes ; TLS réel lorsque la propriété l'exige.
- Envoy : contrôles négatifs et positifs avec certificats signés par la même CA. Un simple statut HTTP d'erreur ne distingue pas indisponibilité et refus d'identité ; observer l'échec TLS et la santé du témoin positif.
- OAuth : matrice minimale du contrat existant via navigateur et vraies routes ; aucune promotion sur reqwest/curl seul, rendu YAML ou connecteur désactivé. Distinguer sécurité locale et cookie HTTPS de production.
- Non-régression : les tests de quotas, émission/fence, cache, sessions et OAuth déjà inspectés sont réutilisables. Rejouer seulement ceux affectés ou dont la provenance ne couvre pas l'état intégré.
- Qualité : fmt/check et Clippy ciblés ; pas de campagne globale Sonar ni de faux gate vert. Les avertissements antérieurs sont séparés des nouveaux diagnostics. Pas d'ajout de `-D warnings` global.
- Clôture : aucun défaut critique/élevé non résolu dans le périmètre ; les capacités exclues et suggestions non bloquantes sont documentées séparément. Pas de livraison cloud/HSM revendiquée. Nettoyage des ressources possédées et rapport des résidus selon le canon du dépôt.

Pas de qa-planner ni de chaîne systématique de reviewers imposés. Le présent registre sert de contrat QA initial ; l'architecte précise les interfaces ouvertes. Un nouveau harness partagé, une nouvelle infrastructure de preuve ou une réduction des critères exige un arbitrage, pas un élargissement autonome.

## Ressources et limites d'autorisation

- Slot Cargo Windows unique ; ne pas déduire les anciens jobs 1/2 des ledgers historiques.
- Docker uniquement pour fixtures tierces et images Linux de livraison Kind ; jamais compilation/test de développement dans Docker.
- Skills lifecycle et test-evidence avant préflight/runtime ; registre contemporain des IDs créés, inventaire avant/après et arrêt des seules ressources possédées. Pas de prune ni de suppression étrangère.
- Pas de nouveau cluster ni de mise à niveau du laboratoire, pas de secret personnel/fournisseur réel ou accès distant sans permission ciblée.
- Publication SDK, exécution Kind et décisions structurelles ont des limites explicites ; start n'est pas une autorisation générale de push, de suppression ou de changement de contrat.
- Le SDK Qwen3-8B du MCP reste résident ; pas de reconfiguration des outils partagés.
- Estimation de durée/coût total : non établie, car le contrat des permissions et le parcours navigateur peuvent changer le volume. Les coûts du catalogue ne sont pas un devis.
