# Sondes admises, outils Linux retrouvés sans lancer le navigateur

L'objectif est d'éprouver les deux pistes proposées par l'utilisateur : capacité
de créer un espace de noms Linux et dépendances manquantes. Le diagnostic est
maintenant admis par un contrat ciblé, sans modifier la sécurité et sans relancer
Chrome. Il ne s'agit ni d'une collecte de son erreur privée ni d'une qualification.

## Décision et préparation vérifiées

L'architecte autorise une opération ponctuelle distincte : commande userns fermée,
`ldd` du seul ELF et chargeur fiables, même superviseur Child et environnement
épuré, sorties privées et projection limitée. Le lead a ratifié ce périmètre dans
la délégation utilisateur ; contrat et référence V1 intégrés avant consommation.
Un succès n'établira pas que tous les mécanismes du sandbox Chrome fonctionnent.

Le document architecte a été copié octet pour octet, puis ratifié. L'auteur existant
prépare le script ponctuel et ses tests de projection, sans modifier les sources
k6 existantes, le conducteur ou ses trois correctifs sous revue. Pas de nouveau
framework, worker, modèle ou politique QA.

Une lecture de métadonnées sous **bail neuf** a retrouvé les outils installés et
le snapshot Linux887b. Les quatre fichiers chargés par le futur script ont des
empreintes identiques au canon Windows. Compte Linux1000 et HOME privé/ext4
contrôlés. Ces lectures ne lancent ni `unshare`, ni `ldd`, ni Chrome, ni le conducteur.

## Runtime et limites

L'instance AIForAll, observée arrêtée avant la lecture, a été réveillée pour cette
seule opération. L'invocation est jointe ; l'instance était brièvement encore en
marche au premier inventaire, puis observée arrêtée lors de la libération. Aucun
arrêt explicite n'a été nécessaire. ARTIST-Ubuntu reste arrêtée ; docker-desktop
reste actif et étranger à cette opération. Données et outils conservés ; aucun
arrêt global ou effacement. La RAM globale n'est pas mesurée ni revendiquée libérée.

Les commandes de sonde restent **non exécutées**. Le script sera relu et ses
fixtures contrôlées avant un nouveau bail, sans adopter le bail de découverte.
Comparer le contexte du parent WSL et celui de Child ne constituera pas un test
des commandes dans deux terminaux ; le terminal interactif utilisateur n'est pas
émulé. Linux natif et Windows restent des replis non démarrés.

La collecte privée et le préflight du conducteur restent interdits selon leurs
conditions propres. Aucun changement des droits du disque, pin Chrome, AppArmor,
sysctl, namespace persistant ou permission CI. HTTP et Chromium non qualifiés.

## Annexe technique

- Contrat : `docs/contracts/k6-chromium-linux-probes.md`, Accepted/Unimplemented.
- Admission locale : commit `b04311f`, neuf fichiers documentaires, hook normal
  `cargo fmt` et `git diff --cached --check` exit0. Aucun push ou dispatch CI.
- Candidat architecte : `result_mvv52baa_0000_eufwkd`, SHA256 avant ratification
  `00401fd3cb3b653cc8ba77dc171b8244a14a957dbf315fbc2ec5b3e10bdee7f0`.
- Bail de découverte `.tmp/k6-linux-probes-discovery-20261030-695facd4f44646e2942768f771fb154f.json`,
  libéré, consommateurs vides, exit0, identité Windows recontrôlée ; registre WSL
  contemporain append-only. Le GUID possédé reste celui du guide WSL.
- Script metadata ponctuel : `.tmp/k6-linux-probes-discover-20261030.py`, AST valide
  avant exécution ; contrôleurs metadata/retrait JS syntaxiquement validés avant
  usage. Un premier appel Code Mode a été refusé pour syntaxe imbriquée et n'a
  exécuté aucune commande ; les scripts fichiers évitent ce problème d'échappement.
- Configuration de chemins observés transmise à l'auteur via
  `.tmp/k6-linux-probes-paths-20261030.json` ; ce fichier n'est pas un bail ni
  une attestation suffisante à réutiliser sans recontrôle.
- Source gelée : quatre modules/lock identiques au canon,22fichiers hors correctif
  inchangés. Correction conducteur : Windows194/16.998s/exit0/skips8 ; revue bornée
  en cours, pas de preuve Linux du correctif ni de fermeture présumée.
