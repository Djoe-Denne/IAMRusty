# Trois scénarios k6 — runner Ubuntu autonome

- **Proposed**, 2026-11-13. Accord utilisateur pour un job GitHub Ubuntu et une
  pile jetable, sans dépendance au poste local. Aucun statut existant promu.
- Cadrage architecte lecture seule : `ses_e2bb54211ffetS1Myw6ZEFwhS5`.
- Complément de [0504](../adr/0504-validation-fonctionnelle-linux-k6.md) et de la
  [parité native](k6-three-scenario-native-parity.md) ; Kind conserve le canal
  E2E mesh prévu par ADR0308 §8. Pas de changement Rust métier.

## Périmètre

Job `k6-e2e` indépendant des autres tests dans `ci.yml`, `ubuntu-24.04` hébergé,
permissions lecture. Ne pas réactiver le workflow Linux tooling suspendu.
Kind mono-nœud neuf par run/attempt, kubeconfig privé, namespace
`aiforall-local-full` uniquement à l'intérieur de ce cluster possédé.

Vrais IAM, OAuthConnect v2 `gitlab`, Envoy et ext-authz du checkout ; simulateur
IdP protocolaire existant ; PostgreSQL, OpenFGA et LocalStack/SQS dédiés.
Réutiliser Bake avec sélection `iam,oauth,ext-authz`, chart natif et backings
kind-minimal. Aucun lab local, OpenBao, service métier additionnel ou registry
publish. Secrets, clés JWT et PKI neufs dans le répertoire privé du runner ;
aucune clé versionnée/opérateur, aucun secret GitHub requis.

## Profil et preuves

Origines fermées `https://127.0.0.1:18443` (IAM) et
`https://localhost:18445` (IdP) ; cookie host-only isolé. TLS ingress navigateur,
mTLS upstream, SNI/SAN, stripping X-Principal, ext-authz fail-closed. Exemptions
exactes health/OAuth seulement ; link/relink-start restent protégés. Admin
loopback. Source Envoy CI publique distincte de la politique nominale.

Cinq phases, trois stimuli JS conservés et **34 checks nommés** : health 200/OK
(2), SAN 303→503→303 (2 chacun), OAuth Chromium (26). Résumés fermés frais liés
au run, expected/started/completed/iterations=1, requests=1 HTTP et 0 k6/http
pour le navigateur. Aucune tolérance ajoutée pour verdir.

Parent : même incarnation Envoy ; SAN fail_verify_san+1, fail_verify_error+0,
handshake/restauration positifs avant teardown. OAuth IAM+7, ext-authz+4,
authorize/token/profile/PKCE+3 chacun, users+1, outbox+0 et zéro erreur transport.
Admission et probes hors des fenêtres de compteurs. Bootstrap via produit réel,
pas de seed utilisateur/token ou substitution du connecteur.

## Linux et sécurité

Réutiliser k6/CfT épinglés et extraction complète. Chromium non-root et sandbox
réel observé ; confiance CA publique dans NSS du profil privé. Aucun no-sandbox,
TLS bypass, helper SUID, sysctl global ou déplacement d'exécutable pour contourner
AppArmor. Un refus userns est un blocage explicite, pas une qualification. Une
exception AppArmor nécessite une décision utilisateur distincte.

## Cleanup et réception

Reçus contemporains IDs Docker/UID Kubernetes, registre append-only, aucune
adoption. Joindre les enfants et forwards, tenter la restauration SAN, arrêter
les writers. Différence du lab : même si la restauration échoue, conserver FAIL
puis détruire le cluster jetable possédé ; sa destruction n'est pas un PASS SAN.
`finally` et garde GitHub `always()`, propriété vérifiée avant retrait. Annulation
dure du runner peut rendre la preuve incomplète, jamais succès présumé.

Artifacts exclusivement allowlistés : cinq résumés validés, deltas, provenance,
verdict et cleanup expurgés. Pas de dossiers récursifs, kubeconfig, clés, secrets,
URLs OAuth query, tokens/cookies/state, HAR, logs/dumps bruts.

Tests ciblés de rendu, résumés/refus, projecteur réel, interruption, restauration
et cleanup, puis vrai run GitHub sur commit publié. Outillage seul, skip Windows
sur POSIX ou simple exit zéro ne qualifient pas cette campagne. Ce contrat reste
Proposed même après un run réussi.
