# Préflight du laboratoire et des builders Linux

Le cluster local-full n'existe pas et ses ports navigateur sont libres. Il peut
donc être créé selon le bootstrap préautorisé, sans adopter un ancien cluster.
Le cluster legacy déjà actif reste étranger à la mission et ne sera pas arrêté.

Les images applicatives du profil OAuth sont absentes. Le seul ancien artefact
de compilation présent n'a pas de provenance vérifiée : il ne sera pas réutilisé
pour les preuves courantes.

La lecture des étapes de livraison a trouvé le même écart Windows/Linux dans
deux builders secondaires : ils copiaient le target Windows mais cherchaient
leurs binaires dans le cache Linux. Le correctif reste limité au répertoire de
compilation Linux et au verrouillage de Cargo.lock. Il ne change ni les24 jobs,
ni une fonctionnalité d'Apparatus, ni le code d'authentification.

Le contrôle source détecte les deux écarts avant correction, puis les23 tests
de la suite passent. Une relecture infra ciblée est demandée ; aucun build réel
ni création de cluster n'est encore annoncé comme réussi.

## Suite

Figer ce petit delta de livraison, créer un worktree Git propre avec le SDK
publié, puis construire les images Linux en traçant leurs IDs. Les IT natives
restent valides : aucun code Rust ou réglage d'authentification n'a changé depuis.

## Références techniques

- Cible : kind-aiforall-local-full absente ; ports18080/18081 libres. Métadonnées uniquement, aucun appel API ou mutation de cluster.
- Conteneur legacy actif conservé : aiforall-local-control-plane, non possédé.
- Deux builders : workers/ext-authz/Dockerfile et workers/apparatus-operator/Dockerfile.controller ; sourceguard existant étendu, basesRust inchangées.

### Filtrage du contexte avant transfert

Le worktree propre contient aussi des archives de missions suivies par Git.
Le contrôle préalable a donc arrêté le transfert avant tout build Docker.
Plutôt que lire ou supprimer ces archives, les filtres Docker excluent maintenant
explicitement les états d'agents, preuves temporaires, worktrees et métadonnées
Git. Le contexte restera issu du worktree propre, jamais du répertoire canonique
contenant les modifications utilisateur.

Le nouveau contrôle échoue avant ce filtrage puis passe ; la suite compte
désormais24 succès. Aucun contenu privé n'a été envoyé à Docker à cette étape.

Le fichier `.dockerignore` canonique s'est révélé être un fichier local ignoré,
non suivi par Git. Son contenu antérieur est conservé, sans l'embarquer dans un
commit. Le filtre de mission est donc un nouveau template versionné, copié
exclusivement dans le worktree de livraison avec contrôle de hash ; le filtre
spécifique du controller reste versionné. Aucun forçage d'ajout ni nettoyage des
fichiers utilisateur n'est utilisé.
