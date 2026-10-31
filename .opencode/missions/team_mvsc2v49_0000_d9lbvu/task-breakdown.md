# Sécurité de l'authentification — tâches actionnables proposées

## Portée et provenance

**Remplacé pour la suite de la mission :** le [plan natif retenu de quatorze lots](prepared-plan.md) est désormais la référence de découpage. Le PM a rendu un document ready et seules ses quatre racines ont été importées. Le présent document conserve la proposition antérieure du superviseur à titre historique ; il ne définit ni une seconde publication ni une campagne supplémentaire de tests.

**Actualisation après lecture du candidat PM :** le PM natif a maintenant rendu quatorze lots, encore blocked sur la clarification de périmètre. Cette proposition native, revue dans le [rapport 04](reports/04-candidat-pm-et-perimetre-authentification.md), est privilégiée pour la reprise. Les quinze fiches du présent fichier restent la trace détaillée d'une proposition antérieure du superviseur ; elles ne sont pas présentées comme importées ou comme résultat final du PM. Le brouillon confirme désormais l'authentification et conserve les autorisations différées, sans modifier la readiness native.

Complément du [plan de mission](task-plan.md), intégré par le superviseur à la demande « intègre le ». Ce document reprend le découpage utile présent dans l'historique du superviseur, dont certaines réponses avaient été attribuées à tort au PM. Il ne constitue ni le résultat final du PM natif ni une approbation d'exécution.

L'objectif reste la correction et la qualification de l'authentification existante. Le développement de l'orchestrateur du plugin Ensemble n'est pas inclus. Aucun score de difficulté, modèle de tâche ou pipeline QA supplémentaire n'est proposé. Politique actuelle : OpenAI uniquement, long_run non actif avant start, QA proportionate. Les commandes du brouillon sont des références à sélectionner selon le diff, pas une campagne exhaustive automatique.

Base de l'exploration réutilisée : application `c72ae362b950c63d6736a6818c68babc5a8d6f40`, SDK `d60a88b2f2f499768a807941a1b4c80c9a366b3f`. Ce document n'apporte aucun nouveau PASS comportemental. Les références locales ci-dessous sont des identifiants de plan, pas des IDs de tâches Ensemble.

## Contrat commun des tâches

- Entrées : plan, ledger `.cursor/handoffs/20261028-auth-security-closure.md`, ADR/contrats cités, HEAD et diff intégrés fournis au départ. Ne pas refaire une exploration globale lorsque ces preuves restent applicables.
- Canon : ADR Accepted prioritaires ; les décisions nouvelles restent Proposed jusqu'à ratification. Les contrats sont écrits par architecte. Code et tests inspectés ne constituent pas une preuve de tests exécutés.
- Auteurs : worktrees isolés, un propriétaire par fichier mutable. Contexte fresh avec briefing ciblé ; aucun héritage implicite de cette conversation ou d'une session PM hors équipe. Les reviewers restent fresh, en lecture seule, sans conclusions d'auteur dans leur briefing.
- Ressources : un seul Cargo Windows, 24 jobs sans override, cible E:. Le lead tient les baux des fixtures et du runtime, le gitlink SDK, les briefings et le ledger. Pas de Cargo pendant la mutation de l'arbre compilé.
- Vérification : preuve ROUGE avant correction lorsque le harness le permet ; erreur d'environnement distincte. test-engineer n'écrit pas la production. Aucun nouveau harness partagé ou élargissement de campagne sans arbitrage.
- Barrière : avant un consommateur, le lead intègre et inspecte le livrable du producteur, fournit son vrai HEAD/diff et les véritables IDs de dépendance. Fin d'auteur n'est pas intégration. Aucun consommateur de branche isolée n'est publié prématurément.
- Autorisations : start utilisateur avant exécution ; décisions structurelles, publication distante SDK et opérations Kind gardent leurs autorisations ciblées. La préparation n'autorise pas leur exécution.
- Estimation : aucun délai ni coût total fiable à ce stade. Les tâches ci-dessous expriment leurs incertitudes plutôt que des scores ou des durées inventés.

## 1. Contrat d'identité jusqu'aux permissions — identity-contract

**Propriétaire conseillé :** architecte. **Type :** architecture. **Dépendances :** aucune. **Architecte préalable :** cette tâche produit la décision ; pas de code.

**Historique nécessaire :** ADR 0305 Accepted / Implemented sur son périmètre ; principal `(iss, sub)`, mais schéma FGA multi-émetteur explicitement non décidé. `middleware_auth::with_principal` conserve le principal mais insère aussi un UUID ; `middleware_permission` et `Subject` consomment le UUID seul. Le traducteur concerné est `workers/sentinel-sync/src/translator/hive.rs` ; les modèles sont `ops/openfga/model.fga` et `model.json`.

**Ownership / livrable :** ADR Proposed et contrat ciblé sous `docs/contracts/`, chemins documentaires réservés par le lead. Définir représentation, propagation, projections, événements sans issuer, compatibilité des droits existants, reconstruction/migration nécessaire et rollback. Pas de modification de modèle FGA ou de production dans ce lot.

**Critères de réussite :**
- Une même valeur de subject provenant de deux autorités ne peut hériter des droits de l'autre.
- Le contrat préserve le compte plateforme multi-organisation, les identités d'organisation légitimes et les accès publics anonymes explicitement prévus.
- SDK, services, producteurs/suppresseurs de tuples et stockage éventuel ont une allowlist et un ordre d'intégration précis.
- Une alternative rejetée est expliquée ; les nouveaux choix à ratifier et leurs conséquences sont identifiés.

**Vérification :** sources/références utiles recoupées avec ADR 0302/0305 ; aucun runtime. L'implémentation attend contrat intégré et ratification des nouveaux choix structurels. **Incertitude :** représentation et transition non fixées.

## 2. Contrat navigateur et laboratoire — browser-contract

**Propriétaire conseillé :** architecte. **Type :** architecture. **Dépendances :** aucune. **Architecte préalable :** cette tâche résout la contradiction documentaire/opérationnelle.

**Historique nécessaire :** ADR 0412 Accepted / Partial ; `docs/contracts/0412-oauth-exact-route-envoy-proof.md` fixe HTTP local, `kind-aiforall-local-full`, navigateur réel et connecteur github ou gitlab activé. Le rendu antérieur de 112 objets conservait TLS et certificat client obligatoire. Ce rendu n'est pas un test réseau.

**Ownership / livrable :** clarification/contrat documentaire sur des chemins distincts du lot identité ; pas de manifests ou runtime. Décrire entrée navigateur, hops internes, ports, origine, cookies, redirections, certificats, fournisseur simulé ou réel et pilote existant réutilisable.

**Critères de réussite :**
- La cible et la matrice P1–P3/N1–N3 restent celles du contrat existant ; aucune substitution silencieuse par curl, autre cluster ou listener inventé.
- HTTP local et transport interne protégé sont distingués, sans affaiblissement global du mesh.
- Un seul connecteur de campagne est retenu ; un fournisseur simulé réellement appelé est recommandé pour éviter les credentials personnels.
- Tout nouvel élément indispensable de preuve est borné avant écriture. Changer une décision Accepted exige un amendement Proposed ratifié ; appliquer une décision déjà Accepted ne nécessite pas une nouvelle ratification par principe.

**Vérification :** comparaison du contrat, du rendu et des configurations Envoy, sans runtime. **Incertitude :** forme exacte de l'alignement à confirmer, sans réinventer la cible.

## 3. Régression de collision d'autorités — identity-regression

**Propriétaire conseillé :** test-engineer. **Type :** implémentation de tests. **Dépendances :** aucune. **Architecte préalable :** non pour reproduire l'invariant déjà Accepted ; le test ne choisit pas la solution.

**Historique nécessaire :** `services/Hive/tests/organization_signer_permissions.rs` possède des témoins réels OpenFGA et refus avant RPC, mais utilise un seul émetteur de test. `services/Hive/tests/common.rs` fournit serveur réel et fixtures.

**Ownership :** tests Hive et helpers strictement nécessaires, fichier dédié ou extension attribuée avant écriture. Aucune production.

**Critères de réussite :**
- Témoin plateforme autorisé sur l'organisation exacte avec OpenFGA réel.
- Jeton d'une autorité d'organisation reconnue avec le même subject ; preuve qu'il est authentifié avant le contrôle ciblé, pas simplement invalide.
- Refus attendu avant RPC IAM et mutation, avec observations de stockage et compteurs.
- Un défaut produit reste ROUGE, transmis à son propriétaire ; une panne de fixture n'est pas ce ROUGE.

**Vérification :** commande ciblée existante `cargo test --locked -p hive-service --test organization_signer_permissions` ; nom et commande d'un nouveau test enregistrés avant exécution. Préflight, slot Cargo et fixtures sous bail ; cleanup IT selon canon. **Incertitude :** adaptation bornée du harness aux clés/émetteurs de test.

## 4. Régressions du signataire distant — remote-regressions

**Propriétaire conseillé :** test-engineer. **Type :** implémentation de tests. **Dépendances :** aucune. **Architecte préalable :** non pour les refus attendus et écarts observés ; remonter une rupture de contrat nécessaire.

**Historique nécessaire :** `services/IAMRusty/infra/tests/remote_signer.rs`, fixture référencée `services/IAMRusty/tests/fixtures/remote_signer/mod.rs`, ADR 0309 Accepted / Partial. Le constructeur possède déjà un timeout de dix secondes : ne pas annoncer son absence.

**Ownership :** tests externes configuration/infra et fixture existante ; pas de modification de `configuration/src/lib.rs` ou du client de production.

**Critères de réussite :**
- Reproduire l'admission actuelle de HTTP remote en mode Verified avec témoin distinct pour les exceptions locales.
- Détecter toute redirection suivie vers une seconde destination, notamment la transmission d'identité.
- Couvrir la lecture excessive sans Content-Length et le chemin d'erreur ; borne proposée explicite à confirmer au plan local.
- Conserver les témoins digest-only, clé publique et identité de service. Les défauts restent ROUGES sans correction produit par test-engineer.

**Vérification :** `cargo test --locked -p iam-configuration --lib` et `cargo test --locked -p iam-infra --test remote_signer`, filtres utiles et commandes nouvelles nommées avant exécution. Wiremock aux frontières HTTP sortantes ; ce n'est pas une preuve TLS réelle. **Incertitude :** limite de réponse et exceptions locales à fixer.

## 5. Durcissement du signataire distant — remote-hardening

**Propriétaire conseillé :** implementer. **Type :** implémentation. **Dépendances :** régressions remote intégrées. **Architecte préalable :** pas de nouvelle architecture pour un durcissement compatible ; requis si contrat public modifié ou fonctionnalité désactivée.

**Ownership :** `services/IAMRusty/infra/src/signing/remote.rs`, `services/IAMRusty/configuration/src/lib.rs` et seuls consommateurs setup nécessaires après examen des références. Tests du producteur modifiés seulement après réattribution explicite.

**Critères de réussite :**
- Verified refuse les destinations non HTTPS/invalides avant résolution de secrets et réseau ; exceptions non productives explicites.
- Redirections refusées ; réponses succès/erreur bornées réellement, même sans taille annoncée ; timeout conservé.
- Aucun credential, matériau privé ou corps distant brut dans les erreurs ; aucun repli PEM ou autre clé.
- Régressions intégrées vertes et protocole existant préservé ; pas de nouveau fournisseur matériel/cloud.

**Vérification :** suites configuration/remote ciblées, fmt et Clippy utiles, références des signatures affectées et branchement setup. Ressource Cargo exclusive. ADR 0311 Accepted / Implemented reste applicable. **Incertitude :** compatibilité des paramètres du constructeur.

## 6. Noms des serveurs joints par Envoy — envoy-server-identity

**Propriétaire conseillé :** implementer. **Type :** implémentation infra. **Dépendances :** aucune. **Architecte préalable :** seulement si la hiérarchie de confiance change ; pas pour appliquer la vérification compatible de l'identité déjà attendue.

**Ownership :** validation TLS des destinations dans `ops/deploy/mesh/envoy.yaml` et `ops/deploy/apps/overlays/kind-mesh/envoy-mesh.yaml`. Pas de listener navigateur dans ce lot.

**Critères de réussite :**
- Politique explicite de nom serveur pour ext-authz, IAM, Hive, Telegraph et Manifesto.
- Noms recoupés avec les certificats provisionnés, et non copiés aveuglément du SNI.
- CA toujours vérifiée ; pas de désactivation TLS, confiance élargie, changement des filtres ou exemptions OAuth.
- Contrôle statique et scénario négatif livrés sans prétendre avoir exécuté le test Envoy.

**Vérification :** rendu hors ligne ciblé sans accès cluster ni sortie de Secrets ; contrôler les cinq destinations dans les sources et le rendu. Le bon nom/mauvais nom avec même CA sera exercé par la campagne réelle. ADR 0308 Accepted / Partial. **Incertitude :** correspondance exacte avec les certificats actuels.

## 7. Autorité dans les permissions et projections — authority-permissions

**Propriétaire conseillé :** hard-implementer. **Type :** implémentation transverse. **Dépendances :** contrat identité intégré et nouveaux choix ratifiés ; régression identité intégrée.

**Ownership :** allowlist finale du contrat : SDK auth/permission, consommateurs et projections nécessaires, modèles/stockage seulement si retenus. Aucun chevauchement avec configuration/signature remote sans séquencement explicite du lead. Auteur transverse unique recommandé, subdivision seulement après fixation des interfaces.

**Critères de réussite :**
- Middleware et producteurs/suppresseurs de tuples emploient la même identité autoritative ; aucun repli UUID seul ne réintroduit la collision.
- Anciens événements/droits traités selon le contrat ; créations, mises à jour et suppressions cohérentes.
- Accès plateforme multi-org, identités org légitimes et anonymat public prévu restent fonctionnels.
- Régression de collision et témoin positif verts ; frontières Bearer et principal mesh utiles couvertes.
- SDK exact et consommation traçables, sans push ni gitlink choisi avant l'autorisation distincte prévue.

**Vérification :** tests SDK permissions/mesh, régressions Hive et tests des traducteurs/consommateurs modifiés. Analyse des références avant changement partagé ; limites d'index worktree annoncées. SDK puis consommateurs intégrés réellement, pas validation contre un fichier absent du checkout. Slot Cargo unique. **Incertitude :** branche principale de volume, gouvernée par la décision d'architecture.

## 8. Alignement du laboratoire navigateur — browser-lab-alignment

**Propriétaire conseillé :** implementer. **Type :** implémentation. **Dépendances :** contrat navigateur intégré et nouvelles décisions ratifiées si nécessaires ; lot noms TLS intégré pour éviter deux auteurs sur Envoy.

**Ownership :** seuls chemins overlay/rendu/configuration attribués par le contrat. Pas de refonte OAuth, stockage transactionnel ou responsabilités du connecteur.

**Critères de réussite :**
- Entrée, ports, origine et redirections conformes au contrat, hops internes toujours protégés.
- Profil de campagne activant le connecteur choisi, sans changer implicitement tous les profils.
- Matchers exacts préservés ; pas de wildcard IAM ou alias public de callback.
- Paramètres navigateur/certificats explicites, sans secret personnel requis par défaut.
- Nouvel élément de laboratoire borné et approuvé avant écriture ; aucun runtime pour préparer le lot.

**Vérification :** rendu hors ligne, comparaison des routes à la matrice et maintien des politiques TLS intégrées. Pas de verdict navigateur/réseau déduit du rendu. **Incertitude :** delta exact déterminé par le contrat.

## 9. Scénarios de preuve intégrée — integrated-scenarios

**Propriétaire conseillé :** test-engineer. **Type :** implémentation de tests. **Dépendances :** alignement navigateur, permissions et signataire intégrés. **Architecte préalable :** contrats déjà produits ; nouvelle infrastructure hors de ces bornes à arbitrer.

**Ownership :** scénarios/tests ou adaptations de scripts attribués par le lead ; pas de production. Les nouveaux chemins sont annoncés comme propositions, jamais comme fichiers existants.

**Critères de réussite :**
- Six cas P1–P3/N1–N3 exécutables avec navigateur réel, Envoy et connecteur actif.
- Témoin TLS bon nom et mauvais nom sous la même CA ; distinguer refus TLS et indisponibilité.
- Réutilisation sélective des cas mesh/fraîcheur/révocation pertinents, sans recopier la campagne historique.
- Captures expurgées, témoins contre les faux PASS dus à une capture vide ou fixture absente.
- Commandes exactes, contexte, préconditions, effets transitifs et cleanup décrits avant exécution.

**Vérification :** examen des scripts `ops/scripts/mesh-authn-kind-e2e*.sh` avant réutilisation ; `browser_flow.rs` utilise reqwest et ne prouve pas un navigateur. Revue des oracles prévue, pas de Kind dans ce lot. **Incertitude :** adaptations minimales nécessaires aux mécanismes existants.

## 10. Qualification native intégrée — native-qualification

**Propriétaire conseillé :** test-engineer. **Type :** QA exécutée. **Dépendances :** corrections et scénarios intégrés par le lead ; SDK disponible réellement. **Architecte préalable :** contrats ratifiés consommés, pas de décision nouvelle.

**Critères de réussite :** snapshot/diff/SDK exacts identifiés ; nouvelles régressions et témoins verts ; erreurs d'environnement, skips, tests absents et défauts fonctionnels séparés. Les suites OAuth sont rejouées si parcours ou matchers ont changé, conformément au contrat. Zéro conteneur IT possédé running en fin de campagne, inventaire avant/après.

**Vérification :** sélectionner fmt/check/Clippy et tests utiles dans `model-policy.json.verification`, sans lancer toute la liste par principe. Si OAuth/matchers changent : suites `oauth_browser_transactions`, `auth_transactions_postgres`, `oauth_cleanup_lifecycle`. Windows natif, Cargo unique/24 jobs/E:, fixtures existantes, jamais Kind pour les IT. Aucune correction production par l'auteur QA ; remonter la preuve ROUGE.

**Ownership / ressources :** commandes et preuves, arbre compilé gelé ; lifecycle/test-evidence avant préflight, baux et cleanup suivis par le lead. **Incertitude :** durée connue seulement après sélection des suites et préconditions.

## 11. Revue indépendante de sécurité — security-review

**Propriétaire conseillé :** security-reviewer, lecture seule, contexte fresh. **Type :** review. **Dépendances :** diff corrections/scénarios intégré ; ne dépend pas d'un verdict QA pour lire le code.

**Critères de réussite :** examiner collision, projections, échappatoires de configuration, redirections/tailles, TLS et exemptions exactes ; findings avec chemins et scénarios. Distinguer statique, hypothèse et reproduction. Défauts critiques/élevés bloquants ; suggestions et hors-périmètre séparés. Aucun test exécuté revendiqué au titre d'une lecture.

**Vérification :** briefing factuel sous `.cursor/review-briefings/` persisté par le lead avant correction, sans verdicts d'auteurs. Contrats Accepted/ratifiés et snapshot gelé. Corrections par les propriétaires puis recontrôle ciblé. Pas de Cargo/runtime acquis par principe. **Incertitude :** findings inconnus avant revue.

## 12. Revue des nouveaux oracles — test-oracle-review

**Propriétaire conseillé :** test-reviewer, lecture seule, contexte fresh. **Type :** review. **Dépendances :** tests nouveaux, permissions et scénarios intégrés.

**Critères de réussite :** collision authentifiée et témoin légitime ; redirection observée à la destination secondaire ; lecture réellement bornée ; vrai Envoy/navigateur plutôt que rendu/reqwest/Ready. Gaps bloquants attribuables à test-engineer ; suggestions sans boucle obligatoire.

**Vérification :** assertions discriminantes et provenance des ROUGES, sans imposer un framework de mutation. Ne pas confondre couverture inspectée et exécution native. Briefing fresh persistant ; nouvelle passe selon risque et contrat QA, pas parce qu'un test a simplement été ajouté. Aucun Cargo/runtime par principe.

## 13. Campagne navigateur et passerelle — integrated-proof

**Propriétaire conseillé :** test-engineer. **Type :** QA exécutée. **Dépendances :** qualification native pertinente et revues intégrées ; findings bloquants corrigés. **Autorisation distincte :** paquet opérationnel Kind, cible et baux explicites avant démarrage/réutilisation.

**Critères de réussite :**
- Contexte full/topologie/IDs/port observés concordants, images et code identifiés.
- P1–P3/N1–N3 sur un même run, navigateur réel et connecteur actif.
- Bon nom TLS accepté et mauvais nom de même CA refusé par Envoy, échec TLS observé.
- Cas mesh supplémentaires retenus rapportés à leur juste portée, sans qualification production implicite.
- Limites et cas non exécutés explicites ; runtime sous bail pendant la vérification infra puis cleanup possédé contrôlé.

**Vérification :** commandes intégrées et approuvées uniquement, audit des effets transitifs et registre contemporain des IDs. Builds Linux de livraison distincts des tests Windows. Port occupé, mauvais contexte ou IDs absents donnent STOP/INCONCLUSIVE, pas un détour vers un autre environnement. Aucune suppression/prune/reset. **Incertitude :** préflight, images et accès nécessaires, sans coût inventé.

## 14. Vérification infra indépendante — infra-verification

**Propriétaire conseillé :** infra-verifier, lecture seule, contexte fresh. **Type :** QA par observations effectives. **Dépendances :** changement infra significatif et campagne intégrée ; bail parent maintenu le temps de cette vérification bornée.

**Critères de réussite :** verdict PASS/FAIL/INCONCLUSIVE avec contexte, endpoints, healthchecks et chemin réseau ; configuration TLS chargée recoupée avec les sources ; IDs et baux concordants. Absence d'accès runtime explicitement limitée, sans transformer le rendu en PASS réseau.

**Vérification :** lectures ciblées autorisées, sans réparation ni modification de manifests. Inventaire remis au lead pour arrêt possédé final ; pas de runtime prolongé sans travail actif. **Incertitude :** observations disponibles et état du laboratoire.

## 15. Réconciliation documentaire — documentation-reconciliation

**Propriétaire conseillé :** architecte pour ADR/contrats, lead pour handbook/ledger. **Type :** documentation, pas nouvelle architecture. **Dépendances :** preuves natives/intégrées, revues et vérification infra rendues et contrôlées.

**Critères de réussite :** comportements obsolètes de repli corrigés ; refresh, accès déjà émis et révocation de clé distingués ; Statut/Réalité proportionnés aux preuves. Pas de clôture totale de 0309 sans fournisseur matériel qualifié, ni d'Accept/promotion exigeant accord humain déduit du travail d'agent. Bilan avec fichiers, commandes exécutées/non faites, limites, findings et cleanup. Aucun PASS partiel présenté globalement.

**Vérification :** chaque assertion liée au snapshot et aux rapports, cohérence ADR/contrats/handbook et liens. Aucun rejeu de tests pour remplir le rapport. Pas de synchronisation wiki automatique : les notes de conception ne sont pas le canon. Aucun Cargo/runtime.

## Parallélisme et chemin critique

- Branches initiales indépendantes : contrats identité/navigateur sur chemins réservés distincts ; régressions Hive/IAM ; noms des destinations Envoy. Les commandes Cargo de ces branches restent séquentielles.
- Après intégration des entrées : signataire et permissions peuvent avancer en parallèle si l'allowlist exclut tout chevauchement. L'alignement navigateur attend l'intégration TLS parce qu'il peut modifier les mêmes fichiers Envoy.
- Sur snapshot gelé : QA native et lectures indépendantes des reviewers peuvent être concurrentes si leurs ressources ne se heurtent pas. Les reviewers en lecture seule ne sont pas des branches d'écriture factices.
- Chemin critique probable : contrat identité ratifié → reproduction intégrée → correction SDK/projections et consommation autorisée → qualification native → campagne intégrée → vérification infra → documentation. Le navigateur peut devenir une branche critique parallèle.
- Le lead ne publie pas les consommateurs avant intégration de leurs entrées ; pas de tâche factice de merge, pas de faux IDs. Un éventuel import natif est contrôlé séparément de ce plan.

## État de préparation et limites restantes

Le PM natif distinct a été remis en file dans sa session existante. Son résultat final ou son erreur reste attendu ; ce document n'est pas un substitut ready. Le superviseur comparera son retour à ce plan, conservera les véritables blocages et n'importera pas un document blocked/incomplete comme ready. Aucun worker, test, runtime ou implémentation n'est lancé par cette intégration documentaire. Aucune modification de la politique proposée ; aucune approbation utilisateur d'exécution déduite de « intègre le ».
