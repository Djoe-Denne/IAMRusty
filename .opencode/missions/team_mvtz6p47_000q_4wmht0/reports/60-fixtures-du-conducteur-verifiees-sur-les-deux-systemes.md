# Fixtures du conducteur vérifiées sur les deux systèmes

## Résultats distincts

Les tests du code intégré réussissent sur les deux systèmes :

- **Windows : 189 tests, exit0, 8 tests réservés à Linux non exécutés** ;
  181 tests exercés, dont les 11 tests PowerShell natifs.
- **Linux : 189 tests, exit0, 11 tests PowerShell réservés à Windows non exécutés** ;
  178 tests exercés, y compris les tests POSIX.

Les exclusions sont donc explicites et complémentaires. Elles ne valent pas un
PASS universel, et aucun résultat de fixture ne remplace une véritable invocation
du conducteur ou la preuve des observateurs de l'hôte.

## Conditions encore ouvertes

La revue indépendante du conducteur est en cours. Aucun préflight réel de ses
observateurs, session armée, lancement Chromium ou site inconnu n'a été exécuté.
Après la revue applicable, le préflight non armant demandera son propre bail et
ses profils privés fondés sur les observations, pas un inventaire historique.

Le rendez-vous de lecture locale reste à convenir ultérieurement. HTTP, Chromium
et cleanup parent restent non qualifiés ; aucune publication source ou relance CI
n'a été engagée par ces tests.

## Preuves et ressources

- Outillage `195059a`, snapshot détaché propre
  `887b6b001f66a6867a84b01217ea8a339ba9729c`. 28 fichiers outillage/workflow/
  contrats comparés par SHA256 avant et après transfert sur ext4, aucun WIP
  étranger ou sous-module initialisé.
- Tests Linux réellement exécutés sous UID/EUID1000 : Python3 natif, unittest
  discover, Child existant et environnement fermé : **189 en 2,013 s**, exit0,
  skips11, diagnostic nul, cleanup des enfants réussi ; 4 identités tracées,
  aucune encore active. Résultat fermé local `native-unit-result.json` dans
  la campagne `k6-20261030-887b6b0`.
- Nouveau bail contemporain des seuls tests :
  `.tmp/k6-parent-fixtures-20261030-467fd91230b64a70a676039a434136cb.json`.
  Consumers libérés ; inventaire d'installation antérieur non adopté comme bail.
- À la fin, AIForAll est déjà Stopped : **aucun stop explicite exécuté**.
  ARTIST-Ubuntu Stopped et Docker Desktop Running sont inchangés par rapport
  à l'inventaire initial. Disque/outils/sources conservés, registre WSL actualisé.
  Pas de ressource Docker créée, suppression, arrêt global ou RAM totale annoncée.
- Le reviewer n'a pas été présenté comme ayant exécuté ces tests ; sa revue reste
  une preuve séparée à recevoir et à persister avant correction éventuelle.
