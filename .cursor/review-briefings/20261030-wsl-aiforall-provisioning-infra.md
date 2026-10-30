# Vérification indépendante bornée — nouvelle instance WSL de projet

## Mandat et canon

READONLY sur l'hôte AIForAll créé explicitement à la demande utilisateur sur I:.
Contrat `docs/contracts/k6-chromium-causal-observation.md` : **provisionnement
Accepted**, relevé causal et QA de son canal **Proposed/non admis**. Lire aussi
`docs/wsl-aiforall.md`, rapport43 et ledger canoniques. Aucun changement modèle ou
QA ; cette vérification est utile au changement infra, pas une qualification k6.

Identité créée par delta natif : GUID **{22ce910b-45a3-42a4-bbab-dc56a1766035}**,
nom **AIForAll**, BasePath **I:\WSL\AIForAll**, WSL2, fichier ext4.vhdx. Seule cette
instance est dans le périmètre. ARTIST-Ubuntu et docker-desktop sont étrangers ;
ne pas les lancer, sonder de l'intérieur, modifier ou arrêter. Default WSL préexistant
à préserver : GUID {bb000c3f-6fcc-4ca1-859e-6052e02d67e0} (docker-desktop).

Image officielle Ubuntu24.04.5/amd64388975696octets, SHA256
bb415d824822c4b878125729af451a5d18fb13d1cf5cbed9a7393ad64ac6039e vérifié avant
installation. Premier nom avec espaces refusé avant création ; user a ensuite
choisi AIForAll. Aucune branche/donnée d'ancienne distro importée.

## Contrôles indépendants attendus

- Windows registry GUID/nom/BasePath/version2/defaultUID1000 et disque sur I:
  concordants ; aucune nouvelle registration inattendue. Défaut global conservé.
- ACL des seuls nouveaux dossier/VHDX : propriétaire Windows courant, SYSTEM et
  Administrators, pas de principal plus large/reparsepoint. Ne pas modifier ACL.
  Le parent I:\WSL n'a pas été ciblé par le bootstrap ; ne pas prétendre prouver
  son historique d'ACL sans snapshot avant.
- Métadonnées **non-root** par `wsl --distribution-id <GUID> --exec /usr/bin/python3 ...` :
  Ubuntu24.04/x86_64, UID/EUID/GID1000, HOMEaiforall700/owner1000, rootFSext4.
  Pas d'usage de compte root par le verifier, pas de lecture shadow ou de secrets.
- Les28paquets demandés et les étapes refresh/simulate/install exit0 sont attestés
  dans `.tmp/wsl-aiforall-packages-20261030.stdout.log` (JSON fermé de package
  versions, pas un log natif navigateur). Vérifier les paquets réellement installés
  par requête dpkg bornée si utile. Source-hosts déclarésUbuntu officiels et code
  de bootstrap temporaire consultable, pas de flagsd'auth désactivée/repo ajouté.
- Distinguer création/provisionnement et qualifications absentes : aucun k6,
  Chromium, smokeHTTP, relevé privé ou test de sandbox exécuté dans ce nouvel hôte.
  Ne pas lancer ces consommateurs ni déduire un PASS global depuis les140Linux
  historiques. Ne pas les rejouer pour remplir le rapport.

## Baux et permissions

Le lead détient le bail actif de vérification/provisionnement de **ce GUID**,
`.tmp/k6-dedicated-wsl-20261030-mvuqe1yi.json`, registre contemporain
`.tmp/wsl-resources-20261030.log`. Les lectures de métadonnées sur ce GUID sont
autorisées pour cette vérification ; si elles démarrent l'instance arrêtée,
relever avant/après et le dire au lead, sans adopter une autre ressource. Le parent
persiste les deltas et garde l'arrêt final ; **aucun stop, installation, écriture,
cleanup, Docker/Kind, Cargo, CI ou délégation** par ce verifier.

Retour PASS/FAIL/INCONCLUSIVE à la portée réelle, avec commandes/exits, preuves,
écarts bloquants séparés des limites. Aucun contenu de site natif inconnu n'existe
ici ; ne pas créer ce relevé ou examiner des buffers privés. Fresh context, pas
les conclusions d'auteur comme oracle. Geler le retour READONLY au lead.
