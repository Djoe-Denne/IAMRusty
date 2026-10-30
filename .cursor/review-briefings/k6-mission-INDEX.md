# Revues de la mission k6 — portées Linux et Windows distinctes

- [Protection persistante et périmètre R-1](20261030T2027Z-security-k6-windows-r1-trust-9ef802f.md) — modèle de menace initial préservé, clarification recevable sous conditions ; provenance privée, séparation des sorties et conservation à prouver. R-1OPEN, essais réels gelés, aucune garantie contre rôles compromis nouvellement prétendue.

- [Correctifs Windows : dossier et préflight fermés, rétention résiduelle ouverte](20261030T2005Z-security-k6-windows-delta-bd7a2bf.md) — relecture68631c0..bd7a2bf : S-1 fermé, R-1 fermé au préflight mais OPEN si survivant, fauxPASSR-2 fermé/ChromiumINCONCLUSIVE. Les essais réels restent gelés jusqu'à disposition vérifiée de la rétention.

- [Primitives Windows et réserves avant lancement réel](20261030T1913Z-security-k6-windows-primitives-cbcc1bb.md) — REVIEW_REQUIRED : course de création/nettoyage, intégrité des bibliothèques chargées à couvrir, témoin de jeton primaire insuffisant pour qualifier l'isolation. Aucun lancement réel admis par cet avis ; corrections et limites à vérifier avant activation.

- [Conducteur privé Windows/Linux — ancêtres et signal de rétention](20261030T1428Z-security-k6-private-parent-4560b18.md) — deux MEDIUM, aucun CRITICAL/HIGH ; [fermés par revue ciblée](20261030T1542Z-security-k6-private-parent-closure-0f32e8d.md), stockage réel toujours non admis, aucun préflight ou collecte réelle autorisé par cette fermeture.
- [Canal local privé — capture concurrente incomplète à refuser](20261030T1243Z-security-k6-private-site-eb3d355.md) — un MEDIUM, aucun CRITICAL/HIGH ; [fermé sur le correctif cbab06e](20261030T1254Z-security-k6-private-site-s1-19b1cfe.md), aucune admission de collecte ou fuite publique démontrée.
- [Provisionnement de l'instance WSL AIForAll sur I:](20261030-wsl-aiforall-provisioning-infra-result.md) — PASS indépendant borné à l'identité, stockage, compte et paquets ; aucune qualification k6/Chromium/HTTP, historiqueACLparent non prouvé.
- [Consultation technique sur le signal direct22](20261030-k6-signal-expert-result.md) — analyse de contrôle, pas verdict QA ; signal primaire observé avant cleanup, signal exact et cause restant inconnus.
- [Consommateur privé et lancement direct22 séparé](20261030T0023Z-security-k6-direct22-private-consumer-7015c8c.md) — PASS sécurité ciblé d47b805..0601f6f ; aucun finding démontré, preuve native restant à acquérir.
- [Comparaison temporaire Ubuntu22.04, contrôle fermé et non qualifiant](20261029T2358Z-correctness-k6-ubuntu2204-comparison-12c3f123.md) — PASS statique b316668..d47b805 ; aucun finding démontré, pas de qualification Linux/Chromium/HTTP.
- [Diagnostic Chromium direct et suggestion d'exactitude](20261029T2321Z-security-k6-direct-chromium-ffe4a27.md) — PASS sécurité statique14d..e9af ; suggestion overflow/cleanup corrigée dans05e25e7, aucun PASS navigateur implicite.

L'index général local comprend des entrées d'autres missions et n'est pas publié par ce lot. Ces liens sont le sous-ensemble durable propre à la mission k6.
