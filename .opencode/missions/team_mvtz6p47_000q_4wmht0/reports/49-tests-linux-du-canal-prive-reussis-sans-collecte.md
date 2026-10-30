# Tests Linux du canal privé réussis, sans collecte

## Résultat vérifié

Les **162 tests réussissent sous Linux sans aucun skip**, sur une copie propre
du même code intégré que celui soumis à la revue. Le test POSIX utilise seulement
un emplacement source fictif connu et vérifie les permissions et le retrait de
son fichier temporaire. Ce n'est pas une admission d'hôte, de lecteur ou de capture.

La vérification Windows précédente reste valide à sa portée :162tests réussis,
8tests POSIX non exécutés. La vérification Linux comble cette limite d'exécution,
mais n'identifie toujours pas la cause de la panne Chromium.

## Ce qui reste avant activation

La revue indépendante du canal est en cours. Toute correction attendra son
rapport persisté. Un vrai contrôle parent de l'hôte et des preuves, ainsi qu'un
créneau de lecture locale convenu avec l'utilisateur, restent indispensables.
Aucun callback de fixture ne sera utilisé pour une capture réelle.

Aucun Chromium, requête applicative ou site natif inconnu n'a été exécuté ou
collecté pendant ces tests. Aucun changement de sandbox ou qualification
fonctionnelle n'en découle. La mission reste ouverte, HTTP et Chromium non qualifiés.

## Ressources et preuves

- Snapshot détaché propre `eb3d35530cb4d0b3e703c593b868c8047a512a6d`,
  outillage `968592f`,25fichiers comparés avant et après transfert sur ext4.
- Python3 natif non-root, unittest discover, supervision Child existante :
  **162/0.879s/exit0/0skip**, diagnostic nul, cleanup réussi ;5identités tracées,
  aucune encore active. Résultat fermé local native-unit-result.json dans la
  campagne `k6-20261030-eb3d355`.
- Réveil du seul GUID AIForAll après vérification de son identité et réservation
  du bail. À la libération, l'instance est déjà Stopped ; aucune commande stop
  n'a donc été exécutée. ARTIST-Ubuntu Stopped et Docker Desktop Running inchangés.
- Disque, outils et sources conservés ; pas de ressource Docker créée, suppression,
  arrêt global ou mémoire WSL totale revendiquée comme libérée.
- Revue `task_mvuykugk_002f_m27x96` encore attendue ; aucun PASS de revue annoncé.
