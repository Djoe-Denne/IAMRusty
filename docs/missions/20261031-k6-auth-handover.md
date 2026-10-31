# Handover — campagne auth k6 Windows arrêtée

Arrêt demandé par l'utilisateur le 31 octobre 2026. **Aucun PASS auth réel.**

## Livré

- Scénarios : `ops/tests/k6/scenarios/auth/oauth-browser.js` et `envoy-san.js`.
- Parent : `ops/tests/k6/run-windows.ps1`, modes `auth-inspect` et `auth` ; configuration décrite dans `ops/tests/k6/AUTH-WINDOWS-PARENT.md`.
- Inspection native PASS ; revues statiques achevées, défaut de restauration corrigé. Les 17 IT historiques sont réutilisées sur sources pertinentes inchangées, pas réexécutées.
- Commits avant handover : contrat `dde0340`, conservation Kind `a43b0be`, scénarios `0c1e21d`, parent `51e74e1`, correctifs `d00ab08`.

## Blocage réel

Le bootstrap local-full a réussi ; la commande de déploiement a échoué après création de ressources. **Cause initiale inconnue** : le script opérateur temporaire a perdu le détail de l'erreur Python, PowerShell interrompant la capture sur stderr. Ne pas reprendre tel quel `.tmp/20261031-deploy-auth-local-full.ps1` pour diagnostiquer une erreur.

Après reprise en lecture seule : ancrage valide, deux Jobs de bootstrap réussis, IAM et Lazaret observés Ready. Le déploiement complet et le trajet Windows→18080 ne sont pas qualifiés. Aucun scénario auth, navigateur ou stimulus mauvais certificat exécuté.

## État laissé

- Tous les agents arrêtés ; trois nœuds Kind possédés arrêtés et vérifiés ; aucun k6/Chrome restant.
- Conteneurs, volumes, arbres auteurs et diagnostics conservés. Docker Desktop/WSL et ressources étrangères non touchés.
- Localisateur privé : `.tmp/k6-auth-runtime-20261031-d00ab08-index.json` (session, bail, kubeconfig, certificats, images et Config). Les certificats complets sont dans la destination indexée ; la première sortie partielle est conservée séparément.
- Registre : `.tmp/docker-resources-20261031.log`. Preuve d'arrêt : `mission-stop-summary.json` dans la session privée. Aucun bail consommateur actif laissé par la mission ; le fichier d'ancrage reste un état de récupération, pas une permission de redémarrage.
- Ledger détaillé : `.cursor/handoffs/20261029-k6-02-auth.md` ; rapports locaux sous `.opencode/missions/team_mvwirmg1_0000_znnodt/reports/`.

## Reprise par l'utilisateur

Vérifier le pipeline existant et l'entrée18080, puis exécuter les scénarios intégrés via le parent. Fournisseur synthétique18081, autorité8080/iam inchangée ; IAM mono-instance attendu par le raccord. Pas besoin de reconstruire les dix images applicatives : l'image IAM qualifiée est déjà épinglée dans le manifeste privé.

Pour de nouveaux certificats via Git Bash : le générateur existant a réussi avec `MSYS_NO_PATHCONV=1` limité à sa commande. Ne pas écraser les clés existantes.
