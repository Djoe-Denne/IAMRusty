# Authentification fonctionnelle k6 Windows — raccord minimal

- Statut : **Accepted** ; Réalité : **non implémentée/non exécutée par ce document**.
- Date : 2026-10-31 ; mission k6-02 ; base `05941d1`, SDK `02e5`.
- Référence d'admission et de rapprochement : [ledger](../../.cursor/handoffs/20261029-k6-02-auth.md).
- Ratifié par le lead le 2026-10-31 sous la révision1 approuvée de mission2,
  après lecture des deux propositions architecte convergentes et rapprochement
  de la [matrice de parité](../missions/20261031-k6-auth-parity.md).
  Cette adaptation conserve les responsabilités existantes et n'admet aucune
  cible vivante ; intégration requise avant tout code consommateur.
- Canon conservé : [Windows Accepted](k6-windows-functional.md), notamment son
  amendement révision3 ; [version navigateur](k6-windows-browser-version.md),
  [ADR0504](../adr/0504-validation-fonctionnelle-linux-k6.md) et
  [ADR0412](../adr/0412-oauth-transaction-persistante-liee-navigateur.md).

## Portée et invariants

Raccorder les scénarios auth historiques au parent Windows existant, sans nouvelle
architecture de tests. Seules les adaptations indispensables à leur exécution
fidèle sont admises après ratification ; pas de requalification générale du socle,
nouveau framework, fixture, backend ops, laboratoire ou pilote de remplacement.
Les anciens pilotes restent conservés. Le parent Linux/WSL
`ops/tests/k6/local-private-parent.ps1` n'est pas une voie Windows autorisée.

Conserver k6 **2.3.0**, CfT **156.0.8078.73**, manifeste Windows et vérification
existants ; `browser.version()` dans le même cas, protections Chromium natives,
contexte neuf, aucun `no-sandbox` ni contournement TLS. Ne pas ressusciter les ACL,
verrous ou attestations renderer abandonnés. Windows seul qualifié ; Linux/WSL différés.

Ne pas remplacer l'autorité `http://127.0.0.1:8080/iam` par le point d'accès
navigateur `http://127.0.0.1:18080`. Conserver les redirects historiques sur18080
et le fournisseur attendu `http://localhost:18081/oauth/authorize`, sans fallback
d'autorité, réécriture opportuniste d'URL ou exemption gateway.

## Admission et entrées minimales

Le parent établit avant tout appel : cible **existante**, autorisée et observée,
identité/config chargée Envoy/IAM/fournisseur, joignabilité depuis Windows, baux et
propriété applicables, outils exacts vérifiés et chemins privés hors Git.
Un booléen fourni par l'enfant ou un manifeste auto-attesté ne vaut pas observation
de cible/propriété. Absence de cible, identité ou mécanisme parent nécessaire :
**prérequis manquant, zéro appel**, pas création de backend ou réveil Kind/WSL.

Entrées enfant strictement nécessaires : run_id courant, scénario fermé attendu,
origines fixes validées, identité dédiée corrélée (`runId@example.invalid`), chemins
privés de sortie et attente de version. Secrets et références privées restent
dans les canaux privés existants ; aucun secret dans argv, résumé ou dépôt.
Environnement allowlisté et garde de cible restent chez le parent ; pas de commande
libre, URL arbitraire ou privilège de mutation délégué au JS.

L'enfant observe parcours HTTP/navigateur, statuts, présence/absence et attributs
des cookies/headers, identité métier et compteurs fermés. Le parent rapproche ces
observations avec configuration réellement chargée, dispatch IAM, transactions,
consommations et échanges fournisseur du **même run**. Décoder des claims est une
observation, jamais une validation JWT locale remplaçant celle du gateway.

## Matrice de conservation des oracles avant portage

Source navigateur : `ops/tests/oauth-browser-local-full.js`, notamment lignes54–119.
La matrice complète de recherche indépendante est rapprochée dans le document
lié ci-dessus ; une ligne non localisée/observée reste non qualifiée. Le dernier
run historique utilisait un compte déjà inscrit : les compteurs du parcours
inscription puis nouveau login doivent être vérifiés contre le scénario final
et les routes chargées avant run, pas copiés du run historique.

- **Inscription/P2 → k6 Chromium** : login initial202 signifie préparation
  `registration_required`, POST complete-registration200/201, puis **nouveau**
  login achevé200, operation login, identité dédiée exacte.202 seul n'est pas P2.
- **P1/cookies/redirections → k6 Chromium** : login sans Bearer,303 et cookie
  iam-oauth-browser HttpOnly, SameSite=Lax, Max-Age600, Path=/iam, host-only,
  sans Secure dans ce parcours HTTP local ; nonce natif du même contexte.
  Callback exact attendu ; aucun Bearer ni Set-Cookie au callback ; aucune
  credential transmise au fournisseur. Pas de cookie forgé/Authorization global.
- **P3/réassociation → k6 Chromium + parent** : START relink-start authentifié
  avec le vrai JWT du login ;200/auth_url fournisseur exacte ; navigation inchangée,
  callback `/iam/api/auth/gitlab/relink-callback` sans Bearer,200, même user/email
  et liste emails. Le gateway valide réellement ce JWT ; pas de succès synthétique.
- **N1/N2/N3 → k6 Chromium ou API pour la requête, parent pour discrimination** :
  GET sans Bearer sur `/iam/api/auth/gitlab/link`, `/iam/api/auth/gitlab/relink-start`,
  `/iam/api/auth/gitlab/relink/callback` : **403 attendu**, aucun Set-Cookie,
  dispatch/transaction/consommation IAM ou échange fournisseur. Le401/403 toléré
  par l'ancien pilote est historique, pas un élargissement du verdict qualifié.
- **PKCE/fournisseur → parent ops corrélé** : authorize/token/profile et validation
  PKCE du même run, compteurs exacts attendus non nuls, au moins deux authorize
  pour login puis réassociation ; les comptes précis dépendent de l'inscription
  effectivement requise et doivent être figés avant run. Un compteur global non
  corrélé ou la seule observation navigateur ne suffit pas.
- **SAN Envoy → parent ops + k6 API pour stimulus HTTP** : bon certificat, mauvais
  nom signé par la même CA, bon restauré ; mutation/restauration exclusivement
  parent avec garde, propriété et ACK applicables. Hausse du compteur de refus SAN
  approprié et preuve Envoy du même stimulus ;503 seul insuffisant. Distinguer ce
  cas `envoy-san` des preuves de signataire remote ; aucun backend nouveau si le
  mécanisme parent autorisé manque. Restauration même après setup interrompu.
- **0412 → UT/IT conservées, non réécrites en k6** :17IT historiques9+6+2 et preuves
  principal/remote restent à leur niveau. Réutilisation seulement après comparaison
  des sources/parcours/matchers pertinents ; pas une preuve recalculée aujourd'hui.

## Commandes à figer avant run, sans lancement ici

Le parent courant `ops/tests/k6/run-windows.ps1` n'exécute que le smoke browser ;
ses modes validate/run/selftest refusent la cible à ce stade. Ne pas présenter une
invocation auth comme déjà disponible et ne pas appeler k6 directement en contournant
ses gardes. Réutiliser `Invoke-K6Child`, environnement privé et supervision existants.

Avant tout run, le lead consigne les **commandes effectives complètes** : chemin
PowerShell/parent et paramètres admis après adaptation minimale, chemins exacts
k6/CfT vérifiés, cwd, config/cible autorisée et chemins privés ; puis les argv enfant
`version`, `inspect <scénario-copié>` et `run --quiet --address 127.0.0.1:0
--system-tags name,check,scenario,group <scénario-copié>` réellement construits par
le parent pour chaque cas API/Chromium. Ce sont des formes observées dans le socle,
**pas des commandes auth finales** : noms/options/cartes de scénarios doivent être
figés après implémentation, sans supposer une CLI dans PATH. Inclure également les
commandes parent réelles de stimulus/restauration SAN et de collecte corrélée.
Tant que cette fiche privée et sa projection expurgée ne sont pas établies : pas de run.

## Verdict, validation et hors scope

Attendus/démarrés/achevés exacts pour tous les cas, checks et thresholds fermés,
compteurs exacts non nuls, résumé neuf lié au run ; setup interrompu, cas absent,
summary ancien/vide ou corrélation manquante ne sont jamais positifs. Export public
borné aux résultats expurgés ; aucun JWT/cookie/code/state/clé, URL sensible,
trace/HAR ou stream brut publié. Cleanup/restauration parent sur succès et échec,
inventaire final et zéro ressource possédée active sans bail ; pas d'adoption par nom.

Contrôle documentaire : `git diff --check`. Revue indépendante bornée de parité et
des oracles modifiés, pas audit du framework. Aucune suite systématique ajoutée.
Seulement si le delta invalide les preuves pertinentes :
`cargo test -p iam-service --test oauth_browser_transactions`,
`cargo test -p iam-service --test auth_transactions_postgres`,
`cargo test -p iam-service --test oauth_cleanup_lifecycle` selon le risque réellement
touché, sur Windows et un seul Cargo. Défaut produit = ROUGE remonté, sans correction
silencieuse ni assouplissement d'oracle. Aucun run/test/runtime dans cette rédaction.

Pas de nouvelle ADR : raccord réversible déjà borné par les contrats existants,
sans nouvelle frontière ni autorité. Aucun contrat Accepted modifié ; statut
Accepted pour ce raccord borné après rapprochement ; aucune qualification runtime
ou admission de cible déduite de cette ratification documentaire.
