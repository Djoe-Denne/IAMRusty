# Préparation locale GitHub seul

## Périmètre vérifié

La cible locale est GitHub seul. Le raccord de coexistence avec GitLab et la
fusion future des services sont hors de ce lot. Les secrets déjà provisionnés
dans OpenBao doivent être réutilisés ; aucune migration, rotation ou nouvelle
pile n'est nécessaire à cette préparation.

Le pilote `ops/tests/local_full_e2e.py` accepte le snapshot IAM standard fourni
par `local_full_openbao.snapshot`, exige GitHub seul en rétention et conserve
les empreintes de configuration dans le Pod. Seules les identités publiques et
la projection numérique des connecteurs ressortent. Les gardes strictes, l'essai
unique, la conservation des données, la révocation métier et les 39/6 obligations
restent inchangés. Cela ne qualifie pas l'inverse de nettoyage Telegraph.

Le filtre `ops/deploy/image_sources.py` autorise uniquement les deux fichiers
publics SDK `rustycog-config/src/secrets/mod.rs` et `tests.rs`, et la traversée
nécessaire pour les atteindre. Il ne permet pas l'export des dossiers secrets
en général. Le SDK, les fichiers Rust, Dockerfiles et clés restent inchangés.

## Vérifications effectuées

- Tests du pilote : 78 réussites après intégration du complément indépendant.
  La production du pilote a aussi reçu une revue indépendante sans finding.
- Tests du filtre : 12 recensés, 11 exécutés avec succès, un test Cargo optionnel non activé.
  Capture et copie exactes, 23 voisins privés exclus sans lecture, identités des
  consommateurs modifiées quand l'un des deux fichiers publics change.
- ROUGE discriminant du filtre conservé avant correction ; oracle Windows
  corrigé pour comparer les octets réellement écrits, sans normaliser la copie.
- Capture du SDK réel vérifiée ; revue indépendante du filtre sans finding ;
  `git diff --check` réussi. Aucun nouveau harness ou Cargo cache-proof activé.

## Construction et activation observées ensuite

La construction Linux de GitHub seul a réussi sur les sources publiées en
`d11aa2ce36486527492c5b59df381f58b0bdd3ad`. L'image a été publiée au registre local
existant et téléchargée avec contrôle du digest sur les trois nœuds autorisés.
Le défaut E0583 du dossier de code exclu est résolu, sans modification Rust/SDK.

L'activation a créé les ressources standard nécessaires, puis GitHubConnect,
sous le bail original. Les identités sont enregistrées ; les données, anciennes
configurations, clés et secrets OpenBao sont préservés. Aucun seed ou déploiement
général n'a été rejoué. IAM n'a pas été basculé : l'activation de GitHub n'a pas
atteint sa condition de disponibilité.

## Limites constatées le 6 novembre 2026

La vérification infra indépendante conclut **FAIL** pour la disponibilité de
GitHub : son agent attend un fichier de jeton absent, l'application n'est pas
initialisée et son endpoint n'est pas prêt. OpenBao répond depuis ce container
(initialized=true, sealed=false, TLS vérifié), mais cela ne prouve pas que
l'authentification réussit. Le JWT est lisible sans en avoir ouvert le contenu.

Le répertoire du jeton est reconnu inscriptible ; le fichier PID de l'agent est
présent. Ces métadonnées ne prouvent pas la réussite de son authentification ni
de l'écriture finale. La cause précise demeure inconnue : aucun correctif de
rôle, reset ou reprovisionnement n'est justifié par ces seules observations.

Le client ancré existant n'expose pas la lecture des journaux. Ne pas contourner
cette garde ni introduire une nouvelle capacité de diagnostic implicitement.
Aucun oracle métier ou départ E2E n'a été exécuté ; les 39 obligations mesh et
six Apparatus restent conservées, non qualifiées par cette préparation.

Le registre relancé pour la publication a retrouvé son état arrêté initial une
fois l'image chargée sur les trois nœuds. Ses données et volumes sont conservés,
tous ses auxiliaires sont arrêtés. Les nœuds et autres workloads préexistants
n'ont pas été arrêtés. La configuration GitHub reste sous le bail de diagnostic ;
la mission n'est pas clôturée.
