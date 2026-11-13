# Trois scénarios k6 sur Ubuntu GitHub — résultat du13novembre2026

## Résultat mesuré

[Run43461703946, attempt1](https://github.com/Djoe-Denne/IAMRusty/actions/runs/43461703946)
du workflow manuel `.github/workflows/k6-e2e.yml` : **34checks PASS**, cinq phases
complètes, corrélations parent PASS, restauration SAN PASS et cleanup PASS avec
`owned_running=0`. Run fonctionnel `8fd27663ad0443bf854231df68e2d5b4`.
Vérification indépendante finale **PASS borné** par infra-verifier, session
`ses_e2b39f576ffe6sWFVATRaMqC3i`, sur API, artefacts et source exactebc143cc8.

- Health :2checks.
- TLS/SAN bon, mauvais, restauré :2checks par phase, même incarnation Envoy.
- OAuth navigateur réel :26checks, inscription/login/relink, PKCE, cookies et
  refus protégés ; pas de sonde substituée et pas de seed utilisateur/token.
- Par phase :expected/started/completed/iterations=1 ; une requête k6/http par
  phase HTTP, zéro k6/http pour le cas navigateur.
- Corrélations acceptées par le parent :IAM+7, ext-authz+4, provider et PKCE+3,
  users+1, outbox+0, SANfail_verify_san+1 ; restauration positive. Les récepteurs
  exigent également les sous-compteurs provider et l'absence d'erreurs transport.

## Sources et reproduction

- Harness exécuté :`bc143cc827d84543880506c5352f1c81f496ce5e`.
- Applications T :`1ca9a4c83226da50adb33822233b488c98a4736b`.
- SDK S :`93b890ff7b821e9cb2e7d0aeb81957d098778a2f`.
- Packaging H :`8f614eb3a9c50245d5e5f0cb57bed03021d5ac91`,
  [run43420550908, attempt1](https://github.com/Djoe-Denne/IAMRusty/actions/runs/43420550908).
- k6 :2.3.0 ; Chromium complet :156.0.8078.73, distributions épinglées/vérifiées.
- Artefact de résultat :`k6-e2e-43461703946-1`, rétention sept jours.
  ID14055609627,7623octets,14entrées ;14/14SHA256 distants égaux aux copies locales
  selon le recoupement indépendant en mémoire.
- Copies locales expurgées :`.tmp/k6-e2e-43461703946/` (non versionnées).

Images applicatives reçues, non instrumentées :

- IAM :`sha256:357baa08fefe9dde4a9577b7c29c6d56484085bcff314cc9c55928779c4fec29`.
- OAuthConnect :`sha256:ee5fd763ff1969e52fba53c4874e6babb79304cad1e600dadadfc5340e1b6bb7`.
- ext-authz :`sha256:5d1a6162c4f54d3f8ab8a94df5310e8a5d37803b98871bf3202b60e46eacbb10`.

Voir le [runbook](../../ops/tests/k6/ci/README.md) pour le dispatch. Aucun Cargo,
UT/IT ou build applicatif dans ce consumer ; seule l'image d'outillage navigateur
est empaquetée. Les artefacts expirés entraînent un refus, pas une recompilation
automatique. Aucun secret ni certificat privé opérateur/GitHub requis.

## Navigateur et sécurité

Les sondes précédentes ont résolu les barrières namespace/filesystem du sandbox,
puis le défaut de repérage des titres Linux Chromium réécrits. Le raccord initial
5c4be526 ne qualifiait que8checks :OAuth refusé au démarrage. Le correctifbc143cc8
monte la case sous le chemin court `/run/case` et préserve les preuves de cleanup
sur exit enfant non nul. Le raccourcissement est cohérent avec la limite des
sockets Unix Chromium ; l'ancien run ne contenait pas de cause native classifiée,
donc aucune causalité exclusive n'est revendiquée rétroactivement.

Dans la campagne finale, quatre témoins renderer distincts sont conservés :
UID1001, user namespace et PID namespace distincts du superviseur, seccomp2 et
deux filtres contre un dans le conteneur, NoNewPrivs1, aucun drapeau interdit.
Page/parcours réels, exit0 et arbres enfants joints. AppArmor DOCKER_DEFAULT
mesuré depuis le processus ; rootfs readonly, capdropALL, utilisateur non-root,
aucun privileged, helper SUID, no-sandbox ou changement AppArmor/sysctl global.

Profil seccomp :référence Moby `9ee3204f0d0cfa19ca2c6a7b0fcf5a2b353954ae`,
delta explicite clone/setns/unshare/chroot, SHA256 dérivé
`c7f098ba7f9c2eb7f36bfe909c2ee6837efe39a82a3f35ddcf661cd4caac8ee8`.
Ce profil n'est pas prétendu identique au builtin du daemon. Réseauhost limité
à la VM GitHub jetable pour garder `https://127.0.0.1:18443` et
`https://localhost:18445`. Deux binds seulement :case RW et inputs publics RO ;
aucun kubeconfig, clé privée ou socket Docker exposé au navigateur.

## Nettoyage et état local

Les cinq cases rapportent cleanup=true/exit0. Résultat navigateur et cleanup
conteneur PASS. Le parent et la garde always confirment cleanup PASS, aucune
erreur, `owned_running=0`, nœud et volume reçus retirés.

- Nœud :`2a1a2441cd5681676d2108cb4268d70120b631fda9c72a6afe5b140f4c11e00f`.
- Navigateur :`5aacab71c9781e98835160836d09fa717a877eaf96b48d09aef8440854614ea3`,
  created→stopped→reclassifiedyes→removed avant le teardown du nœud.
- Volume :`5cc298b959f0826b16baef1a2a18ffb147724e2b6b8011482f0230289297ea46`.
- Image d'outillage :`sha256:f482d15a77467aff04b3bd7613103c99517272367479520817322e30788f55d1`,
  conservée sur la VM jetable, comme les autres images ; aucun prune global.

Aucun runtime Windows/Docker/Kind/WSL démarré, adopté ou arrêté par ce lot.
Les nœuds Kindfull/SMTP rendus par l'autre campagne restent hors propriété.
Aucune branche/worktree temporaire créée pour cette implémentation directe.

## Validation et limites

116helpers CI PASS/0skip après le dernier correctif ;4tests ciblés du témoin
sandbox et10régressions native-campaign mesurés verts sur leurs frontières
inchangées. Les tests de câblage vérifient le vrai JS, la réception des26checks,
les refus de preuves/mounts, l'environnement fermé et le cleanup sur erreur.
Ces tests seuls n'étaient pas une qualification ; le run ci-dessus apporte la
preuve réelle de la slice applicative ciblée.

- Les corrections coverage90 publiées après T ne sont pas incluses dans ces
  images ; aucune couverture90%/mesure Sonar déduite de cette campagne.
- Pas de charge, maintenance/mTLS exhaustive, qualification du lab/production
  ou de toutes les fonctions métier.
- Les deltas publiés sont ceux acceptés par le parent ; les snapshots privés
  bruts et flux OAuth ne sont pas publiés pour permettre une reproduction libre.
- Les retraits sont vérifiés depuis les reçus/source et les artefacts ; le runner
  n'est plus accessible pour une réobservation live. Les images sont conservées
  sur la VM jetable, aucune observation supplémentaire de leur destruction.
- La branche corrigée de remontée cleanup après un exit enfant non nul est
  couverte par régression unitaire. Le run final vert ne l'exerce pas ; le run
  échoué5c4be526 prouve le défaut initial, pas un replay rouge du code corrigé.
- Les artefacts sont exclusivement allowlistés ; aucune URL OAuth query,
  cookie/token/state, clé, kubeconfig, HAR ou log brut.
- Le [contrat](../contracts/k6-three-scenario-ubuntu-ci.md) reste **Proposed** ;
  un succès ne promeut pas une décision d'architecture.
