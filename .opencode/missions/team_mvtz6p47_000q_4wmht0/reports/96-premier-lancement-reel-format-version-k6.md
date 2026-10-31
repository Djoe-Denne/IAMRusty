# Premier lancement réel : format de version k6 à corriger

## Objectif et résultat concret

Exécuter sous Windows le Chrome épinglé sur `about:blank`, vérifier42 et le fermer.
Le code simplifié est intégré ; le premier lancement réel a été effectué. Il s'est
arrêté avant Chrome parce que le lecteur de version attendait `k6`, tandis que le
binaire Windows se présente comme `k6.exe`. Ce n'est ni une panne de Chrome ni un
problème de sandbox.

La sortie réelle est `k6.exe v2.3.0 (commit/e088784614, go1.26.8, windows/amd64)`.
k6 a quitté avec le code0 et sans erreur. Le parent a refusé ce format avec exit2 /
`VERSION_INVALID`. Les inspections et le cas navigateur n'ont donc pas été exécutés.

## Preuves vérifiées et portée

- Les9fichiers sont comparés byte-identiques au candidat auteur, puis intégrés
  via l'outil patch. La première transcription de patch a été rejetée sans modification
  des sources ; récupération corrigée par mouvements de copies d'intégration vérifiées.
- `git diff --check` exécuté :exit0. Les quatre SHA256 des archives/exécutables
  k6 et Chrome ont été revérifiés et correspondent au manifeste épinglé.
- Le premier mode browser dure1067ms, parent exit2, stderr parent vide.
- Le journal privé de l'enfant confirme : exit0, raisonNONE, capture complète,
  job total1/active0, une identité observée terminée, image égale au k6 attendu.
- Inventaires Windows avant/après :0/0 k6 ou Chrome ; aucun processus nouveau restant.
  CleanupPASS et confidentialitéPASS. Aucun profil Chrome créé par ce cas.
- Trois fichiers privés bornés de diagnostic sont volontairement conservés, pas
  publiés. Les autres données du run sont retirées. Aucun Docker/WSL/Kind réveillé.

## Suite autonome

Le même auteur corrige uniquement le préfixe du lecteur et ajoute un test de
régression avec la sortie réelle, en conservant la version exacte et les autres
contrôles. Le lead relancera ensuite le même cas minimal dans un nouveau répertoire.
Aucune nouvelle protection, revue préalable ou décision utilisateur n'est nécessaire.

Les32tests de l'auteur, réussis avant ce run, ne couvraient pas ce préfixe. Leurs deux
passages ne sont pas64tests distincts. Chrome, HTTP/selftest, WSL et Linux restent
non qualifiés ; aucune réussite globale n'est annoncée.

## Références techniques

Candidat `result_mvw6lbki_0000_4m8tgj`, patch103873octets / SHA256
`18330fbf8f1ea473163c11ad64b047bde739d10598f75ee6bb4a42ffc36ee559`.
Base `a5a71fe` + delta source intégré. Preuve privée :
`.tmp/k6-browser-r3-20261031-lease.json` et diagnostics associés, exclus de publication.
Brief de correction : `.cursor/review-briefings/20261031-k6-windows-version-prefix-fix.md`.

La simplification et ce diagnostic sont committés dans `7e3415f`, avec hook normal
de formatage et contrôle du diff réussis. Le même auteur est repris sur la seule
correction du préfixe. Les quatre inventaires privés des deux suites de l'auteur
ont été recoupés :22identités sur22observées terminées et quatre répertoires de
fixtures absents. Les journaux bruts ne sont pas publiés.

Préparation indépendante de la publication : le workflow porte bien le nom
SUSPENDED et les deux jobs ont leur condition littérale fausse. La référence distante
master reste `ca0c994fa72203f38270f229125b73acb818461a`, relue sans push ni dispatch.
