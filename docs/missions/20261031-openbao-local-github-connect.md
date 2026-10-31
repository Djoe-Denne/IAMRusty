# Mission suivante — Raccord OpenBao local / CI pour GitHubConnect

## État et objectif

**Préparation seulement** : aucun déploiement, runtime, build d'image ou implémentation autorisé par cette fiche à ce stade. Base : AIForAll `c3da69f`, GitHubConnect livré en `21caa8b`, SDK consommé `93b890f`. Préserver le travail étranger et les anciens laboratoires.

Fournir au connecteur le coffre, la confiance TLS, les références et le fichier d'identité attendus par le [contrat SDK/service](../contracts/startup-secrets-openbao.md). Le même mécanisme doit pouvoir servir au développement Windows et à une CI Linux. Les adaptateurs cloud et la mise en production d'OpenBao restent ultérieurs ; un coffre DEV n'est pas une garantie de durabilité/HA/HSM.

Choix de raccord et invariants détaillés : [contrat proposé de déploiement](../contracts/openbao-local-githubconnect-deployment.md). Les ADR Proposed ne deviennent pas Accepted par cette préparation.

**Choix de sécurité acceptés par l'utilisateur :** root généré privé réservé au bootstrap ; credential Lazaret limité à son besoin ; credential GitHubConnect déjà borné ; TLS et réseau restreint. Le durcissement rejoint ce lot préparatoire, avant l'import de vrais credentials. Il n'est pas déjà implémenté : les premières preuves utilisent des sentinelles synthétiques. Les détails restent Proposed, notamment fenêtre Lazaret, matérialisations seed et configurations runtime privées en RAM (IAM/Lazaret), sans promesse d'absence de copie etcd/backups. Cet accord n'est ni un GO runtime ni une acceptation des ADR en bloc.

## Contrat d'entrée déjà livré — à ne pas refaire

- Mode `openbao` explicite ; aucune valeur directe OAuth/HMAC concurrente.
- Deux documents KV v2 distincts : `secret/github-connect/oauth` champ `client_secret`, `secret/github-connect/iam` champ `hmac_secret` ; ce dernier doit correspondre exactement au HMAC utilisé par IAM.
- Fichier de token externe non-root, TTL restante `1..3600`, vérifiée par `lookup-self` ; droits de lecture sur les deux documents et lookup uniquement.
- HTTPS vérifié ; l'exception HTTP ne vaut que pour une IP littérale loopback, **pas** `http://openbao:8200`.
- Snapshot des deux secrets au démarrage ; pas de renouvellement/rotation applicative. Réutiliser les preuves Windows de la mission précédente, sans les présenter comme un test de déploiement réel.

## Lots d'implémentation à préparer

1. **Durcissement du coffre et transport.** Générer le root privé pour le bootstrap du coffre, ne pas le mettre en argument, source, config applicative ou logs ; le révoquer après initialisation et vérifier son refus avant import réel. Un redémarrage du coffre volatile rejoue explicitement ce cycle, sans réutiliser les anciens credentials. Exposer OpenBao en HTTPS avec CA/SAN corrects pour tous les clients autorisés ; HTTP reste au seul bootstrap en loopback, pas entre Pods. Ajouter les certificats nécessaires sans écraser les anciennes clés/CA ni provoquer une rotation implicite.
2. **Identités applicatives séparées.** Lazaret perd le root et reçoit seulement la lecture de ses chemins nécessaires (actuellement `secret/data/ops/token`). Son client garde le bearer en mémoire : la proposition locale bornée à 8 h exige login/rollout avant échéance ou après restart du coffre, sans annoncer de refresh transparent. Cette durée reste à confirmer. GitHubConnect conserve son identité ≤1 h et son agent durable, avec token-file privé en mémoire utilisable aussi après restart du seul conteneur. Aucun des deux credentials ne permet de lire les secrets de l'autre ou d'administrer le coffre.
3. **Bootstrap des données sensibles.** Après révocation du root, un seed distinct non-root initialise les deux documents depuis une source privée locale ou GitHub Actions Secrets, d'abord synthétiques pour prouver le durcissement. Vrais credentials seulement après vérification et autorisation du parcours réel. Ne pas copier tout le `.env` dans le Pod/image. Création réentrante : références existantes incompatibles => refus, jamais écrasement/rotation silencieuse. Le bootstrap privilégié et le seed applicatif restent séparés des identités de lecture des services.
4. **Raccord Kubernetes et IAM.** Ajouter un profil GitHub explicite, ses volumes/configuration, policies réseau et référence d'image ; préserver le profil `oauth-proof` GitLab. Proposition : fournir à IAM un snapshot du même HMAC dans sa configuration runtime privée en RAM, sans modifier son code ni dupliquer une autorité ESO pour les valeurs déjà résolues par le SDK. Ce rendu et le transfert du seed restent des choix à approuver.
5. **Entrées hôtes et images.** Garder une commande de raccord sans logique métier dupliquée entre Windows/Linux. Le déployeur actuel appelle `powershell.exe` pour Kind ; le CLI Python sous-jacent ne réalise pas à lui seul toute l'initialisation CNI/Calico/attentes du wrapper. Identifier et traiter le raccord portable complet nécessaire, ou déclarer clairement l'exécution Linux bloquée, sans contourner les gardes de cible. Consommer une image GitHubConnect contenant le code livré ; construire seulement les artefacts nécessaires s'ils manquent, puis les épingler. Pas de reconstruction systématique de toutes les images.

## Surface pressentie

Profil/configuration local-full, scripts de bootstrap/seed et manifeste d'images sous `ops/deploy/`, générateur commun de certificats si son extension est nécessaire, documentation et entrée CI ciblée. Le SDK et le code métier GitHubConnect ne changent pas sauf incompatibilité concrète à remonter avant extension de portée. Delta Lazaret à expliciter/autoriser : `VaultConfig` expurgé en Debug/sérialisation avant d'y porter un vrai bearer, et éventuellement ajout local d'une CA explicite si le backend TLS de l'image ne permet pas un raccord Ops vérifié. Pas de migration SDK ou de nouveau port/refresh général. Si une nouvelle image Lazaret est nécessaire, ne reconstruire que les artefacts concernés, pas tous les services.

`services/GitHubConnect/Dockerfile` copie un binaire préconstruit : reconditionner une ancienne image artefacts ne suffit pas. `Dockerfile.build` exporte actuellement tous les exécutables présents à la racine du cache release : si un build ciblé est ajouté, exporter/qualifier uniquement ses binaires attendus, sans attribuer le nouveau hash source aux autres anciens exécutables du cache. Un ID Docker épinglé prouve une identité d'image, pas à lui seul son origine source. `local-full-images.py` ne connaît actuellement que le nominal et l'ajout GitLab ; GitHub doit être explicite. Conserver les exclusions `.env`/états privés du contexte Docker de livraison et ses caches Linux séparés du target Windows.

## Vérification bornée

- Avant runtime : contrôles de configuration/rendu, absence de secret dans les sorties, compatibilité des commandes Windows/Linux et conformité au contrat SDK. Pas de nouvelle batterie de tests du framework.
- Après implémentation, intégration et gates applicables : un parcours réel OpenBao HTTPS → identité workload → fichier → boot du connecteur, avec secret OAuth/HMAC effectivement lus. L'éventuel test GitHub fournisseur reste distinct : des valeurs synthétiques peuvent qualifier le raccord au coffre, **pas** un OAuth GitHub réel.
- Vérifier le refus d'un accès hors policy et d'une confiance TLS incorrecte aux frontières ajoutées, puis le redémarrage applicatif avec credential renouvelé. Une injection directe des valeurs attendues ne remplace pas cette preuve.
- Vérifier root privé absent des sorties et révoqué, permissions Lazaret/GitHub mutuellement bornées, absence de fallback root après expiration/restart et maintien des parcours Transit/GitLab. Une preuve accélérée ne doit pas être présentée comme une endurance de 8 h.
- Rapporter Windows et Linux séparément, ainsi que LIVE / SIMULÉ / NOT RUN et les limites. Aucun PASS auth global déduit de Pods Ready ou d'un rendu réussi.
- Runtime uniquement sur autorisation explicite du lot d'exécution : inventaire/bail avant, IDs/UID tracés, registre à la création, stop des seuls possédés à la fin. Ne pas réveiller ou réinitialiser les laboratoires conservés pour préparer le plan.

## Hors périmètre

Flux GitHub App installation/API, seed métier général, choix une/deux Apps, migration SDK Lazaret/IAM, ESO généralisé, provisioning cloud, PKI de production et rotation à chaud. Le lot produit un raccord consommable par les futurs tests E2E, pas un nouveau framework de campagne.
