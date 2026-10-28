# 135. Disponibilité du SDK vérifiée sans publication

## Résultat utile

Le SDK corrigé est présent localement et propre. Son historique descend du pin
committé de l'application. La branche principale distante est accessible en
lecture et son commit est un ancêtre de la candidate : aucune divergence n'est
constatée qui imposerait une fusion ou un écrasement de la branche distante.
Ce constat ne prouve pas le droit de push et n'est pas une publication.

La priorité reste la preuve verticale finale en préparation. Le composant132 et
les revues HTTP déjà acquises ne la remplacent pas. Aucun test, compilation,
checkout, fetch, push ou changement de pin n'a été fait pour cet inventaire.

## Limites et gates de livraison

Le SDK est en HEAD détachée, sans branche locale contenant la candidate. Ne pas
changer ce checkout avant de conserver sa référence et ses preuves. Une future
publication doit porter sur le SHA exact testé/revu, jamais sur une reconstruction
approximative ni un force-push. Le pin final n'est mis à jour qu'après vérification
de sa disponibilité distante et des gates QA applicables.

Les preuves HTTP ciblées et revues restent historiques valides pour la source
inchangée ; les limites de couverture, la dette Clippy stricte non verte et Sonar
non exécuté restent explicites dans le ledger. Aucun scan global ou traitement
de dette horsscope n'est ajouté par cet inventaire. La politique ancienne des
skills de campagne (`-j1`, activation d'un projet Serena partagé) ne s'applique
pas contre le canon actuel : jobs24, un seulCargo, pas activate_project.

L'agent de recherche n'a pas livré de réponse finale récupérable après sa reprise.
Le lead a exécuté l'inventaire directement ; ce n'est pas un résultat d'agent
inventé ou une nouvelle revue de code.

## Annexe des observations et commandes

Répertoire : TI/rustycog ; `status --porcelain` vide, HEAD candidate
`d60a88b2f2f499768a807941a1b4c80c9a366b3f`, index application
`129dcc1575e7dd59b4c7935de0b3794dbb7e7d07` mode160000.

- `merge-base --is-ancestor 129dcc1 d60a88b` :0.
- `log --oneline 129dcc1..d60a88b` :8 commits HTTP feature/fixes/tests/style.
- `diff --name-only 129dcc1 d60a88b` : Cargo.lock, Cargo.toml,
  rustycog-http/src/{jwks.rs,lib.rs,owned_server.rs},
  rustycog-http/tests/mtls_client_auth.rs. Aucun dispatcher outbox.
- `branch --contains d60a88b` : HEAD détachée seule.
- `ls-remote --heads origin main` : succès, distant
  `41a86dba0a24844f900783e19ef2405319d58b3f`.
- Objet distant déjà connu localement (`cat-file -e` :0).
- `merge-base --is-ancestor 41a86db d60a88b` :0 ; inverse :1.
- `show --no-patch --format` : ce commit distant porte le correctif historique
  «close remaining clippy sites», pas une preuve du Clippy actuel.

Résultats traités dans ContextMode ; aucune URL avec credentials ou sortie brute
de configuration sensible reproduite. Localisation : historique et refs SDK.
Impact : préparation de livraison. Limites : pas test ni permission de push
attestée, pas preuve de publication, pas modification de la candidate.
