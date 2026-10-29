# Deuxième essai sandbox non concluant — analyse spécialisée sans nouveau lancement

## Objectif et résultat

Le deuxième essai natif réussit encore les tests, l'acquisition épinglée, les versions et les inspections. Chromium reste INCONCLUSIVE avec exit2 et cleanup des enfants PASS. Le nouveau diagnostic est « cause native non classée » : il ne mesure pas la cause de l'échec et n'autorise pas à accuser AppArmor. Aucun artefact n'est publié.

## Décision

Deux tentatives de qualification n'ont pas fourni la preuve navigateur ni une cause vérifiée. Le lead transfère une analyse bornée au spécialiste de récupération, en lecture seule. Pas de troisième répétition, de nouveau runtime ou de modification des règles de sécurité. Une nouvelle expérience exigerait une hypothèse différente, étayée et discriminante, dans le contrat existant ; sinon les prérequis manquants seront exposés précisément.

L'outillage déjà vérifié reste utilisable et publié. HTTP demeure fermé, indépendamment du navigateur. La mission n'est pas clôturée, même si la publication distante a réussi.

## Références

- [Second essai natif](https://github.com/Djoe-Denne/IAMRusty/actions/runs/40965632254), commit4e82a36, failure ; browser SANDBOX_UNPROVEN/NATIVE_CAUSE_UNCLASSIFIED, publication skipped.
- Briefing diagnostic `.cursor/review-briefings/20261029-k6-browser-two-attempts.md`.
- Logs privés des watches `.tmp/k6-ci-watch-40963859987.log` et `.tmp/k6-ci-watch-40965632254.log` ; pas de log brut committé.
- Aucun paquet, wrapper, sysctl/AppArmor ou flag sandbox modifié ; WSL local arrêté, Docker étranger préservé.
