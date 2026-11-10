# Local-full déclaratif (ADR-0609 Proposed)

Helm **4.3.1+gc0d08d0**, rendu client uniquement. Aucun install/upgrade, lookup,
hook, post-renderer, dépendance téléchargée, accès API ou DNS. Le nom de release
n'intervient pas dans les identités Kubernetes. Les bases legacy/P4 partagées
restent indépendantes ; l'ancien overlay local-full et ses générateurs ont été
retirés, sans branche de secours.

## Sources et outils

- `values.yaml` : objets Kubernetes complets, catégories et répliques finales.
- `../../profiles/local-full/*.yaml` : profils exclusifs ; `phases/*.yaml` :
  quiesce, foundation, dependencies, bootstrap, applications-first, final.
- `templates/resources.yaml` : sélection déclarative, images immuables, références,
  configuration et checksum public. Le PodTemplate est identique entre première
  réplique et état final : le scale-out ne redéclenche pas un nouveau rollout.
- `files/config/{iam.toml,oauth-connect.toml,envoy.yaml}` : sources publiques uniques
  pour ces configurations locales ; les autres fichiers publics sont référencés
  par `files` dans les déclarations. Leurs répertoires de provenance ne sont pas
  des noms reconstruits par Python ni des hashes de secrets.
- `files/openbao/bootstrap.json` : policies HCL, rôles/mounts/auth JSON, paramètres
  natifs CLI et références à la CA projetée. Le petit bootstrap applique ces
  documents ; aucune politique par service n'est assemblée en shell/Python.
- `files/lifecycle.yaml` : sélection des profils, contrats de sources privées et
  références de consommateurs publics. Chaque workload déclare `epoch`,
  `migrationScope`, `refreshTarget`, `principalPrefix` selon ses besoins.
- `files/storage-sources.yaml`, `templates/storage.yaml` : références des sources
  et objets distincts des outils backup/restore ; pas de Pod outil nominal.
- `files/cli.json` : options et valeurs par défaut natives argparse des trois
  façades ; Python lie seulement les types Path et les choix des catalogues.
- `files/ingress-attestation.json` : projections, champs constants et paire
  historique qualifiée du protocole ingress. Données suivies du dépôt, jamais
  une autorité fournie par un caller, l'environnement ou des valeurs Helm.

Vérifier l'archive officielle et son SHA256 avant usage, sans installation globale
implicite. `AIFORALL_HELM` désigne l'exécutable vérifié, sinon le PATH est utilisé.
Une autre version ou un outil absent bloque le wrapper, sans fallback.

```powershell
$env:AIFORALL_HELM = 'CHEMIN_EXPLICITE/helm.exe'
& $env:AIFORALL_HELM version --short
& $env:AIFORALL_HELM lint ops/deploy/charts/local-full -f ops/deploy/profiles/local-full/oauth-connect.yaml
& $env:AIFORALL_HELM template local-full ops/deploy/charts/local-full -f ops/deploy/profiles/local-full/oauth-connect.yaml -f ops/deploy/profiles/local-full/phases/final.yaml
python -B ops/deploy/render-local-full.py --oauth-connect --output-dir RENDU_NEUF
```

Le dernier wrapper conserve `render()` et les quatre sorties JSON historiques.
Il analyse/valide/sérialise le rendu, sans reclasser les objets par nom ni modifier
leurs specs. Les ensembles viennent de `aiforall.dev/category`. Les références
dynamiques autorisées (run, images, certificats publics, réseau attesté, références
TLS préservées) sont des valeurs publiques typées, jamais des credentials.

Les invariants publics (identités uniques, namespaces, références PVC/montages,
scopes migrateurs exclusifs, images immuables et absence de dépendance host)
sont vérifiés par le schéma et les templates Helm, pas par un second rendu ou
un validateur Python post-render. Les préimages live, elles, passent par la
même garde UID avant le plan et avant chaque écriture. Les UID Namespace/cluster
restent liés à leurs cartes d'autorité originales ; une ligne d'inventaire
contradictoire ne peut pas les remplacer. Les phases ne rendent que leur ensemble
utile, sauf les préflights complets et la vérification spécifique de préservation
TLS ; cette dernière retourne des références publiques sans réécrire de spec.

## Frontière privée

Le rendu public contient des **enveloppes sans octets secrets**, des marqueurs et
des références, pas les credentials OAuth/HMAC, clés ou empreintes de clés. Les
configurations sensibles sont dérivées du template public uniquement dans leur
Secret déclaré ; les applications/snapshots montent ce Secret en lecture seule.
Les identités privées TLS sont des entrées externes explicitement référencées ;
`files/private/opaque-secret.yaml` déclare leur forme commune. Le wrapper hydrate
seulement identité dynamique, données et owner UID lorsque nécessaire. Aucun
Secret live n'est lu pour reconstituer les sources.

`files/private/local-insecure-fixtures.json` et les fixtures du registre P4 sont
des valeurs de développement **déjà publiquement connues**, jamais des credentials
opérateur/production. Elles restent hors rendu/valeurs Helm. La clé de signature
IAM historique est lue depuis sa fixture existante, au point d'hydratation d'un
Secret distinct, plus dans le ConfigMap public. Aucun nouveau secret réel n'est
versionné ici. Les anciennes configurations des autres profils ne sont pas une
autorisation de copier leurs clés dans ce chart.

La PKI est décrite nativement dans
`ops/scripts/local-full-certificates.cnf` ; le helper OpenSSL conserve validation
CA/SAN/usages/validité et réemploi sans rotation. Pour base/GitHub, préparer
l'extension `--openbao-github` ; ajouter `--oauth-proof` pour GitLab ; OAuthConnect
utilise `--oauth-connect`. Rien n'est préparé implicitement par le renderer.

## Graphe, ownership et reprises

Les façades `just deploy-local-full`, `local-up`, `local-stop`, `local-status` et
les actions Python restent disponibles. Le déploiement prend le verrou exclusif,
puis invoque **`ops/deploy/local-full.just`** : plan → target → quiesce → foundation
→ dependencies → bootstrap → applications-first → final. Le graphe ne contient
aucun nom d'application. Une phase isolée exige la référence privée d'invocation,
le nonce du verrou original, le prédécesseur réellement déclaré et ses ACKs.
L'environnement enfant est limité aux variables OS/outillage publiques ; ni
dotenv, kubeconfig ambiant, override Docker, bearer ni credentials OAuth.

Les gardes conservent ancrage/UID/RV, refus de Job actif, préimages de quiescence,
attente de disparition effective des writers, bootstrap/root révoqué et ACK UID
avant transfert du Secret enfant. Des scopes migrateurs identiques sont refusés.
Les écritures ordinaires sont create-only ou UID/RV-bound ; les deltas réseau ont
aussi un test de spec et une vérification d'ACK. Timeout/Failed n'est pas Complete.
Une interruption laisse le reçu `in-progress` et interdit le replay : pas de
rollback DB/KV/PVC ni de restauration automatique dangereuse de répliques.
Ne pas effacer les marqueurs pour contourner ce blocage ; arbitrage du parent.

Même epoch saine : pas de seed/nouvelle version/redeploy. Une nouvelle epoch
renouvelle les seuls consommateurs déclarant `epoch`, après validation cohérente
des sources privées et seed CAS. Les ACK parent **et enfant** sont enregistrés ;
un ACK perdu, une tentative expirée ou une seconde epoch interrompant la première
bloquent, sans adoption/recréation. Les deux documents KV ne sont pas une
transaction. Les anciennes ressources legacy restent au bail, mais un profil
explicitement sélectionné limite les attentes/reprises. Le status historique
sans marqueur reste informatif sur son inventaire, sans inventer de sélection.

Une identité de tentative privée déjà utilisée est refusée avant quiescence.
Un ancien bail sans attestation des sources TLS ne permet pas une rotation ou
écriture implicite : utiliser le protocole de préservation OAuthConnect existant
lorsqu'applicable, sinon arbitrer explicitement. Une préservation ne prétend pas
attester un nouveau bundle comme source des anciens Secrets conservés.

`refresh` reste image-only : les déclarations chart/profils/just/CNI/Kind/registre
et la configuration PKI sont empreintées dans le bail final. Différence ou ancien
bail sans empreinte ⇒ refus avant pull/patch, même si les binaires n'ont pas
changé. Les cibles migration-free viennent de `refreshTarget`, pas d'une liste
de services Python.

Les exceptions ingress exigent l'attestation du protocole parent existant
`configure_envoy_ingress(...).finish(witness)` (identités, causalité et preuve
inchangées). Le succès persiste source, UID du nœud/policy, spec et provenance.
Une fondation qui rencontre une exception non attestée/périmée s'arrête avant
quiescence, plutôt que de la retirer ou élargir. Aucun `/32` historique portable.

## Exceptions Python délibérées et preuves

Restent : validations d'entrées/ACL/PKI, invocation des outils, ancrage/nonce/
UID/RV, cohérence des sources, ACK/CAS/transfert privé, reçus, protocole de reprise,
vérification d'images/provenance et flux binaires/consistance des sauvegardes.
Les helpers métier OpenFGA/OpenBao restent dans les fichiers publics montés par
les déclarations ; ils ne construisent pas de ressources. Backup/restore conserve
sa destination nouvelle, ses gardes PVC/PV/UID et des changements de répliques
conditionnés par UID/RV, jamais une adoption par nom.

Au pin OpenBao2.6.2, le client HTTP strict existant reste une exception fermée
pour l'authentification et les lectures de snapshots (§4.1 du contrat v1.5).
Aucun fallback, proxy ni nouvelle voie Agent n'est ajouté : `Clone()` perd le
contrôle `DisableRedirects`, donc un drapeau seul n'assure pas la garantie requise.
Les templates, politiques et configurations publics restent déclaratifs. IAM et
le simulateur ne reçoivent pas de token OpenBao ; Lazaret le reçoit déjà dans son
TOML, comportement de compatibilité conservé. Une future bascule Agent exige un
pin autorisé et les preuves natives login/read/redirections/proxy prévues au
contrat ; elle ne fait pas partie de cette livraison.

Oracles synthétiques indépendants `ops/tests/fixtures/local-full*-5199613.json` :
parité des identités/specs/TOML/Envoy et droits du commit initial. Dispositions
explicites : séparation clé IAM/template confidentiel/fixtures en Secrets RO,
marqueurs publics, références TLS de tentative, documents bootstrap standard,
annotations de catégorie/checksum ; pas de suppression globale de champs métier.
Le test de consommateur synthétique ajoute uniquement des déclarations et vérifie
rendu/phases/attentes/reprise sans liste/branche Python supplémentaire.

```powershell
python -m unittest discover -s ops/tests -p test_local_full_declarative.py
python -m unittest discover -s ops/tests -p test_local_full_phase_guards.py
```

Ces gates réels sont **offline**, pas des déploiements ni une qualification
OpenBao/Kubernetes/TLS/backup/navigateur live. Les campagnes réelles restent sous
autorisation et bail distincts du parent ; aucune destruction ou recréation n'est
autorisée par ce chart. ADR Proposed n'est pas implicitement Accepted.

Référence CLI JSON/CA par fichier vérifiée pour OpenBao 2.6.x :
<https://openbao.org/docs/2.6.x/commands/write/>.
