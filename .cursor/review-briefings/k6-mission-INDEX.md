# Revues de la mission k6 Linux

- [Conducteur privé Windows/Linux — ancêtres et signal de rétention](20261030T1428Z-security-k6-private-parent-4560b18.md) — deux MEDIUM, aucun CRITICAL/HIGH ; conditions à corriger avant préflight réel/collecte, pas de compromission de l'hôte constatée.
- [Canal local privé — capture concurrente incomplète à refuser](20261030T1243Z-security-k6-private-site-eb3d355.md) — un MEDIUM, aucun CRITICAL/HIGH ; [fermé sur le correctif cbab06e](20261030T1254Z-security-k6-private-site-s1-19b1cfe.md), aucune admission de collecte ou fuite publique démontrée.
- [Provisionnement de l'instance WSL AIForAll sur I:](20261030-wsl-aiforall-provisioning-infra-result.md) — PASS indépendant borné à l'identité, stockage, compte et paquets ; aucune qualification k6/Chromium/HTTP, historiqueACLparent non prouvé.
- [Consultation technique sur le signal direct22](20261030-k6-signal-expert-result.md) — analyse de contrôle, pas verdict QA ; signal primaire observé avant cleanup, signal exact et cause restant inconnus.
- [Consommateur privé et lancement direct22 séparé](20261030T0023Z-security-k6-direct22-private-consumer-7015c8c.md) — PASS sécurité ciblé d47b805..0601f6f ; aucun finding démontré, preuve native restant à acquérir.
- [Comparaison temporaire Ubuntu22.04, contrôle fermé et non qualifiant](20261029T2358Z-correctness-k6-ubuntu2204-comparison-12c3f123.md) — PASS statique b316668..d47b805 ; aucun finding démontré, pas de qualification Linux/Chromium/HTTP.
- [Diagnostic Chromium direct et suggestion d'exactitude](20261029T2321Z-security-k6-direct-chromium-ffe4a27.md) — PASS sécurité statique14d..e9af ; suggestion overflow/cleanup corrigée dans05e25e7, aucun PASS navigateur implicite.

L'index général local comprend des entrées d'autres missions et n'est pas publié par ce lot. Ces liens sont le sous-ensemble durable propre à la mission k6.
