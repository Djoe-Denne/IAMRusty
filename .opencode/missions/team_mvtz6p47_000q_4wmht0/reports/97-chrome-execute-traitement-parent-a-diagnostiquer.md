# Chrome exécuté ; traitement du résultat parent à diagnostiquer

## Résultat vérifié

Le format de version est corrigé et couvert par un test de régression rouge puis
vert. Le nouveau lancement réel progresse : k6 annonce2.3.0, les deux inspections
retournent les options attendues et Chrome démarre pour le cas minimal.

Le quatrième enfant k6 quitte avec le code0, sans stderr. Dix processus y ont été
observés :1k6 et9Chrome, tous terminés ; son job ne contient plus aucun processus
actif. Les trois enfants k6 de version/inspection finissent également avec le code0.
La séquence complète représente donc13processus observés terminés. Inventaires
Windows avant/après :0/0 k6 ou Chrome. Aucun processus étranger arrêté, aucun forçage.

Le parent rend toutefois exit2/PRECONDITION_REFUSED après l'exécution navigateur,
en3833ms. CleanupPASS et confidentialitéPASS. Le résumé courant n'étant pas conservé
sur cet échec et l'exception parent n'étant pas décrite, **aucun PASS navigateur ou
preuve de version Chrome n'est revendiqué**. Le code0 natif ne remplace pas ces preuves.

## Diagnostic retenu

Ajouter un diagnostic privé borné de l'exception du parent et conserver, s'il existe,
le résumé du run en échec. C'est une réponse à l'échec effectivement observé, pas
une nouvelle protection ou attestation du sandbox. Le même auteur réalise ce petit
delta et le lead rejoue ensuite le cas pour identifier précisément la cause.
Les critères de réussite et les protections natives restent inchangés.

## Preuves et limites

- Correctif préfixe reçu `result_mvw6ydwm_0001_z4ah9h`,2279octets/SHA256
  `77ed6da403b2412a596255cc90ba07ecc2b79949e82c133349a89499d0af93b3`.
- Deux fichiers intégrés SHA256 égaux à la livraison ; `git diff --check`exit0.
- Auteur : test ciblé identique avant/après, ROUGE1test/exit1/0.419s, VERT1test/
  exit0/zéroskip/3.630s. Suite32non rejouée pour ce petit correctif.
- Inspections réelles :1VU/1itération/per-vu-iterations, HTTP15s+2s et browser20s+3s,
  trois seuils fonctionnels exacts ; aucune requête de service dans inspect.
- Preuve privée : `.tmp/k6-browser-r3-20261031-retry1-lease.json` et12fichiers de
  diagnostics enfant/processus, non publiés. Quatre stderr natifs vides.
- HTTP/selftest, WSL et Linux restent INCONCLUSIVE. Aucun runtime tiers démarré.
