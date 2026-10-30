# Vérification indépendante — provisionnement AIForAll

- Profil : infra-verifier, contexte fresh READONLY.
- Verdict : **PASS borné au provisionnement WSL**, aucun défaut bloquant observé.
- Base : `05bd6c63d0c86f090b27c9dbb48217f04b07d5e6`.
- Contrôles :2026-10-30,11:15:49–11:17:44 UTC.
- Candidat : `result_mvuvpc2f_0025_1ovygc`, session
  `ses_e737af0a0ffe3C5Z6YbBJMaQBq`, tâche `task_mvuvkxul_0024_2df5eo`.
- Rapport READONLY persisté par le lead ; aucun fichier modifié par le verifier.

## Résultats indépendants

- Registre Lxss, exit0 avant/après : GUID `{22ce910b-45a3-42a4-bbab-dc56a1766035}`,
  nom AIForAll, BasePath I:\WSL\AIForAll, WSL2, defaultUID1000. Trois registrations
  seulement, aucune supplémentaire inattendue. Défaut global conservé au GUID
  `{bb000c3f-6fcc-4ca1-859e-6052e02d67e0}`.
- Disque/ACL, Get-Item/Get-Acl exit0 : VHDX1925185536octets, aucun reparsepoint,
  propriétaire Windows courant et DACL protégées limitées au propriétaire,
  SYSTEM et Administrators, FullControl, sur les deux seuls objets possédés.
- Métadonnées Linux par GUID et Python non-root, exit0/stderr0 : Ubuntu24.04/x86_64,
  UID/EUID/GID1000 sans--user, utilisateur aiforall, HOME/home/aiforall owner1000,
  mode0700 et non symbolique, racine ext4.
- Dpkg-query des28paquets : exit0/stderr0,28entrées `ii`, versions concordantes
  avec le manifeste fermé. Pas de réinstallation.
- JSON historique de bootstrap relu : refresh0/simulate0/install0,28demandés,
  stderr vide. Script SHA256
  `23da9997e786de8d555172b7b00b48b82ffdaf6192d245bb4747f0d65178218e` concordant.
- Sources apt relues non-root : ubuntu.sources seul, hôtes archive.ubuntu.com et
  security.ubuntu.com, suites noble attendues et clé Ubuntu ; aucun Trustedyes,
  source classique active, `.list` supplémentaire ou option d'auth désactivée.
- Image officielle rehachée :388975696octets/SHA256
  `bb415d824822c4b878125729af451a5d18fb13d1cf5cbed9a7393ad64ac6039e` ;
  SHA256SUMS Ubuntu recoupé indépendamment par HTTPS, HTTP200, correspondance exacte.
- `git diff --check` exécuté : exit0, sorties vides.

## Runtime avant/après et limites

WSL-list exit0 aux deux captures :11:15:56 AIForAllStopped/ARTISTStopped/
docker-desktopRunning ; les lectures Linux ont réveillé le seul GUID autorisé ;
11:17:43 AIForAllStopped/ARTISTStopped/docker-desktopRunning. Aucune commande stop
par le verifier, mécanisme du retour à Stopped non déterminé. Parent responsable
du registre, du bail et de l'arrêt final.

L'historique des ACL du parent I:\WSL est **INCONCLUSIVE**, faute de snapshot avant
indépendant ; aucune mutation du parent par le verifier. Cela ne contredit pas
la portée des commandes qui n'avaient ciblé que les nouveaux objets.

Pas de navigateur, k6/Chromium-version, unittest, inspect, validate/run/browser/
selftest, relevé privé, Cargo, CI, Docker, Kind ou accès interne aux distros étrangères.
HTTP, Chromium/sandbox et qualification k6 restent INCONCLUSIVE. Les140Linux
historiques ne qualifient pas cette instance. Aucun PASS global ni admission du
relevé privé ; aucune délégation, correction, écriture ou arrêt par le verifier.
