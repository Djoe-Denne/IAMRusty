# Couverture Sonar — 13 novembre 2026

## Périmètre et état initial

Projet SonarCloud `Djoe-Denne_IAMRusty` (AIForAll), analyse du 12 novembre
2026 à 20:10:40 UTC, révision `0d63b1920085dc931fe8a11d0adeea0f071fdf66`.
L'API retourne 77,9 % de couverture, 40 825 lignes à couvrir, 9 011 non
couvertes, 1 302 code smells, 6 bugs, 0 vulnérabilité et 1 308 issues classiques
(`violations`). Les issues ne sont pas corrigées dans ce lot coverage.

Sources : [mesures Sonar](https://sonarcloud.io/component_measures?id=Djoe-Denne_IAMRusty&metric=coverage&view=list),
API `/api/measures/component`, `/api/project_analyses/search`, et artefact
`lcov` de la [CI baseline 43293107226](https://github.com/Djoe-Denne/IAMRusty/actions/runs/43293107226).

## Livrables

- `services/Manifesto/tests/runtime_writer_postgres.rs` : sept IT sur le port
  `RuntimeControlPort`, l'adaptateur SQL réel et Postgres migré. Réutilisation
  de `common::setup_test_server`, du descripteur Manifesto, d'OpenFGA réel et
  des fixtures de données existantes. Tests `#[serial]`, aucune infrastructure
  parallèle ni double du système testé.
- Couverture des leases et fences, CAS/idempotence des observations, identité
  Pod immuable, révocation durable, pagination authentifiée et bornée,
  fermeture transactionnelle/tombstones, réservation d'effet avec consentements
  relus en DB, replay et complétion terminale. Après review : rejets de contexte
  périmé/deadline/identité révoquée, observation après fermeture, CAS périmé,
  curseurs authentiques expirés et concurrence entre deux holders.
- Sélection exacte `runtime_writer_postgres` dans le lanceur IT natif, avec
  extension de sa régression de sélection/refus. Aucun affaiblissement du cleanup.
- Ajout des tests unitaires **existants** `sentinel-sync` à la matrice
  `coverage-unit`. Leur artefact `lcov-unit-sentinel-sync` rejoint le merge
  `lcov-*` déjà existant.

Les tests SQL arrangent explicitement les états runtime dans la DB jetable et
exercent un contexte trusted de port. Ils ne constituent **pas** une preuve
positive d'authentification mTLS, d'attestation Kubernetes ou de provenance du
canal de maintenance. Aucune API de production n'est ouverte pour les tests.

## Validations locales

- Windows natif, Rust 1.99.1, 24 jobs configurés, target E:, un seul Cargo.
- Sept IT instrumentées : **7 PASS, 0 FAIL, 0 ignored/filtered**, 12,94 secondes
  d'exécution au dernier run. Une première erreur de setup a été corrigée : les
  builders métier créent un projet `draft` et des composants `pending`, donc le
  test de réservation doit les arranger `active` explicitement.
- La concurrence comprend un verrou de ligne détenu par une transaction réelle,
  un waiter observé via `pg_blocking_pids` et une assertion sur l'horloge du lease
  après le début de cette attente. Pas de sleep ni de port d'horloge de production.
  Les curseurs expirés sont signés avec la clé injectée déjà possédée par le test,
  sans altérer leur MAC ni attendre leur TTL.
- `sentinel-sync --lib --bins` : **50 PASS**, 0 FAIL/ignored/filtered ; aucun
  test dans le binaire. Pas de runtime tiers lancé par cette commande.
- Clippy ciblé pedantic + nursery : commande réussie, aucun diagnostic attaché
  au nouveau fichier de tests après correction de deux avertissements locaux.
  Les avertissements préexistants hors fichier ne sont pas présentés comme résolus.
- Régression PowerShell de sélection IT : PASS. YAML parsé et contrat de collecte
  vérifié (entrée unique, nom d'artefact, fichier `.info`, dépendance du merge et
  glob `lcov-*`).

## Mesure locale et limites

Instrumentation `-C instrument-coverage` et outils LLVM correspondant au
toolchain Windows. Comparaison par **chemin et numéro de ligne** avec le LCOV
baseline, en conservant uniquement les lignes mesurées dans la baseline et
nouvellement exécutées dans Manifesto infra ou sentinel-sync. Les fichiers de
production de ce périmètre sont inchangés.

Résultat : **1 790 lignes auparavant non couvertes exécutées**, dont 1 409
dans Manifesto infra et 381 dans sentinel-sync. À dénominateur baseline constant :
`(40 825 - 9 011 + 1 790) / 40 825 = 82,3123 %`.

Il s'agit d'une **projection locale**, pas du résultat d'une nouvelle analyse
Sonar. La CI Linux, la fusion des rapports et l'analyse distante doivent encore
confirmer la métrique publiée ; l'ajout de cibles unitaires peut aussi modifier
les lignes remontées par les rapports. Les nouvelles lignes de tests absentes
de la baseline ne sont pas utilisées pour gonfler ce gain projeté.

Les exclusions Sonar, le code de production et la prise en compte des E2E
restent inchangés. Les parcours de maintenance avec preuves signées restent une
zone non couverte importante, hors preuve de ces sept IT.

## Ressources et publication

### Première publication et résultat Sonar effectif

Commit `33fddc887331bbc87df78cf01ded08f653d5bc6b`, vérifié sur
`origin/master`. Analyse Sonar `be36dae3-668e-47d6-8457-c4da4e0a7cef`,
13 novembre 2026 à 10:59:08 UTC, **révision exactement égale au commit** :
**82,7 % de couverture**, 41 827 lignes à couvrir et 7 219 non couvertes.
Gain affiché : **+4,8 points** par rapport à 77,9 %. Les rapports supplémentaires
font évoluer le dénominateur, d'où la différence avec la projection conservatrice
à dénominateur constant. Code smells : 1 303 ; bugs : 6 ; vulnérabilités : 0 ;
issues classiques (`violations`) : 1 309. Aucun finding Sonar sur le nouveau
fichier de tests lors de la consultation ciblée de l'API.

La [CI 43409146018](https://github.com/Djoe-Denne/IAMRusty/actions/runs/43409146018)
a exécuté les **sept nouvelles IT sous Linux : 7 PASS**, en 7,95 secondes.
Le job unit Sentinel Sync, le merge LCOV et Sonar sont verts. **La CI globale
est néanmoins rouge** : deux anciens tests lexicaux P3/P4 signalent les données
`invoke` présentes dans le nouveau test, et les tests HTTP Telegraph échouent au
setup SQS/LocalStack (dispatch failure puis readiness après 30 tentatives).
La cause profonde du démarrage LocalStack n'est pas établie ici : ce n'est pas
un PASS ni un flake déclaré résolu.

Correction de la régression lexicale : ajout de témoins de contrat limités au
**chemin exact** du nouveau test dans `runtime_gate_contract.json`, sans modifier
les scanners ni exempter le fichier entier. Douze cas accept/refus supplémentaires
préservent les interdictions de méthode `invoke`, suffixes ASCII/Unicode et
transfert vers un autre chemin. Validation native des deux binaries P3/P4 :
**6 + 13 tests PASS**. Cette correction ne change ni les IT writer ni la production.
Re-review spécialisée : **PASS**, aucun affaiblissement du scanner ni nouvelle
exemption globale détecté ; les douze cas ont été vérifiés avec mutation mindset.

### Confirmation finale — campagne complète verte

Correctif publié et vérifié sur `origin/master` :
`65bc37677d176a485172e9c4969b738c9d0b4475`.
La [CI 43414594622](https://github.com/Djoe-Denne/IAMRusty/actions/runs/43414594622)
est **entièrement verte** : 24 jobs réussis, dont toutes les intégrations
(Manifesto, Telegraph, IAM, Hive, Apparatus), les tests unitaires, le merge et Sonar.
L'échec de setup LocalStack de la première campagne ne se reproduit pas dans
cette campagne ; aucun changement de production Telegraph ni assouplissement
de readiness n'a été appliqué pour obtenir ce résultat.

Analyse Sonar `14caec52-a416-4637-a3a9-f9ec12a376ad`, 13 novembre 2026
à 11:45:14 UTC, révision exactement égale à `65bc376` : **82,7 %**,
41 827 lignes à couvrir, 7 219 non couvertes ; code smells 1 303,
bugs 6, vulnérabilités 0, issues classiques 1 309. Le résultat de couverture
est donc confirmé par une campagne complète réussie, et non seulement par la
projection locale ou les rapports de la première campagne rouge.

Cette confirmation documentaire est postérieure à la campagne ; elle ne change
aucun source Rust, test exécutable, scanner, fixture ou réglage de coverage.

Dernière campagne IT : 2 conteneurs créés, 2 retirés, 0 possédé running, 0 nouvelle
ressource non attribuée. Les 4 conteneurs préexistants restent inchangés ; aucun
arrêt global Docker/WSL, aucune intervention sur le cluster d'une autre campagne.
Registre local : `.tmp/docker-resources-20261113.log` ; inventaire de campagne :
`.tmp/itfixtures-it-361bbd1364a7446bb826c294736a9c8b/inventory.jsonl`.
Logs/profils LLVM/scripts ponctuels restent sous `.tmp`, sans publication.

Reviews indépendantes initiales : correctness PASS sans finding ; tests PASS
borné avec cinq gaps non bloquants. Les gaps ont donné lieu aux ajouts décrits
ci-dessus ; relecture du delta **PASS, T-1 à T-5 résolus**, aucune assertion
initiale affaiblie. Ce rapport archive les preuves locales ; il ne remplace pas
le résultat autoritatif de la nouvelle analyse Sonar déclenchée à la publication.
