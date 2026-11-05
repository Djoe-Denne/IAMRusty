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

## Limites à cette étape

Cette preuve concerne les sources et leurs tests, pas encore la construction
Linux, la publication de l'image, son activation locale ou un départ E2E réel.
Le précédent build s'arrêtait sur E0583 parce que le dossier de code `secrets`
était exclu de sa copie de sources. La suite prévue est de reconstruire seulement
GitHub, puis d'activer les composants existants sous le bail local-full autorisé,
avec vérification infra indépendante. Aucun déploiement général ou seed rejoué.
