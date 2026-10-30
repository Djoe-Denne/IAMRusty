# Reprise ciblée : isoler les restrictions Linux et les dépendances

L'objectif reste de faire démarrer Chrome avec son sandbox, avant qualification
du navigateur k6. L'utilisateur demande explicitement de reprendre à partir de
ses recherches, après sa pause. Les deux pistes prioritaires sont une restriction
de création d'espaces de noms et des bibliothèques Linux manquantes. Les espaces
de noms sont un mécanisme d'isolation des processus, utilisé par le sandbox.

## Ce qui change

La pause est levée pour cette investigation. Pas de nouvelle tentative Chrome,
changement de version, logs verbeux ou désactivation de sécurité. L'hypothèse
AppArmor reste moins étayée sur notre WSL où il a été observé désactivé. Le
signalement Claude Code décrit un client différent, pas une preuve sur notre hôte.

Le document officiel Chromium est disponible pour la version156 épinglée et pour
main. Les signalements publics fournis ont été retrouvés. Après un premier
timeout Defuddle, la page Canonical a été obtenue via Context Mode : elle confirme
qu'AppArmor est installé mais pas activé par défaut sous Ubuntu WSL. Ce fait
général ne remplace pas les observations de l'hôte.

Le catalogue Google officiel, horodaté2026-10-30T07:16:02.827Z, contient les deux
versions156.0.8078.73 et156.0.8078.80 pour linux64, même révision1706539. Cela ne
prouve ni leur identité binaire, ni un correctif, ni à lui seul le canal Stable.
La version.73 reste épinglée ; aucun nouvel exécutable n'est téléchargé.

## Prochaine étape et bornes

En parallèle de la vérification des sources, l'architecte examine la compatibilité
d'un diagnostic ponctuel sans navigateur : lecture des versions réelles, création
bornée d'un espace de noms, et recherche locale de bibliothèques manquantes.
Les résultats seraient des états fermés, jamais un journal natif publié. Les
mêmes mécanismes de lancement seront comparés sans prétendre simuler un véritable
terminal interactif de l'utilisateur.

La collecte privée et le préflight du conducteur restent interdits tant que leurs
conditions propres ne sont pas résolues. Le correctif local non committé est
préservé. Ses trois fichiers ont été comparés octet pour octet à l'auteur et ses
22autres fichiers gelés à la source intégrée. La suite Windows corrigée passe
194tests, huit skips POSIX,186exercés, en16.998s, exit0. Le premier lancement avait
échoué parce que la variable TMP manquait dans Context Mode ; il n'est pas
qualifié de test ROUGE du produit. La revue indépendante des deux corrections
est reprise, sans conclusion anticipée. Aucun runtime n'est encore démarré pour
cette reprise.

Inventaire sans réveil : AIForAll arrêtée, ARTIST-Ubuntu arrêtée, docker-desktop
déjà actif et étranger à cette opération. WSL2.6.1.0, noyau annoncé par l'outil
Windows6.6.87.2-1 ; le futur `uname` Linux restera à mesurer. Identité de la seule
instance AIForAll recoupée par GUID, nom, emplacement, UID1000 et WSL2. Anciens
baux sans consommateurs, non adoptés ; aucun bail runtime nouveau encore réservé.

Linux natif est le repli envisagé par l'utilisateur si cette enquête ne débloque
pas la situation. k6 natif Windows serait un essai distinct, non une preuve Linux ;
ni lui ni Kind/Docker ne sont lancés maintenant. Aucun nouveau GO de jalon n'est
demandé pour les diagnostics conformes ; une migration n'est pas présumée acquise.

## Références

- [Contexte Chrome et limites](63-contexte-partageable-du-demarrage-chromium.md).
- [Pause précédente, conservée comme historique](64-pause-demandee-pour-recherches-personnelles.md).
- Brief de cette reprise : `.cursor/review-briefings/20261030-k6-userns-dependencies-research-followup.md`.
- HEAD vérifié : `0f32e8dab504458074a353cb555139801db81487` ; trois fichiers
  correctifs non committés inchangés ; `git diff --check -- ops/tests/k6` exit0.
  Aucune preuve de runtime, de navigateur ou de nettoyage nouveau n'en découle.
- [Chromium156 et restrictions AppArmor](https://chromium.googlesource.com/chromium/src/+/156.0.8078.73/docs/security/apparmor-userns-restrictions.md).
- [Sécurité Ubuntu WSL — Canonical](https://ubuntu.com/wsl/docs/latest/explanation/security-overview/).
- [Signalement de restrictions dans le sandbox Claude Code](https://github.com/anthropics/claude-code/issues/103811).
- [Catalogue Google consulté](https://googlechromelabs.github.io/chrome-for-testing/known-good-versions-with-downloads.json).
- [Manuel unshare](https://man7.org/linux/man-pages/man1/unshare.1.html) et
  [précautions ldd](https://man7.org/linux/man-pages/man1/ldd.1.html) relus :
  aucun ldd d'exécutable non fiable, aucun faux succès par `grep ... || echo`.
