# Socle k6 intégré — revue indépendante avant qualification Linux

## Objectif et étape

Les scénarios, le lanceur, la configuration épinglée, les tests et le workflow Linux sont intégrés. Cela livre le socle pour examen ; la mission n'est pas qualifiée ni clôturée. HTTP reste fermé faute de cible publique autorisée, Chromium n'a pas encore été exécuté sous Linux.

## Résultats vérifiés

Quatorze fichiers nouveaux ont été récupérés et sauvegardés avec empreintes identiques depuis le worktree auteur. Aucun fichier préexistant écrasé. Le lead a rejoué les tests purs au checkout principal :37 tests,32 exercés et5 skips POSIX, exit0. Le commit ciblé a passé son véritable hook de formatage. Aucun WIP étranger inclus.

L'auteur distingue explicitement une configuration admissible hors ligne d'une autorisation live. Ce point reste à examiner par une revue indépendante du diff, avec les sorties, les processus enfants, la sandbox et les permissions CI. Les tests de parsing et les fakes ne sont pas des preuves HTTP/Chromium.

## Suite

Revue fresh ciblée du diff intégré, puis correction dans la même session auteur si nécessaire. Le lead vérifie en parallèle la provenance du navigateur et prépare la qualification Linux sans démarrer de runtime par anticipation. Les preuves Linux réellement exécutées seront rapportées séparément ; absence de cible = HTTP INCONCLUSIVE, pas PASS global.

## Annexe technique

- Base3f2e8fb, commit source intégrédb16aa726ca0f44876180ac45b4922110c030c09 sur master,14 fichiers/1904lignes nouvelles. Pas de push.
- Sauvegarde vérifiée .tmp/k6-tooling-candidate-9decf1c9e176446a9e5cabd76adcca0e, manifest de SHA256 des14fichiers.
- Auteur session ses_e76907ccbffet7VF3cPr8KCFXV/worktree pool-4 conservé pour corrections ; branche sans commit nouveau à ce stade, source capturée au principal avec disposition explicite, pas fusion de commit auteur revendiquée.
- Windows natif : C:/Python313/python.exe -B -m unittest discover -s ops/tests/k6/tests -p test_*.py, exit0,37tests/skipped5. Python3 Linux et cinq tests POSIX non exécutés.
- Diff staged ciblé14fichiers contrôlé et hook cargo fmt --all -- --check exit0 lors du commit.
- Briefing fresh : .cursor/review-briefings/20261029-k6-functional-smoke-security.md.
- Aucun k6/Chromium lancé, aucun runtime/pull/ressource Docker créé ou adopté, aucune preuve CI acquise.

## Actualisation — revue admise et provenance recoupée

La revue indépendante est démarrée sur le diff intégré ; l'auteur reste disponible dans sa session pour les éventuelles corrections. Aucun nouveau modèle ni pipeline de rôles ajouté.

Le lead a acquis indépendamment les deux archives aux URL officielles fixes en mémoire, vérifié leurs SHA256 et ceux des exécutables ELF sans les exécuter, et recoupé les métadonnées Google versionnées. Les quatre empreintes correspondent au lock. Cela apporte une preuve indépendante de provenance/octets ; aucune version retournée par binaire, compatibilité Linux ou sandbox n'est encore attestée. Les sources n'ont pas été modifiées pendant la revue.
