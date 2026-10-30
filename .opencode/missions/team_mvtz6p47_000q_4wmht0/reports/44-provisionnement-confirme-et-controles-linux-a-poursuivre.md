# Provisionnement confirmé ; poursuivre les contrôles Linux

## Résultat

La vérification indépendante confirme le provisionnement de **AIForAll sur I:**,
sans défaut bloquant observé : identité et disque attendus, permissions privées,
compte non-root, paquets réellement installés et sources Ubuntu conformes.
L'image a été rehachée et recoupée avec la source officielle par le verifier.

Les autres distributions et le défaut global sont inchangés. Les lectures du
verifier ont réveillé seulement l'instance autorisée ; elle était de nouveau
arrêtée à sa capture finale, sans commande d'arrêt. Le disque et les outils sont
conservés conformément à votre demande.

## Suite autorisée

Le lead poursuit sans nouveau GO : copier le snapshot propre déjà vérifié sur le
FS Linux natif privé, y vérifier l'intégrité et le GitHEAD, puis acquérir/contrôler
les outils k6 et Chromium épinglés avant un essai navigateur borné. La copie Windows
est prête et propre ; aucun WIP ni sous-module n'est importé.

Ce PASS est **celui du provisionnement**, pas du navigateur. Les140tests GitHub ne
se transfèrent pas au nouvel hôte. Les contrôles réels de cette instance et leurs
résultats seront distincts ; le relevé source privé reste non admis, sans collecte
ou publication de diagnostic natif inconnu.

## Preuves et limites

- Résultat complet persisté :
  `.cursor/review-briefings/20261030-wsl-aiforall-provisioning-infra-result.md`.
- Revue sur HEAD05bd6c6, candidat `result_mvuvpc2f_0025_1ovygc` ; commandes
  Get-ItemProperty/Get-Acl/dpkg/Python non-root/WSL-list et diff-check exit0.
- VHDX1925185536octets à la mesure du verifier, pas une taille de disque virtuel
  maximale ;28paquets demandés `ii`, versions concordantes.
- Historique des ACL du parent I:\WSL non prouvé indépendamment, aucun snapshot
  antérieur disponible. Pas d'anomalie démontrée ni de mutation par le verifier.
- Snapshot Windows neuf : HEADdetached05bd6c6/statusclean,20fichiers d'outillage
  byte-vérifiés. Copie Linux et commandes applicatives pas encore exécutées.
- Aucun build/test Rust Linux, Docker, Kind ou nouveau déploiement. Mission non
  clôturée ; HTTP et qualification Chromium restent ouverts.
