# État qualifié — sécurité de l'authentification, 29 octobre2026

**Accepted / Implemented pour la preuve locale0412 uniquement.** Auteur de la
proposition : architecte/pool-4,97lignes lues intégralement par le lead. Le lead
ratifie explicitement cette réconciliation sous délégation utilisateur AVANT
modification des ADR/handbooks. Proposition originale conservée au worktree auteur.
Aucune nouvelle décision produit, campagne, permission ou promotion transitive.

## Jugement et portée

Les critères du contrat [0412 exact-route](0412-oauth-exact-route-envoy-proof.md)
§§162–173/195–216 sont satisfaits pour le laboratoire ratifié. Réalité0412 passe
à **Implemented**, Statut **Accepted inchangé**, décisions1–7 inchangées. Les états
historiques Partial2026-10-04/08 et le refus401 avant correction restent datés.
L'accord explicite du lead applique la délégation humaine, pas un Accept autonome.

Cela qualifie le parcours et les refus ciblés, **pas production/edgeTLS/HA, deux
répliques déployées, GitLab vendor réel ou toutes les projections runtime**.
Atomicité de consommation writer ne signifie pas rollback global des effets
après consommation. Échec après consommation exige un nouveau démarrage.

## Sources et preuves exécutées

- Application/image/navigateur7b8c1f95faa7c5db8d7aa9bcfbfb37fd8cb56ccd ;
  SDK02e5db23aac6016aab062c3078696d5f7fd464ef publié sur sa branche de mission.
  Validation complémentaire8606173f3a5a5f82e4f67aa75c62006977cdffa7 : diff des
  services/workers/crates/SDK/Cargo/config vide ; seuls scripts QA/docs évoluent.
- Chrome149, profil neuf/contexte vide, run browser-proof-2b6c3a80b3b6498aaedcd8393987b180.
  P1départlogin, P2login200, P3départauthentifié puis callbackexact sansBearer,
  N1/N2/N3=403 dans le même run. Vrai connecteurGitLabConnect configuré, fournisseur
  synthétique et PKCE effectivement utilisés ; pas qualification vendor réel.
- Cible kind-aiforall-local-full/aiforall-local-full, topologie1CP+2workers,
  entréehttp://127.0.0.1:18080→30080→Envoy10000 ; autoritéIAM8080/iam inchangée.
  CPcontainer6fd561584da07159621e52e04580dacbbc0dfabdf7a79a221e730ea2da89b7f1,
  kube-systemUIDd5a5c123-384d-4b3e-a45a-43d9ad6170d7 et mapping vérifiés au bail6b7.
- Même Envoy/incarnation/epoch0, uptime+49s. QuatrecompteursIdP+2chacun ; IAM12→16
  pour quatre succès applicatifs, ext-authz3→7. IAM5xx1→1 et erreurs mesurées
  SAN/connexion/timeout sans hausse. Les erreurs historiques ne sont pas effacées.
- Trois suites IT : oauth_browser_transactions9, auth_transactions_postgres6,
  oauth_cleanup_lifecycle2 : **17PASS/0failed/0ignored/0filtered** sousWindowsnatifs,
  unCargo/24jobs/E:. Trois fixtures PostgreSQL créées/retirées ; six conteneurs
  préexistants AUX IT inchangés, dont trois nœuds possédés par cette mission.
- Gates purs postfix :19tests domaine/3tests vraiRS256, checkiam-service/fmtall
  exit0. Reproduction préalable distincte1PASS : ancienmint refusé, accèsconfiguré
  accepté ; elle n'est pas comptée comme correctif ou exécution navigateur.
- ImageIAM seule+inittrust : manifeste OCIb6f140d16558ae9b9dae53a4944e897226952ab91b8b09d02f9dcff69fc6d960,
  config.digest33e39e6487b76dbc71770512b93dae66d06ac922d785e485024ea4df5ddf3b6e
  rapporté par le Pod. RelationEngineDescriptor archivée, pas fausseégalité des
  deuxdigests ni inspectionCRIindépendante. Pod377006…→b4f83cd0…Ready ; seules
  deux référencesimage changées, autresressources comparées en mémoire parparent.

## Revues indépendantes et limites de preuve

Briefings canoniques `.cursor/review-briefings/` :1533Zcore,1538Z/1603ZsélectionIT,
1602Zoraclesnavigateur, infrarollout7b8/addendumOCI et finalinfra de campagne.
Aucun défaut HIGH/CRITICAL ouvert connu dans le périmètre selon ces revues ; pas
garantie exhaustive de tous les services. Auteur documentaire sans runtime.

RefuspréIAM des négatifs inféré par conservation quatrepositifs/+4, trois403 et
routes protégées. Absence d'effetsDB inférée de ce refus, pas mesureSQL. Liaison
configurationstatique par Podargs/mount/CMUIDRV/hash, no_dynamic, incarnationstable
et effetsréels : **pas config_dumpbyteexact**. Revue1602Z : oracles recevables,
aucun gap bloquant additionnel ou nouvelle infrastructure demandée par principe.
Quatrecompteurs5xxMISSING et statutsAdminINCONCLUSIVE conservés ; sous-ensemble
numérique exploitable seulement. Snapshotscomplets de préservation du rollout non
archivés : vérification comparativemémoireparent, pas recalcul indépendant global.

Finalinfra14 : PASS documentaire borné du laboratoire observé, aucun écart
bloquant ; pas PASSinfra global/HA/production. ChromePID51016 et forward fermés,
ports/markers absents vérifiés parent. Les trois nœuds possédés sont ensuite
arrêtés gracieusement, sans suppression :275.185s/exit0, troisExited130/nonrunning.
SixIDsavant/après conservés, trois autres ressources et états inchangés, dont
legacy40201…toujoursrunning. DockerDesktop/WSL restent actifs pour ce travail
étranger ; RAM globale non libérée/mesurée, pas de shutdown global.

## Preuves historiques réutilisées et gaps non promus

Principal : Hive67 sur sourceba21/SDK74e8 ; IAM290PASS/1ignored sur00c6cf2/SDK02e5,
preuves historiques précisément étiquetées au ledger, pas requalifiées en tests
du nouveau correctifOAuth. SDK7unitaires de ledger/discovery ; enfantWindowsréel
non exécuté. Remote : config29/protocole19/TLSprivé2 sur ses sources historiques,
contrattransport respecté, pas qualificationcloud/HSM/production.

Nonqualifiés : deuxrépliqueslive/HA/projectionsruntimecomplètes ; suiteManifesto
complète échouée en environnementLNK1102/E0463, jamaisPASS ; Apparatusoperator
PodSecuritybaseline et InspektorGadget nonattesté horsmissionauth. Ne pas étendre
ce verdict à ces capacités ni relancer une campagne par principe pour leur statut.
ADR0304/0308/0309/0314/0606 et contratsprincipal/remote gardent leurs états propres.

## Références de campagne

Rapports locaux de mission :66intégration,67neufIT/cleanup,68image,69navigateur,
70qualification/arrêt, puis clôture. Ledgercanon :
`.cursor/handoffs/20261028-auth-security-closure.md` (tentatives/ratifications/limites).
Preuvesprivéesnonsecrètes référencées au bail6b7 : dossierbrowser2b6, image-rolloutdb884
et final-runtime-release. Registre append-only :`.tmp/docker-resources-20261029.log`.
OAuth9log :`.tmp/oauth-authority-it-native-5e08087b59d54094bc6fad12967cb5b7.log` ;
6+2logs :`.tmp/oauth-remaining-it-b57e446a5d5544cc927fa7cded96517f/`.
Les artefactsPENDING historiques ne sont pas réécrits ; ce jugement final les
complète après les résultats réels. Aucun JWT/cookie/code/state/clé en sortie.
