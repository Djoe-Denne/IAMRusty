# k6 vérifié ; Chromium reste bloqué par un prérequis système

## Objectif et résultats nouveaux

Le défaut de sécurité est résolu selon la revue indépendante. Le lead a ensuite exécuté les outils épinglés sous Linux non privilégié. k6 annonce v2.3.0 et les deux inspections de scénarios réussissent : une seule VU (utilisateur virtuel) et une seule itération, sans requête applicative.

Chromium ne démarre pas : son `--version` retourne127 car `libnspr4.so` manque dans la distribution WSL. Le mode composite `tools` est donc en échec ; il ne peut pas être présenté comme qualifié. Le navigateur sandboxé n'a pas été testé. Les processus possédés des commandes terminées ont été contrôlés par le superviseur, avec cleanup vérifié.

## Suite dans le mandat

Ne pas installer de paquet implicitement ou assouplir la sandbox locale. Publier les livrables durables vérifiés sur master, comme approuvé, permet au workflow dédié de qualifier les prérequis sur un runner Linux natif existant. Ce sera un déclenchement normal par push, pas un dispatch manuel, un nouveau déployeur ou une nouvelle cible. Le workflow CI préexistant reste inchangé.

Ce passage ne promet aucun résultat CI : son exécution et ses artefacts doivent être observés. Même une CI de prérequis verte ne qualifierait pas HTTP, toujours sans exposition publique conforme. La mission reste non clôturée jusqu'à publication vérifiée, disposition des branches et nettoyage applicable.

## Preuves techniques

- Source exécutée : copie native propre de 5d79557, UID réel/effectif1000, archives/ELF revalidés par le lanceur avant usage.
- Mode tools : exit1, diagnostic TOOL_VERSION ; log privé `.tmp/k6-tools-qualification-1793311665764.log`.
- Versions isolées : k6 exit0, `k6 v2.3.0 (commit/e088784614, go1.26.8, linux/amd64)` ; Chromium exit127, bibliothèque manquante libnspr4.so. Log `.tmp/k6-versions-diagnostic-1793311695515.log`. Le booléen version_matches du script diagnostic k6 est invalide à cause d'un échappement de regex dans ce script, pas du lanceur ; seule la ligne de version observée est retenue. Aucun relancement de binaire nécessaire pour cette correction d'interprétation.
- Deux inspect : exit0 ; iam-health per-vu-iterations/1VU/1itération/maxDuration15s, browser-prerequisite mêmes cardinalités/maxDuration20s ; cleanup true pour chaque enfant. Log `.tmp/k6-inspect-qualification-1793311714374.log`.
- Revue S-1 : `.cursor/review-briefings/20261029T2206Z-security-k6-s1-fix-5d79557.md`.
- Aucun container créé/redémarré, aucun paquet installé, aucune requête applicative, aucun test navigateur réussi déduit des inspect.
