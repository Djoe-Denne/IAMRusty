# Composition privée ratifiée ; code et tests autorisés

## Étape atteinte

Le contrat précise désormais la voie retenue : le contrôleur Windows conserve
l'identité et le handle de son unique processus WSL ; le compagnon Linux garde
la session privée ouverte jusqu'au retrait. L'amendement de l'architecte a été
récupéré à l'identique, sauvegardé et ratifié avant code.

Les pipes ne portent que des métadonnées fermées et bornées, jamais le site ou
le corps du diagnostic. Les contrôles Windows sont renouvelés à chaque étape,
avec un budget de 30 secondes mesuré séparément sur chaque système. Les messages
étrangers, incomplets ou tardifs sont refusés. La fin du processus Windows ne
suffira pas à déclarer le nettoyage Linux réussi.

## Suite sans nouvel essai Chromium

L'auteur existant reprend le code et les tests dans sa même session, avec une
transition de spécialité native adaptée au travail Windows/Linux. Le modèle et
la politique approuvée restent inchangés. Trois fichiers de conducteur et tests,
plus une documentation courte, sont prévus ; le consommateur déjà testé ne change pas.

Après intégration, une revue ciblée examinera la composition réelle. Un préflight
des observateurs pourra ensuite être exécuté sous un bail distinct, sans créer
de session privée, lancer k6/Chromium ou écrire un site. Le vrai rendez-vous de
lecture sera convenu seulement lorsque le conducteur sera vérifié.

**La collecte reste désactivée.** Aucun nouveau runtime ou processus WSL n'a été
lancé pendant cette préparation. Les 165 tests Linux et la revue close restent
des preuves du consommateur existant, pas du conducteur encore absent.

## Preuves techniques

- Amendement architecte `result_mvv0mhui_0002_p982l8`, +55/-27 lignes, limité à la
  section opérationnelle ; diff-check auteur et lead exit 0, récupération exacte.
- Sauvegarde vérifiée : `.tmp/k6-real-parent-pipes-proposed-dd60256a68e040cf8fd3999fcb8c4a6c.md`.
- Contrat : `docs/contracts/k6-chromium-causal-observation.md`, amendement pipes
  Accepted pour implémentation/vérification, conducteur Unimplemented.
- Brief complet : `20261030-k6-real-parent-pipes-implementation.md` ; source du
  consommateur `cbab06e` gelée, workflow et pins inchangés, aucun push déclenché.
