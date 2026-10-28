# Rapport 77 — correctif testé, formatage C-1 à terminer

## Preuves finales recoupées

Lead lit les logs/JSON réels pool11 `.tmp/http-c1-rp-fix-evidence/` :
- `t1_t2_final` :2PASS/0ignored, source owned SHA3bb306aa..., T1Err→Err→Err avec RAII1 après récolte et mutation0 ; T2destruction indépendante avant release, forcedtrue,mutation0.
- `owner_regressions_final` :8PASS/0ignored sur la même source, grâce/force/joinrepris/body/stream/erreurcoordinateur/Drop/keepalive/H2 ; panic volontaire injectée observée, pas échec du test.
- commit SDK local **b7bfbc7afedf570e0c303567dae5064fa09c9511**, parent426dc24, arbre42249412046327299dbbcabb479e6a156d81c52a, SDKstatuspropre ; `check_exact_commit` exit0 et `owner_exact_commit` **10PASS/0ignored** sur ce SHA/source exacts. Aucun pin/publication ni fermeture indépendante des findings encore acquis.

## Attribution précise du formatage

Rustfmt ciblé owned_server strict exit0. Fmt global SDK exit1, diff uniquement HTTP lib.rs. Lead compare129dcc1→426dc24 : les deux lignes export/module owned_server incriminées ont été introduites par **le sous-lot HTTP de cette mission**, pas une dette utilisateur historique. Le worker a correctement restauré son seul effet de formatter hors scope privé ; cela ne rend pas le défaut de formatage acceptable comme dette ancienne de la plateforme.

## Décision mécanique bornée avant publication

`rustycog-http/src/lib.rs` et ses exports sont déjà explicitement dans la surface autorisée du contratHTTP§2. Autoriser au même auteur une **correction de formatage uniquement**, séparée du commit b7 : ordre/reflow exigés par rustfmt, ensemble des exports/signatures inchangé. Pas nettoyage de la dette Clippy registry, d'autres modules ou des tests, pas changement fonctionnel/API ou extension QA.

Conserver le bundle/preuves b7. Vérifier hash owned_server et suffixe tests inchangés après cette mécanique ; fmt global et check ciblé sur le SHA final. Les10 tests de logique restent réutilisables si leur source est byte-identique ; pas rerun10/16+10 pour remplir le rapport. Revue indépendante sur diff final et intégration exacte avant publication/pin, C-1 toujours ouvert.

## Remise b7 conservée et directive distincte

Remise b7 complète reçue :27artefacts+source/manifeste archivés dans `.tmp/http-c1-rp-fix-delivery-evidence/`, tous hashes vérifiés, tests suffixeSHA094809d9... identiques426. Une erreur d'échappement du script d'archive lead corrigée avant exécution réussie, pas panne MCP. Aucun source/log b7 modifié.

Les retours « déjà livré b7/fmtbaselineNONVERT » précèdent la décision mécanique. Le lead fournit copie locale distincte `report77-new-format-directive.md` et réactive le même worker pour **nouvelle action post-b7**, sans confondre avec un rerun de ses preuves. Nouvelles preuves format dans dossier distinct ; aucun fmt final vert ou nouveau SHA déjà acquis à cette annotation.
