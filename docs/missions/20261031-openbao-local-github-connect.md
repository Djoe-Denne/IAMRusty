# Mission suivante — Raccord OpenBao local / CI pour GitHubConnect

## État et objectif

**Préparation seulement** : aucun déploiement, runtime, build d'image ou implémentation autorisé par cette fiche à ce stade. Base : AIForAll `c3da69f`, GitHubConnect livré en `21caa8b`, SDK consommé `93b890f`. Préserver le travail étranger et les anciens laboratoires.

Fournir au connecteur le coffre, la confiance TLS, les références et le fichier d'identité attendus par le [contrat SDK/service](../contracts/startup-secrets-openbao.md). Le même mécanisme doit pouvoir servir au développement Windows et à une CI Linux. Les adaptateurs cloud et la mise en production d'OpenBao restent ultérieurs ; un coffre DEV n'est pas une garantie de durabilité/HA/HSM.

Choix de raccord et invariants détaillés : [contrat proposé de déploiement](../contracts/openbao-local-githubconnect-deployment.md). Les ADR Proposed ne deviennent pas Accepted par cette préparation.

**Arbitrage avant implémentation :** le coffre DEV actuel possède un root connu, également donné à Lazaret. Ce premier raccord ne doit recevoir que des **secrets synthétiques**. TLS et le rôle limité du connecteur ne suffisent pas à rendre sûrs de vrais credentials GitHub dans ce coffre ; leur utilisation exige d'abord un durcissement distinct. Le plan propose également un transfert temporaire par Secret Kubernetes pour le seed et un TOML runtime IAM privé en RAM pour le HMAC : ce ne sont ni des secrets versionnés ni une garantie d'absence de matérialisation dans etcd/backups. Ces choix restent à approuver, pas à considérer acquis.

## Contrat d'entrée déjà livré — à ne pas refaire

- Mode `openbao` explicite ; aucune valeur directe OAuth/HMAC concurrente.
- Deux documents KV v2 distincts : `secret/github-connect/oauth` champ `client_secret`, `secret/github-connect/iam` champ `hmac_secret` ; ce dernier doit correspondre exactement au HMAC utilisé par IAM.
- Fichier de token externe non-root, TTL restante `1..3600`, vérifiée par `lookup-self` ; droits de lecture sur les deux documents et lookup uniquement.
- HTTPS vérifié ; l'exception HTTP ne vaut que pour une IP littérale loopback, **pas** `http://openbao:8200`.
- Snapshot des deux secrets au démarrage ; pas de renouvellement/rotation applicative. Réutiliser les preuves Windows de la mission précédente, sans les présenter comme un test de déploiement réel.

## Lots d'implémentation à préparer

1. **Transport et certificats.** Exposer OpenBao en HTTPS avec CA/SAN corrects. Prévoir les clients actuels (seed, Lazaret, utilisateurs Transit) sans affaiblir TLS ou les contourner. Ajouter les identités de certificat nécessaires sans écraser les clés/CA existantes ni provoquer une rotation implicite.
2. **Identité du connecteur.** Authentifier son workload avec une identité/policy minimale et fournir le token-file via un volume privé, en mémoire lorsque possible. Le fichier doit être prêt avant le boot et encore valide lors du redémarrage du seul conteneur applicatif, pas seulement à la première création du Pod. Pas de token root ou de token fixe expiré après une heure.
3. **Bootstrap des données sensibles.** Initialiser les deux documents depuis une source privée locale ou GitHub Actions Secrets, **synthétiques uniquement tant que le root DEV est exposé**. Ne pas copier tout le `.env` dans le Pod/image. Création réentrante : références existantes incompatibles => refus, jamais écrasement/rotation silencieuse. Le bootstrap initial est séparé de l'identité de lecture du connecteur.
4. **Raccord Kubernetes et IAM.** Ajouter un profil GitHub explicite, ses volumes/configuration, policies réseau et référence d'image ; préserver le profil `oauth-proof` GitLab. Proposition : fournir à IAM un snapshot du même HMAC dans sa configuration runtime privée en RAM, sans modifier son code ni dupliquer une autorité ESO pour les valeurs déjà résolues par le SDK. Ce rendu et le transfert du seed restent des choix à approuver.
5. **Entrées hôtes et images.** Garder une commande de raccord sans logique métier dupliquée entre Windows/Linux. Le déployeur actuel appelle `powershell.exe` pour Kind ; le CLI Python sous-jacent ne réalise pas à lui seul toute l'initialisation CNI/Calico/attentes du wrapper. Identifier et traiter le raccord portable complet nécessaire, ou déclarer clairement l'exécution Linux bloquée, sans contourner les gardes de cible. Consommer une image GitHubConnect contenant le code livré ; construire seulement les artefacts nécessaires s'ils manquent, puis les épingler. Pas de reconstruction systématique de toutes les images.

## Surface pressentie

Profil/configuration local-full, scripts de bootstrap/seed et manifeste d'images sous `ops/deploy/`, générateur commun de certificats si son extension est nécessaire, documentation et entrée CI ciblée. Le SDK et le code métier GitHubConnect ne changent pas sauf incompatibilité concrète à remonter avant extension de portée. Aucun changement de transit IAM, port Lazaret ou autre service pour obtenir une clôture artificielle.

`services/GitHubConnect/Dockerfile` copie un binaire préconstruit : reconditionner une ancienne image artefacts ne suffit pas. `Dockerfile.build` exporte actuellement tous les exécutables présents à la racine du cache release : si un build ciblé est ajouté, exporter/qualifier uniquement ses binaires attendus, sans attribuer le nouveau hash source aux autres anciens exécutables du cache. Un ID Docker épinglé prouve une identité d'image, pas à lui seul son origine source. `local-full-images.py` ne connaît actuellement que le nominal et l'ajout GitLab ; GitHub doit être explicite. Conserver les exclusions `.env`/états privés du contexte Docker de livraison et ses caches Linux séparés du target Windows.

## Vérification bornée

- Avant runtime : contrôles de configuration/rendu, absence de secret dans les sorties, compatibilité des commandes Windows/Linux et conformité au contrat SDK. Pas de nouvelle batterie de tests du framework.
- Après implémentation, intégration et gates applicables : un parcours réel OpenBao HTTPS → identité workload → fichier → boot du connecteur, avec secret OAuth/HMAC effectivement lus. L'éventuel test GitHub fournisseur reste distinct : des valeurs synthétiques peuvent qualifier le raccord au coffre, **pas** un OAuth GitHub réel.
- Vérifier le refus d'un accès hors policy et d'une confiance TLS incorrecte aux frontières ajoutées, puis le redémarrage applicatif avec credential renouvelé. Une injection directe des valeurs attendues ne remplace pas cette preuve.
- Rapporter Windows et Linux séparément, ainsi que LIVE / SIMULÉ / NOT RUN et les limites. Aucun PASS auth global déduit de Pods Ready ou d'un rendu réussi.
- Runtime uniquement sur autorisation explicite du lot d'exécution : inventaire/bail avant, IDs/UID tracés, registre à la création, stop des seuls possédés à la fin. Ne pas réveiller ou réinitialiser les laboratoires conservés pour préparer le plan.

## Hors périmètre

Flux GitHub App installation/API, seed métier général, choix une/deux Apps, migration SDK Lazaret/IAM, ESO généralisé, provisioning cloud, PKI de production et rotation à chaud. Le lot produit un raccord consommable par les futurs tests E2E, pas un nouveau framework de campagne.
