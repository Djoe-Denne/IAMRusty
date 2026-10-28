# Rapport 42 — témoins Sentinel recoupés, runtime nécessaire à AC6

## Preuves lues via Context Mode

Logs réels du worktree T3 `.tmp/t3-evidence/` :
- sentinel-classifier-red.log : cinq témoins exécutés avant fix,1PASS/4FAILED/0ignored,2filtered. Le cas réel et les cas authn/authz/indisponibilité, ambiguïté/opération et batch sont rouges ; le marqueur historique passe.
- sentinel-classifier-green-2.log : mêmes cinq noms,5PASS/0failed/0ignored,2filtered ; auteur rapporte ajout des cas malformed/duplicatefield/plainlegacy dans ces témoins.
- sentinel-private-unit.log :3PASS/0failed/0ignored,45filtered, dont rejet d'erreur non attribuable et tests normalisation/reconcile existants.

Ces témoins ne remplacent pas le rejeu réel OpenFGA/SQS ni la revue indépendante du diff.

## État hôte observé sans Docker CLI

Get-Process : aucun Docker Desktop/com.docker.backend/cargo observé. WSL list : seule distribution docker-desktop, Stopped, version2. Exécutable Docker Desktop présent. Aucun daemon réveillé par ce préflight. Les anciens états de conteneurs ne sont pas présentés comme inventaire live.

## Décision opérationnelle dans le mandat IT

Besoin concret restant : exécuter la cible routing réelle après correctif autorisé. Lifecycle chargé et règles relues ; autoriser un démarrage à la demande de Docker Desktop seulement, pas de Kind ni redémarrage manuel d'un nœud/ancien conteneur. Inventaire complet dès disponibilité et avant création fixture. Toute restauration automatique par le daemon est observée, non adoptée et non revendiquée comme démarrage explicite possédé.

Intention et bail runtime persistés avant opération dans `.tmp/telegraph-outbox-runtime-20261026.jsonl` du checkout lead. Conserver les quatre SMTP mission-owned arrêtés ask, volumes opaques et node non possédé ; ne pas promettre leur état avant inventaire. Démarrage borné, pas de campagne de réparation si échec. Nouveau bail fixture T3 à confirmer après inventaire contemporain, puis cible routing seule, unique Cargo24jobs. Aucun autre test déjà vert à rejouer.

## Démarrage et inventaire observés

Desktop démarré par le lead, PID initial47888, horodatage2026-10-26T15:26:29Z. Readiness Docker réussie, version29.8.2. Inventaire full-ID immédiatement après startup, avant fixtures : quatre SMTP historiques exited et node40201...running. Aucun autre container observé. Le node a été observé running lors du retour daemon ; aucun docker start/Kind/manual-restart exécuté par le lead, aucune appropriation du node. Les quatre SMTP restent intacts. Inventaire archivé `.tmp/telegraph-outbox-post-start-containers-20261026.jsonl`.

Bail parent renouvelé pour une seule cible routing et ses nouvelles fixtures, après préflight frais auteur/images/volumes. Même wrapper inspecté, allowlist réduite à routing et préfixe26octobre ; pas de changement des gardes. Stop gracieux/retrait Exited0 possédés comme précédemment, zéro possédé running en fin, nouveaux Exited2 et volumes opaques conservés ask. Pas de shutdown global Desktop/WSL tant que l'état non possédé du node/workload rend ce shutdown non sûr ; ne pas annoncer une restitution RAM non mesurée.

Complément auteur reçu ensuite, antérieur au message de startup : Clippy strict Sentinel lib exit0 et fmt-check sept packages exit0 annoncés, logs disponibles ; pas encore recoupés lead. Le Clippy Telegraph antérieur reste NONVERT. Archive source-before-ac6-it/manifest.json douze sources/baseea2c009/contrat09a59ee annoncée ; script routing seule hash9e899f802434efdb8bb9221dd0240eaabcaf5fba435d942d969841f482e7c731, delta allowlist/préfixe seulement et syntaxcheck annoncés. Pas de rerun statique demandé. Startup déjà exécuté et bail confirmés à nouveau pour éviter attente sur un constat hôte désormais historique.
