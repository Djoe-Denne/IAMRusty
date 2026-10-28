# 12 — Partage du modèle et reprise ciblée

## Delta vérifié depuis 11
- Pool faible : 4 tests PASS (identité, concurrence, libération du dernier
  client, nouvelle tentative après échec), sans chargement de modèle.
- Fraîcheur BM25 : test réel de commit/suppression avec reader manuel PASS ;
  les lecteurs conservés en cache sont rechargés avant recherche.
- Corrections indexing : 6 tests PASS (dont deux audits opt-in non activés dans
  ce filtre) ; incrémental : 4 PASS, 3 tests upstream marqués ignored car modèles
  requis. Ces trois derniers ne sont pas annoncés comme exécutés.
- Audit réel des métadonnées avec profil 8B explicite : **914/914 sources**
  correspondent aux stat/hash persistés de la collection historique
  `code_chunks_55cdd149_06293987`, aucun embedding produit par cet audit.
- Build native v3 PASS, binaire côte à côte SHA256
  `dd8647649275f7b295d73acecb396efcc1b5867ffe537356d283cc871b56a28f`.
  Backend actuel vérifié PID 54960 ; v2/v1/upstream et données backup conservés.
- Export complet sans staging : `patches/workspace-corrections-v3/manifest.json`,
  21 fichiers (y compris nouveaux modules). Aucun changement applicatif.

Preuves sous l'installation privée : `corrections-preparation-v3.json`,
`workspace-persisted-metadata-v3.json`, logs natifs liés. L'audit a été corrigé
pour nommer explicitement le backend 8B : sa première version utilisait le
backend par défaut pour dériver un nom de collection, sans charger/embeddre
aucun modèle ; cette première étiquette n'est pas la preuve de collection.

## Vérification active
Shell sh_16de23098001xdHr9ahQxZuYnf, `verify-workspace-v2.mjs --resume-v3`.
Preuve distincte `workspace-v3-verification.json`, preuve rouge v2 conservée.
Réutilise le HIR et la récupération déjà acquis ; aucun force rebuild/index.
Scénario : alias, recherche, fixture edit/rename/delete/recréation, avec caches
recherche conservés pour démontrer le partage d'une seule instance 8B réelle.

## Limites à ce stade
Les contrôles natifs ne suffisent pas encore à déclarer la fixture live PASS.
Attendre sa notification, vérifier libération mémoire et persistance après
restart. Obsidian CLI reste non vérifiable application fermée ; QMD fonctionne.
Aucun nouveau agent, Docker, Kind ou WSL.
