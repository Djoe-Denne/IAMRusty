# Rapport 23 — préflight et bail IT SQL T2

## État annoncé, pas verdict d'exécution SQL

T2 rapporte cargo check --locked telegraph-service PASS et compilation de
outbox_tests --no-run PASS. Un import privé de mapper dans le test a été
corrigé localement sans exposer d'API. Huit scénarios SQL écrits ; le nombre
effectivement exécuté et leurs résultats restent à recevoir/recouper.

## Préconditions et bail borné

Le membre a lu lifecycle/evidence/registre et examiné les effets transitifs :
descripteur SQL-only, sans build/run app, SQS/FGA désactivés, singleton Postgres
possédé, migrations down/up dans sa seule DB, pas d'éviction/adoption par nom.
Desktop/WSL étaient déjà actifs, image postgres15-alpine déjà en cache.

Inventaire BEFORE fourni par auteur : un container préexistant NON POSSEDÉ,
ID40201bda8bb2d1eeaa51287be96fc3448752c219dba74791fa326c5071705f29,
aiforall-local-control-plane. Il reste intouché, sans déduction de propriété
par son nom. Le lead ne déclenche pas de démarrage ou diagnostic Docker.

Bail parent donné au même membre pour la seule cible outbox_tests, IDs créés
par ses fixtures seulement, jusqu'à fin de batch et cleanup même en échec.
Slot Cargo Windows unique/jobs24. Commande : cargo test --locked
-p telegraph-service --test outbox_tests -- --test-threads=1.

## Preuves attendues et limites

IDs réels journalisés à création, registre append-only et inventaires avant/après,
arrêt gracieux exact-ID et zéro container possédé running à fin. Fournir chemins
absolus pour recoupement du lead ; aucune rétention spéculative pour T3.
Pas de stop/restart/prune/suppression de ressources non possédées. Aucun shutdown
global Desktop/WSL en présence du workload préexistant. Aucun test ou runtime
supplémentaire lancé par le lead ; pas de preuve SQL verte à annoncer encore.
