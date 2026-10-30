# Sources Linux et outils vérifiés avant l'essai navigateur

## Objectif et étape atteinte

La mission doit qualifier un vrai Chromium Linux sans affaiblir sa sandbox.
L'instance dédiée AIForAll dispose maintenant d'une copie propre des sources sur
son disque Linux natif. Les **140 tests Python y réussissent sans aucun skip**.
Il s'agit de résultats locaux nouveaux, pas d'une reprise des preuves GitHub.

Les deux outils ont été acquis explicitement dans un dossier neuf. Archives et
exécutables correspondent aux empreintes épinglées ; les binaires annoncent
réellement **k6 2.3.0** et **Chrome for Testing 156.0.8078.73**. Les inspections
des deux scénarios réussissent. Ces commandes ne lancent aucun cas navigateur
ni requête vers l'application. Le nettoyage des enfants réussit à chaque étape.

## Suite retenue

Le lead poursuit avec **un essai navigateur borné sur ce nouvel hôte**, comme
utilisateur non-root, avec le scénario et les protections existants inchangés.
Ce nouvel environnement et ses dépendances constituent une expérience distincte
du runner GitHub. Aucun succès local ne sera rétroattribué à la panne GitHub.

L'essai ne collecte ni emplacement source privé ni journal natif. Le canal privé
reste Proposed et non implémenté. Aucun appel HTTP applicatif, déploiement,
changement AppArmor, argument de contournement ou nouvelle reconnaissance n'est
ajouté. Il n'y a pas de nouvelle demande de GO de jalon.

## Preuves et limites

- Source détachée `05bd6c63d0c86f090b27c9dbb48217f04b07d5e6`, propre ;
  23 fichiers comparés par SHA256 avant et après transfert, sans WIP ni sous-module.
- UID/EUID1000 ; données sous `/home/aiforall/.cache/opencode/k6-20261030-05bd6c6`.
- Tests : Python3 natif, unittest discover, 140/0.805s/exit0/0skip ; supervision
  Child existante, environnement fermé et fichiers temporaires privés.
- Acquisition : provision-linux.py exit0/VERIFIED_NOT_EXECUTED, cleanup des enfants
  attesté ; outils conservés selon la demande utilisateur.
- Commande tools : exit0, versions réelles et deux inspections réussies, provenance
  liée au même HEAD ; nettoyage de la commande et de sa supervision réussi.
- Résultats fermés locaux : native-unit-result.json, native-acquisition-result.json
  et native-tools-result.json. Aucune sortie native brute publiée.
- HTTP et selftests réels toujours sans cible conforme autorisée ; Chromium pas
  encore qualifié. Le runtime reste sous bail pendant l'essai actif et sera
  arrêté de façon ciblée lorsqu'inutile, sans toucher aux distributions étrangères.
