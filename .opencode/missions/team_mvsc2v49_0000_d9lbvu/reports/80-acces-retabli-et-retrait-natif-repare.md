# 80 — Accès rétabli et retrait natif réparé sur un témoin

Les deux obstacles du rapport79 ont reçu des corrections vérifiées, sans modification
du plugin ni de la base interne d'OpenCode.

## Accès aux livrables et sauvegarde

Le serveur réellement actif est OpenCode2.2.1. Les permissions publiques de la
session auteur et du profil ne montraient pas de deny external_directory explicite,
malgré le refus natif observé. Avec l'autorisation explicite utilisateur,22règles
temporaires limitées aux11worktrees ont été ajoutées àcette session etrelues ;
ses22règlespréexistantes et les interdictionsbrowser ont été conservées.

La lecture native anciennementrefusée aensuite réussi. Onzecommitslocaux préservent
leslivraisons, sanschangerleurscontenus ; onzearbres etdeuxSDKpropres. L'auteurn'a
ni fusionné nipublié. LespinsSDKhistoriques74e8/72845 sont desancêtresdupin02e5,
pasdesdowngrades àintégrer.

Un écart de procédure a été identifié : l'auteur avait omis ponctuellement le hook
fmt pour respecter notre interdiction Cargo de préparation. Cette omission n'est
pas acceptée comme un PASS. Pool34 détient maintenant le seul slotCargo pour les
11contrôles `cargo fmt --all -- --check`, sans build/test/runtime ou reformatage.
Les permissions ciblées requises lui ont été délivrées ; les snapshots ne sont pas
à refaire. Revue indépendante de contenu en parallèle, strictement READONLY.

## Retrait de l'enregistrement orphelin

Un DELETE public forcefalse a confirmé l'erreur sur2.2.1 : dossier indisponible.
Le dossier était vide, absentdeGit mais présentdansl'inventairenatif. Lesrefs de
checkout et sauvegarde étaient identiques etpropriétésdumembre.

La correction a consisté à reconstruire un VRAIcheckoutGit àpartir de sabranche
préservée danscecheminvide, puis àrelancer `team_merge` natif. Cettefois l'outil
aannoncé intégration etlibération terminées ; le statut du membre est released.
Pasdefichier .git falsifié, deforce, deprune ou d'écriture de base. Lesautres
orphelins ne seront traités que si cesmêmesconditions sont vérifiées.

## Suite

Terminer les vérifications des snapshots et les résolutions de versions obsolètes,
effectuer les intégrations natives séquentielles, puis contrôler et publier les
compléments utiles sur master avant cleanup final. Aucune promotion de0315rejetée,
0504Proposed, production/HA/HSM ni lancement de campagne k6.
