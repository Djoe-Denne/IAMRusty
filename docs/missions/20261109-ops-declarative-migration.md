# Migration Ops vers les formats déclaratifs

## Mandat

Le 9 novembre 2026, l'utilisateur a demandé l'implémentation dans cette session : tout ce qui peut être décrit en infrastructure as code doit l'être ; Python doit principalement appeler les outils standard et conserver les contrôles indispensables. L'utilisateur fixe les priorités ; ce rapport n'attribue ni classement ni score.

Périmètre : Ops local-full déjà livré, profils existants, registre, réseau, images, reprise et auxiliaires correspondants. Aucun nouvel hébergeur cloud ou comportement métier Rust. [ADR-0609](../adr/0609-local-full-declaratif-helm-compose-just.md), [contrat de réception](../contracts/ops-local-full-declarative-iac.md). Statut ADR Proposed, distinct de l'autorisation d'implémenter.

## État de livraison

**En cours — aucune qualification du nouveau déploiement à ce stade.**

La migration doit retirer les constructeurs et branches nominales remplacés, pas seulement exporter des manifests à côté d'un second déployeur Python. Les baux, UID/ownership, non-adoption, contrôles de secrets CAS, PKI et provenance restent exigés.

## Base et mesure reproductible

- Base Git : `51996139ec31317aaef85e49ab3c9266591f3902`, `master`, checkout initial propre.
- Sous-module rustycog : `93b890ff7b821e9cb2e7d0aeb81957d098778a2f`, inchangé.
- Périmètre final constant : tous les `.py` sous `ops`, hors `ops/tests/**`, avec l'unique entrée de simulateur `ops/tests/support/oauth-idp-simulator.py` explicitement incluse. Avant : objets Git de la base ; après : fichiers présents, nouveaux modules et helpers relocatés du chart inclus.
- Comptage : `len(text.splitlines())`, sans ligne terminale vide artificielle ; fonctions = tous les nœuds AST `FunctionDef`/`AsyncFunctionDef` (méthodes et fonctions imbriquées incluses). Le premier relevé historique 7176 utilisait un autre périmètre et une autre convention ; il n'est pas comparé au bilan final.
- Relevé auteur du candidat final : **7 355 → 7 267 lignes** (−88, soit −1,2 %), **288 → 310 fonctions**, **22 → 29 fichiers**. La réduction des lignes est modeste ; les fonctions augmentent. Aucun compactage de lignes ni suppression de commentaires pour changer le compteur. La vérification indépendante par le gate parent reste à consigner.

Reproduire la mesure depuis la racine, sans outil externe hors Git/Python (sous PowerShell, passer ce programme avec une here-string à `python -`) :

```python
import ast, pathlib, subprocess
root = pathlib.Path.cwd()
base = "51996139ec31317aaef85e49ab3c9266591f3902"
def selected(p):
    return p.startswith("ops/") and p.endswith(".py") and (
        not p.startswith("ops/tests/") or p == "ops/tests/support/oauth-idp-simulator.py")
names = subprocess.check_output(["git", "ls-tree", "-r", "--name-only", base, "ops"], text=True).splitlines()
before = [subprocess.check_output(["git", "show", base + ":" + p]).decode("utf-8") for p in names if selected(p)]
after = [p.read_text(encoding="utf-8") for p in (root / "ops").rglob("*.py") if selected(p.relative_to(root).as_posix())]
for name, files in (("before", before), ("after", after)):
    functions = sum(isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef)) for text in files for n in ast.walk(ast.parse(text)))
    print(name, "files", len(files), "lines", sum(len(t.splitlines()) for t in files), "functions", functions)
```

## Outils de validation observés

- Python natif Windows 3.13.1.
- kubectl client 1.36.1, Kustomize intégré 5.8.1 ; commandes client/offline uniquement pendant l'implémentation.
- Docker Compose 5.5.1, Buildx 0.37.2 : validation `config`/`bake --print` seulement, pas de build ou création implicite.
- just 1.40.0.
- Helm absent du PATH hôte : binaire local **4.3.1+gc0d08d0**, sans installation globale ni modification du PATH. Release officielle [v4.3.1](https://github.com/helm/helm/releases/tag/v4.3.1), archive `https://get.helm.sh/helm-v4.3.1-windows-amd64.zip`, SHA256 vérifié contre le fichier officiel : `fcfd7b796dcf0f225abc40b5636a3b01f55f3de2fb1dfb18475df94e62c14112`. Outil jetable ignoré sous `.tmp/ops-iac-tools/`.

Le PATH Context Mode est distinct du PATH hôte ; les chemins effectifs des exécutables ont été vérifiés avant de conclure qu'un outil est absent.

## Docker : décision de préservation

Nettoyer Docker ne corrige pas les définitions dupliquées ; aucun nettoyage ou reset n'est nécessaire pour les rendus et tests hermétiques. Docker Desktop était déjà actif avant cette implémentation.

Inventaire initial en lecture seule : **cinq conteneurs préexistants**, dont trois nœuds full et un nœud legacy actifs, ainsi qu'une fixture PostgreSQL à l'état created. Les IDs complets sont conservés dans le ledger privé ignoré, pas dans ce livrable documentaire.

Ces ressources ne sont pas adoptées par cette mission. Aucun arrêt, suppression, redémarrage, changement de volume ou de cluster déduit du mandat. Aucune nouvelle ressource Docker à enregistrer à ce stade.

## Validation et publication

À compléter avec les résultats réellement obtenus après intégration : rendus et schémas standard, parité des profils/phases, tests négatifs, mesures finales, commit publié, référence distante et inventaire de nettoyage. Les résultats historiques OAuth ne qualifient pas le nouveau chemin.

### Raccords de consommation et gate d'infrastructure

Les deux tests OAuthConnect lisent maintenant la source TOML unique sous `charts/local-full/files/config/`, et le test Sentinel inclut le fichier public du chart plutôt que l'overlay retiré. Tests natifs Windows exécutés : `ops_config` **1 PASS**, `ops_mapping` **2 PASS**, projection Sentinel **1 PASS** (49 autres tests filtrés, pas exécutés). `cargo fmt --all -- --check` : **PASS**. Un seul Cargo, configuration24jobs et targetE inchangés ; aucune infrastructure tierce lancée.

`ops/tests/infra-static.sh` intégré : **83 sources YAML, 8 rendus Kustomize, 24 Helm lint/template, 12 scripts shell : PASS**. Les sources Helm templatisées sont exclues du parse YAML brut, mais les configurations YAML embarquées sont parsées après rendu. Identités Kubernetes présentes et uniques pour chaque profil/phase. La CI installe également just1.40.0 depuis la [release officielle](https://github.com/casey/just/releases/tag/1.40.0), archive Linux musl SHA256 `181b91d0ceebe8a57723fb648ed2ce1a44d849438ce2e658339df4f8db5f1263`, pour ne pas dépendre d'un outil préinstallé sur le runner. CI distante non encore exécutée à ce relevé.

### Revue indépendante des déclarations local-full

Verdict infra **PASS hors cluster**, comparaison à l'ancien renderer régénéré indépendamment depuis Git5199613 : aucun objet attendu manquant pour les quatre profils ; différences de secrets publics/enveloppes et sérialisation explicitement disposées. RBAC, comptes, probes, volumes, politiques réseau et rôles OpenBao sans élargissement relevé. Les neuf PodTemplates applicatifs sont identiques entre première réplique et scale-out ; les répliques finales déclarées conservent celles attendues. Sept rendus négatifs refusés. Aucun verdict sur le Python en cours d'amincissement ni sur le runtime.

Les fichiers sources `files/private/**` sont volontairement versionnés : placeholders et fixtures de développement déjà publiquement connues, sans octet d'opérateur. Leur nom indique la destination d'hydratation, pas un dossier de secrets réels à publier. Le renderer produit leurs enveloppes sans données. Le code natif d'échec des recettes est normalisé à1 par PowerShell, mais reste non nul et arrête le graphe ; cette observation ne bloque pas le contrat.

Revue des oracles : provenance des fixtures recoupée contre Git, parité structurelle et scénario d'ajout déclaratif jugés probants dans leurs limites. Compléments demandés avant le gate final : négatif sur le CIDR ingress (et pas uniquement API), assertions du montage de configuration IAM/Manifesto en lecture seule et rejet d'un scope migrateur partagé entre deux workloads. Le test d'ordre du graphe sera nommé sans prétendre couvrir lui-même le court-circuit, déjà exercé par le vrai moteur just dans la suite voisine. Aucun résultat de ces compléments n'est anticipé ici.

### Lot CNI et images — intégré, réception globale en attente

Seize fichiers intégrés depuis l'arbre isolé ; contenus comparés par SHA256, tous identiques à la livraison. Calico officiel 3.29.7 est conservé sous `ops/deploy/third-party/calico/`, checksum des octets vendor `b200cb3c2c7341717abe98e4acc80a13117bcd2c9c46ad46e8c209c5753f0b2f`, configuration locale en patch Kustomize. Le bootstrap n'effectue plus téléchargement/édition regex/set-env.

Les cibles et groupes d'images sont déclarés dans `ops/deploy/build/targets.json`, partagés par Bake et le lecteur Python ; les recettes et dépendances de build sont dans `docker-bake.hcl`. Les vues Python de compatibilité ne constituent pas un second catalogue maintenu. Le builder appelle Bake et conserve ses contrôles d'instantané, labels, reçus et plateforme.

Preuves du sous-lot auteur : **48 tests réussis, 1 ignoré** (preuve Cargo cache opt-in), rendu Kustomize et `bake --print` réels hors daemon ; aucun build Linux ou runtime. Sous-périmètre de cinq fichiers Python : 652 → 650 lignes, 26 → 27 fonctions. Cette baisse faible n'est ni une mesure globale ni la preuve suffisante du mandat ; la suppression des définitions et la réception complète restent à vérifier.

Recoupement parent indépendant : les onze entrées du catalogue déclaratif (images, Dockerfiles, outputs, inputs, features) et le groupe local-full sont égaux à l'ancien catalogue lu depuis l'objet Git `5199613`, après normalisation de l'absence historique de liste features en liste vide. Téléchargement HTTPS distinct du manifest officiel Calico3.29.7 : 325 176 octets, checksum identique au vendor. Aucune signature upstream revendiquée.

La revue indépendante a ensuite révélé deux défauts non couverts par ce premier vert : la dépendance artifacts sélectionnée implicitement est `cacheonly` dans le plan Bake exact alors que le wrapper exige son image Docker, et les groupes directs workspace/openbao ne changent pas leur sélection d'exécutables. Ces résultats invalident une réception fonctionnelle du builder en l'état ; correction demandée au même auteur, puis vérification du plan réellement invoqué.

Suivi : les corrections ont été intégrées et relues. Les cibles sont canoniques, les groupes partagent ces identités ; overrides nominaux conservés, sélections recouvrantes sans export concurrent, artifacts explicitement exporté. Le coût du défaut Bake direct (onze images, sans monolithe) est documenté ; le wrapper garde sa sélection exacte attestée. C-1 à C-4 fermés par revue client-only. Un cas Windows supplémentaire a ensuite été corrigé : filtrage des noms réservés d'environnement sans distinction de casse ; test du passage réel wrapper → Bake. Aucun build ou export Docker réel prétendu.

### Registre — correction de compatibilité en cours

Le candidat Compose a passé 25 tests registre et 4 refresh ainsi que les validations client Compose, mais bloquait la publication d'un registre historique. Ce candidat n'est pas reçu en l'état. Le contrat v1.1 précise qu'un helper Skopeo neuf déclaré Compose peut assurer le transport, sans adopter/recréer le registre ni son réseau/stockage ; son reçu et ses montages privés sont distincts. Inputs en lecture seule, sorties/tmp séparés, identités et stop du seul helper possédé restent obligatoires. Aucun nouveau transport HTTP Python n'est ajouté. La première version augmentait aussi le Python du module ; la simplification et la mesure globale restent exigées.

Suivi : publication historique rétablie avec helper déclaré et reçus distincts ; neuf fichiers intégrés, contenus comparés égauxSHA. La revue sécurité n'a démontré aucune vulnérabilité, mais a reproduit un refus ACL réel sur Windows/Python3.13 avant helper. Le staging a été corrigé sans élargir les SID autorisés. **41 tests registre réussis** chez l'auteur ; disposition indépendante : deux nouveaux tests vrais contrôles ACL et reproduction du vrai copy réussis, interfaces Engine/OCI seulement simulées. Aucun PASS mTLS, OCI ou persistance réels. Le module passe de497 à630lignes ; cet accroissement des preuves d'ownership/création opaque est explicite et doit être confronté à la réduction globale, pas présenté comme une baisse locale.

### PKI — inventaire et extensions OpenSSL déclaratifs

Les identités, SAN, usages, autorités et sélections de bundles sont désormais dans `ops/scripts/local-full-certificates.cnf`. Ce fichier porte les sections natives OpenSSL ; Python ne génère plus les fichiers d'extensions et ne maintient plus les listes d'applications/SAN. Les appels et validations OpenSSL ainsi que les protections des clés restent dans le wrapper existant. Aucune clé réelle existante n'a été lue ou modifiée.

Parité des APIs de catalogue recoupée contre Git5199613 : autorités, profils base/GitLab, rôles registry et deux bundles OpenBao identiques. Smoke natif Windows OpenSSL avec toutes les options : création et validation réussies, puis réutilisation réussie avec **36 fichiers clés/certificats inchangés**. Matériau de test conservé privé hors Git ; aucun hash de clé publié, aucun service ou conteneur lancé. La validation Linux reste celle de la CI à exécuter, pas une preuve déduite du smoke Windows.

Références officielles de syntaxe utilisées : https://docs.openssl.org/3.5/man5/x509v3_config/ et https://docs.openssl.org/3.5/man1/openssl-req/ .

### Exception privée examinée, pas conservée par habitude

La possibilité de remplacer le helper de snapshots privés par OpenBao Agent a été examinée à la version exacte2.6.2. La perte de `DisableRedirects` dans le clone du client d'auto-auth empêche de préserver le refus strict des redirections avec le seul drapeau natif. Le contratv1.5 maintient donc le client strict existant dans la frontière privée, sans nouveau fallback et sans dictionnaire public de service Python. Seeds CAS et exposition du token Lazaret déjà contractuelle restent distincts ; aucun token n'est à ajouter à IAM/simulateur. Une future bascule exige pin autorisé et preuves de redirections/proxy avant remplacement, pas un simple template syntaxiquement valide.
