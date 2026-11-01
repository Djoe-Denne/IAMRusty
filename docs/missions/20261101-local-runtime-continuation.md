# Suite locale — stockage, consommation OCI et diagnostics

## Mandat

Suite de `e1607ae`. L'utilisateur autorise la poursuite autonome, en demandant
des questions concises seulement si nécessaire. Préserver données, certificats,
images et caches ; aucune suppression/reset/recréation implicite. OpenBao local
reste volontairement volatile ; preuve auth/métier reportée à l'E2E convenu.

## Diagnostics d'échec désormais conservés

`private_diagnostics.py` est activé seulement par les entrées opérateur deploy,
lifecycle, bootstrap Kind, consumer d'images et registre. Échecs natifs : octets
stdout/stderr, code/timeout, étape/outil/opération dans un dossier owner-only hors
Git ; argv/stdin/environnement non copiés. Limite par flux 1 MiB tail, tailles et
troncature explicites. Message public expurgé + référence du dossier, aucun dump.
Sans échec natif/intermédiaire, rien n'est créé ; un problème de capture ne masque jamais l'erreur
d'origine. Aucun hook, daemon, purge automatique ou bibliothèque de plugins.

Preuves : cinq tests ciblés, dont un vrai enfant Python qui écrit une sentinelle
sur stderr puis sort17, garde privée Windows/POSIX réelle, absence de fuite publique,
timeout/partiel/troncature, capture inaccessible et exception arbitraire dont le
texte n'est pas archivé (type/code seuls, pas d'argv caché dans une exception).
Puis première tentative CRI
réelle refusée : le diagnostic privé a bien été conservé avant la sortie du wrapper.

## Première consommation OCI réelle — refus au préflight

Le registre possédé a été repris, sans nouveau build ni publication : l'image
`apps/ext-authz@sha256:dd015871d73913647e8515be64a32d44dc9de074f6f4f8a8894158778a9ff008`
était déjà publiée et vérifiée. Les nœuds sont ceux du bail d'origine.

**Pull CRI NOT RUN** : le préflight a refusé la configuration actuelle avant de
transférer la clé reader ou de tirer l'image. Le dump réel de containerd **2.0.3**,
config v3, possède deux `config_path` : CRI images et service transfer, tous deux
vides. Les confondre par grep ou imposer une seule ligne est un bug de vérification.

Correction : parser TOML, choisir le plugin CRI correspondant à containerd1/2,
ignorer la section transfer et valider tous les nœuds **avant** connexion réseau
ou lecture/transfert de clé. Le refus d'une CRI sans hosts-dir reste conservé.
Le YAML Kind full prépare ce chemin pour de **futures créations explicites** ; il
ne modifie/recrée pas les nœuds existants. Aucun restart/reconfiguration containerd
n'a été exécuté par cette tranche. Pas de substitution `ctr` ou `kind load` à une
preuve CRI manquante.

Sources primaires lues au tag effectif :
[hosts v2.0.3](https://raw.githubusercontent.com/containerd/containerd/v2.0.3/docs/hosts.md),
[résolution CRI v2.0.3](https://github.com/containerd/containerd/blob/v2.0.3/internal/cri/server/images/image_pull.go).
La documentation main plus récente ne prouve pas les defaults du binaire épinglé.

## Vérification et suite

**61 tests ciblés finaux PASS, 32,406 s**, incluant diagnostics, parser CRI1/2 avec
quotes TOML/section transfer, refus d'un nœud ultérieur avant réseau/clés, gardes
cible et décodage natif, lifecycle, registry/refresh, source acceptance et OpenBao
avec rendu stockage des trois profils. Syntaxe10Python et diff whitespace PASS.
Pas de nouveau build applicatif, Cargo ou test E2E.

## Stockage : cause trouvée, sources corrigées, live non migré

Le spécialiste Kubernetes RO a confirmé que les PVC Pending n'ont aucun consommateur
à cause du rendu : PostgreSQL/Zot utilisent encore emptyDir, l'operator hostPath
est refusé par PSA baseline. Redis Bound prouve que SC/provisioner fonctionnent.
Les trois `$patch: replace` n'ont pas l'effet attendu avec le renderer effectif
(Kustomize5.8.1) ; cela a été reproduit offline, sans attribution upstream inventée.

Correction : JSON6902 avec test du nom puis remplacement du **seul élément data**,
pas de la liste complète. PostgreSQL garde `init`, Zot garde `config`/`htpasswd`.
Le renderer vérifie PVC, volumes requis et cohérence des mounts ; trois profils
base/GitLab/GitHub passent et les mutations dangereuses sont rejetées.

**Le deploy normal refuse désormais une transition de volume existant avant
bootstrap/quiesce/apply.** Ni `--confirm-quiesce` ni le nombre de réplicas ne
transforment une perte de données potentielle en migration autorisée. Après une
bascule distincte et vérifiée, une spec stockage identique reste acceptée.

Les Pods sources sont restés les mêmes pendant l'investigation (UID PostgreSQL
2e4e55e7-5052-4094-8e56-35b6354591bf, Zot f147d8a4-dadd-4ae4-a5c9-e952df10c8db).
Aucun contenu de DB/registre lu ou archivé. **Aucune bascule live exécutée** : il
faut préserver puis vérifier la restauration avant le remplacement des Pods.
L'outil backup existant suppose une source PVC Bound et scale Zot à0 : il ne doit
pas être utilisé tel quel sur ces emptyDir.

L'installation existante n'a pas les initContainers `openbao-snapshot` IAM/Lazaret ;
l'operator référence encore l'ancien endpoint HTTP. Ce n'est pas une défaillance de
la reprise quotidienne, mais une mise à niveau initiale explicite encore à faire.
Ne pas utiliser un déploiement complet comme raccourci de migration stockage.

Plan borné : [préservation et activation CRI](../local/local-full-storage-cutover.md).
Audit : [source vs live](../local/audits/kubernetes/2026-11-01-local-full-storage-render.md).
Question restante : autoriser la brève indisponibilité, sauvegarde/restauration,
remplacement ciblé des Pods puis activation/restart containerd nœud par nœud.
Sans cet accord, publier les corrections indépendantes et rendre l'environnement
arrêté. La disponibilité applicative complète et OCI→CRI→refresh restent non qualifiés.

## Vérification indépendante et retour au repos

`infra-verifier`, session `ses_e67e398a5ffeKtbbD2J4h18v7g`, a examiné exactement
les sources committées en `62eeea5` (aucun delta après son examen) : **6/6 checks
source PASS, 0 FAIL**. Rendu base réel de 113 objets, trois remplacements PVC avec
auxiliaires et mounts cohérents, garde deploy avant mutations, parser/préflight
CRI exact et YAML Kind borné à la création ; revue diagnostics limitée au périmètre.
Ses quatre observations live confirment ancrage/UID, mêmes Pods sources, stockage
encore non migré et hosts-dir CRI vide. **Cela ne vaut pas PASS cutover ou plateforme.**

Précision parent sur la conclusion du reviewer : recréer Kind n'est **pas** requis
pour activer le hosts-dir. Une modification ciblée de la configuration existante
et un redémarrage progressif autorisés restent l'option du plan. Le préflight
refuse tant que ce chemin n'est pas effectivement activé ; aucune recréation n'est
autorisée ou proposée pour contourner ce refus.

Les 19 livrables source/docs/tests ont été publiés en
`62eeea5a8481a7f6c1eb03da9a274dfb2ac40f85` sur `origin/master`, référence distante
vérifiée. L'archivage de ce résultat de vérification/cleanup est un complément
documentaire, sans changement de code nécessitant de rejouer les 61 tests.

Après fin des vérifications, **`just local-stop` réel exit0** : les 3 nœuds du bail
original et le registre/5 helpers enregistrés sont arrêtés, IDs contrôlés par inspect.
Deux autres conteneurs historiques de registre étaient déjà arrêtés et n'ont pas
été adoptés/manipulés. Inventaires avant/après 12 conteneurs, mêmes ID/états finaux,
aucun nouveau conteneur ni suppression ; le nœud local partagé reste running.
Les 3 nœuds et le registre ont bien été repris puis arrêtés pendant ce lot : l'égalité
des états finaux ne signifie pas « jamais touchés ». Pas de remplacement des Pods
source ni de lecture/copie des données ; certificats/images/caches non effacés.

Preuve : `.tmp/ops-continue-closeout-20261101-corrected.json`, inventaire brut
apparié `.tmp/ops-continue-closeout-20261101.json`, journal
`.tmp/docker-resources-20261101.log` et journal power du bail. Le premier appel du
comparateur classait à tort des ressources déjà présentes comme newly-owned ;
interprétation corrigée avec comparaison complète des préexistantes et contrôle
séparé des IDs explicitement repris. Aucun conflit de propriété n'a été utilisé
pour autoriser une suppression. Trois anciens registres à encodage invalide/mixte
ont été préservés, sans littéral des IDs visés ; les événements contemporains et
le bail d'origine restent les preuves de cette opération réversible.

Bail runtime libéré, aucun job/agent encore actif, aucune branche/worktree/équipe
créée pour ce lot. Les 82 branches/5 worktrees préexistants et SDK93b890f restent
intacts ; modification UI Obsidian étrangère préservée. Desktop/WSL partagés
conservés à cause du nœud local non visé ; RAM non mesurée. **Suite globale non
clôturée** en attente de l'accord de bascule, sans runtime gardé actif pour l'attente.
