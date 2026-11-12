# Trois scénarios k6 sur le lab HTTPS natif

`campaign.ps1` prépare une slice IAM/OAuthConnect v2/Envoy/IdP, exécute les
trois scénarios conservés, puis ferme ses ressources même après échec. Il ne
démarre, ne répare et n'arrête **jamais le lab**. Le simulateur Python reste
une fixture protocolaire ; le parent et le provisioning sont PowerShell/CLI.

## Entrées explicites

Un JSON privé hors Git fournit :

- `operator_directory` : dossier opérateur natif avec `apps/iam/`,
  `apps/envoy-mesh/envoy.yaml` et `pki/`. Les entrées originales restent intactes.
- `kubeconfig` : fichier visant `kind-aiforall-local-full`.
- `tools` : `k6`, `chromium`, `kubectl`, `docker`, `helm`, `openssl`, chacun
  avec `path` absolu et `sha256`. Aucun téléchargement ou fallback d'outil.
- `images` : références explicites `iam`, `oauth`, `envoy`, `idp`. Les images
  applicatives doivent déjà être chargées dans Kind. L'image IdP est une image
  Python officielle épinglée par digest, pas une image applicative reconstruite.

Prérequis : Windows x64 non élevé, k6 2.3.0, Chromium 156.0.8078.73,
trois nœuds déjà running, Postgres et ext-authz nominaux disponibles, ports
loopback 18443/18445/19901 libres. Aucun Secret Kubernetes n'est lu.

Le navigateur complet est requis, pas seulement `chrome.exe`. Une archive locale
déjà épinglée peut être extraite sans téléchargement ni exécution par
`provision-chromium.ps1 -Archive <zip-absolu> -Directory <nouveau-dossier-prive>`.
Ses champs `path`, `sha256`, `bundle_manifest`, `bundle_sha256` alimentent
`tools.chromium` ; tous les fichiers compagnons sont ensuite re-vérifiés.

IAM reste sur `https://127.0.0.1:18443`, IdP sur **`https://localhost:18445`**.
Cette séparation de host empêche d'envoyer le cookie HTTPS `__Host-…` (Path=/)
au fournisseur ; deux ports du même host ne l'isoleraient pas. Le certificat
IdP contient DNS:localhost ; les forwards restent liés à127.0.0.1.

```powershell
./ops/tests/k6/native/campaign.ps1 -Action prepare `
  -Config C:/private/native-k6-input.json -Directory C:/private/fresh-campaign

./ops/tests/k6/native/campaign.ps1 -Action run `
  -Directory C:/private/fresh-campaign `
  -ExclusiveDatabaseDdlForThisRun

# Reprendre seulement la fermeture après interruption/échec, jamais rejouer run.
./ops/tests/k6/native/campaign.ps1 -Action clean `
  -Directory C:/private/fresh-campaign -ExclusiveDatabaseDdlForThisRun
```

`prepare` fait des observations du lab et génère les fichiers privés, mais ne
crée ni DB, workload ou confiance CA. Un nouveau run exige un nouveau dossier ;
les sources, artefacts, entrées et outils sont empreintés. Les diagnostics et
reçus restent privés. Ne pas publier/archiver récursivement ce dossier.

## Accords et limites

**Local, décision du 12 novembre 2026 :** installer une fois la CA publique dans
le profil, en vérifiant son SHA256 **DER**, avant les campagnes :

```powershell
./ops/tests/k6/native/trust-ca.ps1 -Action install-profile `
  -CertificateFile C:/private/lab/pki/mesh/ca.crt -ExpectedSha256 <sha256-der-verifie>
```

L'installation peut demander une confirmation Windows, sans test/forward actif.
Ensuite lancer `run` **sans** `-TrustLabCaForThisRun` : la CA préexistante reste
en place. `trust-ca.ps1 -Action verify` vérifie seulement cette confiance.
La CI est reportée ; un éventuel secret GitHub sera étudié séparément, sans
export automatique des clés locales ni désactivation de TLS.

- `-TrustLabCaForThisRun` autorise uniquement la CA publique désignée, dans
  `CurrentUser\Root`, juste avant les essais, puis son retrait dans `finally`.
  Une confiance préexistante n'est jamais retirée. Windows peut afficher sa
  propre confirmation ; ne pas la contourner ni utiliser LocalMachine.
- `-ExclusiveDatabaseDdlForThisRun` atteste une fenêtre sans autre opérateur
  CREATE/DROP/RENAME de la DB `k6_<run>`. CREATE refuse les collisions, le reçu
  mémorise l'OID ; le DROP vérifie cet OID et zéro connexion après arrêt des
  writers, sous verrou advisory. **Contrôle puis DROP par nom non atomiques** :
  sans cet accord, ne pas créer la DB. Jamais FORCE, terminaison de sessions ou
  écriture dans `iam_dev`.
- Le DELETE Kubernetes transmet une précondition UID à l'API, pas un simple
  contrôle suivi d'un delete par nom. Aucune adoption de ressource en collision.
- Le mauvais SAN doit être restauré et son retour vérifié avant suppression.
  Une restauration impossible laisse une fermeture en échec, pas un faux PASS.
- L'edge propre conserve la politique et les autorités de la source Envoy ;
  seule la destination réseau IAM est remappée. Les probes de fixtures sont
  TCP pour ne pas polluer ses compteurs, sans prétendre vérifier l'identité.
- La campagne qualifie cette slice, **pas la disponibilité de l'edge nominal**.
  Le mode opérateur IAM `local_insecure` reste une limite du lab ; pas de bypass
  TLS du navigateur, d'HMAC/PKCE ni de faux validateur de JWT.

Un PASS exige health, les 26 checks OAuth, good/bad/restored SAN, les deltas
parent corrélés, même incarnation Envoy et cleanup réel. Un test unitaire,
`k6 inspect`, un scénario sauté ou les 12 checks stateless ne le remplacent pas.
L'ancien parent n'est retiré qu'après cette qualification.

Qualification locale du 12 novembre : les trois scénarios et le cleanup ont
réellement passé ; voir `docs/missions/20261112-k6-three-native-results.md`.
La CI et le retrait des consommateurs legacy restent reportés.

Contrat : `docs/contracts/k6-three-scenario-native-parity.md` (**Proposed**).
