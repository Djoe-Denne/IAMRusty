# Frontières Ops : déclarer un service, ne pas étendre un déployeur Python

Cadre : [ADR-0609](../adr/0609-local-full-declaratif-helm-compose-just.md) et [contrat](../contracts/ops-local-full-declarative-iac.md). Le statut d'architecture reste Proposed ; l'état réel et les preuves de la migration sont dans le [rapport de mission](../missions/20261109-ops-declarative-migration.md).

## Où placer une nouvelle définition

- **Workload, Service, compte de service, probes, volumes, ressources et règles réseau :** chart `ops/deploy/charts/local-full/`, avec les variantes dans `ops/deploy/profiles/local-full/`. Réutiliser le modèle d'une catégorie existante avant de créer un template spécifique.
- **Image, Dockerfile, binaires et features :** `ops/deploy/build/targets.json`. Bake consomme cet inventaire ; les recettes et dépendances sont dans `docker-bake.hcl`, pas dans une table Python parallèle.
- **Identités TLS, SAN, usages et bundles locaux :** `ops/scripts/local-full-certificates.cnf`. Les sections d'extensions sont lues directement par OpenSSL. Ne pas ajouter une branche Python pour le nom du nouveau service.
- **Configuration publique et droits OpenBao :** fichiers de configuration/templates du chart, avec références aux sources privées ; jamais les credentials OAuth, HMAC, tokens ou clés réelles dans les valeurs Helm.
- **Phases et dépendances d'installation :** `ops/deploy/local-full.just` et métadonnées déclarées des ressources. Un service de catégorie existante doit participer au rendu, aux attentes et à la reprise sans ajout à une liste de noms Python.
- **Registre externe et auxiliaires OCI :** `ops/deploy/registry/compose.yaml`, fichiers de transport et configuration publique associés. Ne pas déplacer le registre dans Kind pour uniformiser artificiellement la description.
- **Topologie locale/CNI :** configuration Kind existante et paquet Kustomize `ops/deploy/third-party/calico/`. Un changement de version/réseau est une modification distincte, pas une conséquence implicite de l'ajout d'un service.
- **Infrastructure d'un hébergeur :** module/adaptateur OpenTofu réellement livré. Le module cluster actuel ne provisionne pas un cloud ; ne pas inventer un provisioner shell/Python pour donner cette impression.

## Réception d'un ajout de même catégorie

1. Modifier les déclarations et leurs valeurs typées ; aucun nouveau dictionnaire `Deployment`/`Service` en Python, ni nouvelle boucle de démarrage par noms.
2. Vérifier le rendu standard du profil et de ses phases, puis la cohérence des références, labels/selectors, TLS, comptes, volumes, droits et dépendances. Un compte d'objets correct ne prouve pas cette cohérence.
3. Vérifier que les répliques transitoires protègent les migrations et que l'état final est celui déclaré. Un Pod Ready ne constitue pas une preuve métier.
4. Exercer les oracles existants de la frontière touchée et les refus utiles : inputs incohérents, ressource étrangère, UID remplacé, Job échoué, CAS divergent, mauvaise CA ou image périmée. Ne pas supprimer ces garanties pour raccourcir un script.
5. Un rendu ou un plan ne démarre aucun runtime. Toute qualification réelle reste liée à une cible autorisée, à son inventaire et à son bail ; aucun reset ou nettoyage automatique n'est une étape d'ajout de service.

## Ce qui peut rester en Python — et pourquoi

- **Appel des outils et lecture de leurs sorties :** Helm, kubectl/Kustomize, Compose, Bake, OpenSSL, Cargo pour le collecteur d'artefacts déjà livré ; codes natifs et diagnostics fiables.
- **Entrées privées et sécurité de l'hôte :** ACL, chemins canoniques, refus des liens/reparse points, validation des PKI, transfert confidentiel des octets. Ces contrôles ne sont pas des définitions de workload.
- **Autorité locale :** baux, identités Engine/cluster, UID/ACK, non-adoption et postconditions. Un label ou un nom Compose/Kubernetes ne suffit pas à prouver la propriété.
- **Protocoles métier :** seeds CAS/idempotents OpenBao/OpenFGA, reprise d'epoch interrompue, sauvegarde binaire et provenance des artefacts. Leurs enveloppes de Jobs/Pods restent déclarées ; un template ne rend pas une transaction multi-documents atomique.
- **Constats et reçus :** suivi des effets réellement acquittés, y compris après un échec. Le wrapper ne doit pas reconstruire des specs ou corriger silencieusement le rendu.

Cette liste ne justifie pas de garder du code nominal devenu sans consommateur. Toute nouvelle exception doit nommer la garantie impossible à exprimer avec les outils retenus, conserver son oracle et éviter une seconde implémentation du déploiement. Déplacer le même générateur dans du shell ou un autre langage ne satisfait pas ce cadre.

### Exception bornée : snapshots privés et OpenBao 2.6.2

L'Agent natif peut authentifier et rendre des templates, mais sa version épinglée ne conserve pas `DisableRedirects` lors du clone du client d'auto-auth. Le drapeau seul ne garantit donc pas le refus des redirections assuré par le client strict existant. Ce défaut upstream vérifié justifie de conserver ce client dans la frontière privée actuelle, pas de maintenir des définitions publiques de services en Python. Aucun proxy, nouveau moteur, bump ou patch du fournisseur n'est introduit pour contourner ce défaut.

Les définitions publiques et templates restent déclarés ; hydratation privée, vérifications et promotion restent bornées. L'exception ne signifie pas que tous les tokens sont cachés aux applications : Lazaret consomme déjà son token dans son TOML ; IAM et le simulateur ne doivent pas le recevoir. La bascule Agent future exige un pin exact autorisé et des preuves auth/lectures contre redirections et proxy ambiant avant remplacement. Voir [contrat §4.1](../contracts/ops-local-full-declarative-iac.md) et [source OpenBao 2.6.2](https://github.com/openbao/openbao/blob/v2.6.2/api/client.go).

#### Limitation connue — OPS-OPENBAO-262-REDIRECTS

- **Symptôme et cause :** dans OpenBao Agent **2.6.2**, le clone du client utilisé par l'auto-auth ne recopie pas `DisableRedirects`. `BAO_DISABLE_REDIRECTS=true` sur le client initial ne garantit donc pas le refus d'une redirection lors du login Kubernetes. Sources exactes : [clone du client](https://github.com/openbao/openbao/blob/v2.6.2/api/client.go) et [auto-auth](https://github.com/openbao/openbao/blob/v2.6.2/command/agentproxyshared/auth/auth.go).
- **HTTPS ne corrige pas ce défaut :** une URL HTTPS peut rediriger vers une autre URL HTTPS. Les certificats locaux peuvent être générés avec OpenSSL et le `.cnf` existant ; ce sujet est indépendant du respect de la destination d'authentification.
- **Portée :** limite du remplacement du client strict actuel par l'Agent à protection identique, pas impossibilité d'utiliser OpenBao, TLS ou Kind. Elle ne bloque pas les lots indépendants de nettoyage et ne justifie pas de conserver le framework de déploiement.
- **État au 2026-11-09 :** constat par lecture des sources épinglées ; reproduction native **non exécutée** et aucune version corrigée qualifiée ici. Avant une bascule Agent, vérifier un mécanisme/version compatible et le refus effectif des redirections sur l'authentification et les lectures ; ne pas déduire cette garantie du seul succès TLS. Le présent repère ne change ni le pin ni le contrat courant.

## Compatibilité et limites

Les façades documentées conservent leurs noms et flags, notamment `just deploy-local-full`, `local-up`, `local-stop`, `local-status` et les profils historiques exclusifs. `up` sain reste un réemploi, pas un redeploy. Les modes de refresh restent image-only et refusent les changements de configuration/recettes incompatibles.

Helm est utilisé comme renderer local, pas comme nouvel owner de releases existantes. Compose n'importe pas un ancien registre par son nom. Ses auxiliaires de transport ne doivent ni adopter ce registre ni devenir un second writer sur ses volumes persistants. Les bases Kustomize partagées et le comportement métier de l'operator Apparatus ne sont pas retirés par ce guide.
