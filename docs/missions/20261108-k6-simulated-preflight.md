# k6 simulated — préflight live bloqué

## Demande et périmètre

Instruction utilisateur : « go k6 alors », après les huit tests HTTP du simulateur.
Cible demandée : navigateur k6 → IAM → OAuthConnect/instance simulated → fournisseur
simulé, avec vérification du client secret. Exécution directe, sans nouvel agent,
branche ou worktree. Base `master` : `8388e44c204627c9a0e3632c8169c93664a88d65`.

## Constat live du 8 novembre 2026, 15:17 UTC

Ancre privée courante validée avec `LocalClusterTargetAnchor.check`, cible
`kind-aiforall-local-full`, namespace `aiforall-local-full`. Lectures uniquement
des Deployments et états publics des Pods ; aucun log ou secret exporté.

- Aucun Deployment `oauth-connect` : le profil livré dans les sources n'est pas
  activé sur ce runtime.
- IAM : 0/1 Ready. Son init container `openbao-snapshot` est en
  `CrashLoopBackOff` (41 redémarrages observés). Le container applicatif est
  terminé avec raison `Unknown`, code255. La cause racine n'est pas établie par
  ces seules observations ; ne pas l'attribuer à un mauvais secret sans preuve.
- Ancien `github-connect` : 0/1 Ready ; `gitlab-connect` : 1/1 Ready.
- Envoy, OpenBao et simulateur : chacun 1/1 Ready. Cela ne prouve ni le nouveau
  profil du simulateur, ni la disponibilité du parcours OAuthConnect.
- Archives et exécutables k6/Chromium portables : quatre SHA256 conformes au
  manifeste `ops/tests/k6/config/tools.windows.lock.json`.

## Verdict et actions non effectuées

**BLOCKED au préflight ; zéro scénario k6 exécuté.** Aucun PASS E2E revendiqué.
Le parent actuel reste lié au profil GitLab HTTP et inclut un stimulus SAN
mutatif. Il n'est pas lancé contre cette stack et n'est pas substitué au
scénario simulated. Les huit tests de fixture restent une preuve distincte.

Aucune mutation du runtime, rotation de secret, création/redémarrage Docker,
tunnel ou processus k6/Chrome. Zéro ressource possédée à arrêter ; les ressources
préexistantes ne sont pas adoptées. Pas de shutdown global Docker/WSL ni de
nettoyage des ressources conservées d'autres campagnes. RAM non mesurée.

## Condition de reprise

La préférence utilisateur est d'adapter les tests plutôt que modifier la stack.
L'absence d'OAuthConnect ne peut toutefois pas être corrigée par un changement
de slug k6. Il faut autoriser explicitement la préparation puis l'activation du
profil `--oauth-connect` sur le runtime local existant, avec ses entrées privées,
ses images et sa CA, sans reset des données ni réarmement de l'ancien run Python.
Le ledger ops précise que build Linux, déploiement et confiance CA n'ont pas été
effectués lors de sa livraison. Vérifier les effets du déploiement avant action.

Après disponibilité IAM/OAuthConnect : adapter et valider le parent/scénario
k6 aux callbacks `simulated`, à HTTPS et au contrat v2, puis lancer la campagne.
Conserver un oracle indépendant pour le client secret attendu ; distinguer le
400 du fournisseur des erreurs traduites par IAM. Ne pas utiliser le scénario
GitLab/SAN historique comme preuve de cette chaîne.

Sources : `.cursor/handoffs/20261108-ops-oauthconnect.md`,
`ops/tests/k6/run-windows.ps1`, `ops/tests/k6/lib/windows-auth.ps1`,
`ops/deploy/local_full_target.py`. Préflight temporaire lu/exécuté depuis
`.tmp/k6-simulated-20261108/preflight.py`, non livré comme nouveau moteur E2E.
