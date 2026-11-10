# k6 Windows — GitHub existant, fournisseur simulé

> Raccord Ops du 2026-11-09 : les configurations de l'ancien overlay sont déplacées dans le chart et ses profils ; les chemins cités restent consultables à `5199613`. Voir le [guide courant](../guides/ops-declarative-boundaries.md). Le déplacement n'est pas une exécution ou qualification k6.

> **Brouillon suspendu, non ratifié et non implémenté.** Après sa rédaction,
> l'utilisateur a interrompu le travail : GitHubConnect et GitLabConnect seront
> remplacés par un service OAuth 2.0 unique à interface encore inconnue ici.
> Les prescriptions liées aux anciens services ci-dessous sont historiques ;
> elles ne valent pas contrat d'implémentation pour la reprise. Relire d'abord
> le service unifié réellement livré, sans présumer sa configuration ou ses routes.

- Cible : instruction utilisateur du 2026-11-06, relayée dans le
  [handoff](../../.cursor/handoffs/20261106-k6-github-simulated-launch.md).
- Réalité : **contrat seulement ; aucun patch runtime ni run par cette rédaction**.
- Delta borné du [contrat auth](k6-windows-auth-functional.md), pas nouvelle ADR :
  fixture et scénario réversibles, aucune API applicative ni architecture changée.
  La qualification historique GitLab/SAN reste distincte, non transférée à GitHub.

## Frontière et raccord minimal

Conserver Kind dans Docker, IAM/GitHubConnect, OpenBao, images applicatives,
données et certificats existants. k6/CfT Windows et parent actuel conservés ; aucun
nouveau pilote Python, pipeline, service, build Rust ou permission globale.
L'objectif GitHub est explicite : pas de nouveau GO de configuration/démarrage.
Les vérifications de cible/propriété et le refus d'un prérequis absent demeurent.

Le parent distingue explicitement le scénario **GitHub simulé, sans SAN** du
parcours auth GitLab historique. Ne pas lancer `Invoke-K6AuthCampaign` inchangé :
il mute d'abord le certificat, puis impose des compteurs GitLab/PKCE. Un cas exclu
est hors périmètre, jamais réussi ni inclus silencieusement dans le total attendu.

Adaptations seules nécessaires, à réaliser par le parent :
- Profil GitHub explicite dans `ops/tests/support/oauth-idp-simulator.py` ; conserver
  le profil GitLab existant et ses validations S256, Bearer et `/api/v4/user`.
  Aucun autodétecteur permissif ni fallback entre profils.
- En profil GitHub : authorize et token existants peuvent conserver leurs chemins
  `/oauth/authorize` et `/oauth/token` ; ajouter `/user` et `/user/emails` selon les
  schémas réellement désérialisés par GitHubConnect. Accepter l'authentification
  client HTTP Basic effectivement émise, rejeter les credentials ambigus/doublés ;
  exiger `Authorization: token <access_token>` pour les deux GET. Jetons opaques
  émis par cette fixture, non expirés et corrélés, pas de valeur acceptée quelconque.
- Configuration fixture locale du connecteur : seules les trois URLs fournisseur
  pointent vers le simulateur existant, après vérification du routage navigateur
  et serveur. Conserver `secrets_mode = "openbao"`, les références de secrets,
  `pkce_supported = false`, redirects GitHub, issuer, TLS et volumes existants.
  Le profil GitHub n'exige donc pas S256 et ne produit aucun PASS PKCE fictif.
  Si le chargement des URLs exige un redémarrage, le parent le borne au workload
  concerné sous bail existant et vérifie la configuration effective, sans rebuild
  d'image, reset, nouvelle stack ou prétendue interception des endpoints GitHub réels.
- Scénario k6 et raccord parent : routes `/iam/api/auth/github/...`, identité dédiée,
  compteurs et projections GitHub explicites ; pas de remplacement global GitLab.
  Restaurer la configuration fixture temporaire et fermer les forwards possédés,
  même sur échec, sans supprimer/adopter les ressources préexistantes.

## Credentials : trois frontières, oracle indépendant

Avant stimulus, le parent fige en privé les valeurs **attendues préparées**, issues
de la préparation locale autorisée, et leur provenance indépendante de la lecture
OpenBao utilisée par le connecteur. Il compare ensuite les champs OpenBao effectifs
aux attendus : `secret/github-connect/oauth:client_secret` et
`secret/github-connect/iam:hmac_secret`. Aucun attendu reconstruit depuis la même
mauvaise lecture OpenBao que le client ; provenance indépendante absente = BLOQUÉ.
La fixture reçoit cet attendu OAuth figé, jamais le credential extrait de la requête
ni une valeur OpenBao substituée opportunément. Comparaison constante en temps du
`client_id` attendu et du **client_secret réellement reçu au token exchange**.

La clé HMAC IAM→GitHubConnect n'est ni le client_secret OAuth, ni l'access_token
fournisseur. Vérifier séparément la concordance de l'attendu HMAC, d'OpenBao et de
la configuration effective IAM ; conserver le `trim()` HMAC existant, aucune autre
normalisation. Le fournisseur ne reçoit pas cette clé. Un succès OAuth ne suffit
pas à prouver la provenance de la clé HMAC. L'access_token est celui émis par le
simulateur et doit être vérifié sur `/user` **et** `/user/emails`.

Les secrets OpenBao sont chargés au démarrage : modifier le coffre seul ne prouve
pas leur consommation. Ne pas tourner les clés pour ce test ; vérifier le chargement
effectif par les observations de frontière, sans exposer les valeurs, Basic, HMAC,
tokens, cookies, codes ou leurs empreintes publiques. Désaccord = refus explicite,
échec de scénario/campagne, jamais fallback vers secret direct ou ancienne valeur.

## Étapes et oracles fermés

1. Préflight privé : cible GitHub/configuration chargée, profils, routage, attendu
   indépendant versus coffre et IAM, bail/lock, outils épinglés. Ne pas créer/adopter
   `platform-mesh-certs` pour satisfaire le SAN historique. Verrou d'opération stale :
   résolution par le parent selon `local-full-storage.py::operation_lock`, sans
   réarmer l'autorité/consentement consommés de l'ancien run Python.
2. Positif Chromium : login GitHub ; si inscription requise, la terminer puis faire
   un nouveau login réussi ; identité exacte et cookies/redirections conservés.
   Réassociation avec vrai JWT puis callback sans Bearer, même identité et emails.
   Capturer authorize/token/user/emails et dispatch IAM corrélés au même run.
3. Négatifs : routes protégées GitHub sans Bearer, statut exact confirmé au contrat
   chargé, zéro échange fournisseur ; mauvais client_secret refusé au token endpoint,
   sans token émis ; mauvais access_token refusé sur chacun des deux GET. Utiliser
   des codes distincts non réutilisés et des valeurs volontairement différentes,
   sans modifier les secrets du workload. Ces sondes fournisseur prouvent le refus
   de la fixture, pas un parcours IAM avec une clé chargée volontairement erronée.
   Tout mismatch du vrai échange connecteur→fixture fait échouer le positif.
4. Figer avant run les nombres exacts attendus par parcours et phase, y compris
   négatifs et inscription réelle ; ne pas recopier les `3` PKCE / `7` IAM / `4`
   ext_authz GitLab. Compteurs de tentatives/refus et succès séparés, aucun mélange
   entre sondes et navigateur. Un cas manquant, preuve périmée ou absence de refus
   attendu est ROUGE ; prérequis absent = BLOQUÉ, jamais PASS.

Validation parent : tests ciblés du simulateur (GitHub positif, mauvais secret,
token invalide/expiré, double auth, non-régression GitLab S256), tests parent
(sélection GitHub sans appel SAN, erreur de clé/complétude), inspection k6 puis run
réel sous gardes existantes. Aucun test lancé ici. Verdict limité à ces cas :
**aucun PASS des 39 checks mesh, 6 Apparatus, SAN, PKCE GitLab ou GitHub réel déduit**.

Sources : `ops/tests/k6/lib/windows-auth.ps1::Invoke-K6AuthCampaign`,
`ops/tests/k6/scenarios/auth/oauth-browser.js`, simulateur cité ci-dessus,
`ops/deploy/apps/overlays/local-full/github-openbao/github.toml` ; trajet Rust
`GitHubConfig` / `GitHubConnectClient` fourni comme vérifié par le parent
(`services/GitHubConnect/{configuration/src/lib.rs,infra/src/github.rs}`).
Le [contrat startup OpenBao](startup-secrets-openbao.md) distingue chargement au
boot et refresh absent ; aucune acceptation implicite de son ADR-0505 Proposed.
