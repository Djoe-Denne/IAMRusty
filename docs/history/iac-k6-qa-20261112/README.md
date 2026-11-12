# Discussion IaC, k6 et QA — distillation du 9 au 12 novembre 2026

Source : session OpenCode `ses_e3fce1a95ffefKfnyjsMfVxmEn` et ses 22 enfants
directs accessibles, jusqu'à la demande d'ingestion du **12 novembre à
16:33:38.370Z**. L'inventaire récursif ne trouve pas de descendant supplémentaire.
Voir [coverage.json](coverage.json). Ceci est une distillation, pas un export brut.

## Décisions de l'utilisateur et évolution du lot

Le problème initial n'est pas le langage Python en soi : ajouter un service ne
doit pas demander de modifier longuement un déployeur maison. Les définitions
doivent aller aux formats standard (Helm, Kustomize, Compose, Bake, OpenSSL), les
wrappers ne gardant que les contrôles/protocoles indispensables. La première
migration 7 355→7 267 lignes est un jalon modeste, pas la preuve du nettoyage final.
La direction est ensuite recentrée vers un lanceur Kind natif à entrées privées
complètes, sans nouveau provider cloud ni automatisation d'epoch présumée.

Le cycle opérateur du 10 novembre a nécessité des interventions et corrections,
puis a retrouvé 13/13 déploiements actifs disponibles et health HTTPS 200 après
reprise des mêmes IDs. Il n'a prouvé ni arrêt propre des processus des nœuds
(Exited137 observés), ni restauration des messages SQS volatils, ni tous les
parcours métier. La reprise native est distincte de la qualification k6.

Pour k6, **le sens des scénarios prime sur leur code**. Le prérequis autonome
`about:blank` est retiré : il teste l'outillage, pas le produit. Les trois
scénarios utiles sont conservés : santé, OAuth navigateur et identité SAN Envoy.
Le parcours positif repose sur une slice et une DB dédiées, pas sur un ménage
SQL après écriture nominale. La tranche IAM stateless à 12 checks reste distincte.

Le 12 novembre, l'utilisateur autorise la CA publique durable dans son seul
profil Windows et reporte la CI. Le parent sépare l'installation de confiance
des essais temporisés, puis qualifie les trois scénarios. Le nettoyage Python
est volontairement conservateur, suivi du skill k6 fixture-first et de la
révision du QA planner : preuve suffisante au coût justifié, sans dilution des
obligations de sécurité.

## Apports des sous-agents — sans fusion de leurs verdicts

- **Architecte** : contrats Ops puis parité k6, proposition de DB éphémère avec
  OID, clés/JWKS compatibles, suppression Kubernetes conditionnée par UID,
  limite non atomique du contrôle OID puis DROP. Son dernier texte parle encore
  de confiance CA temporaire : la décision utilisateur ultérieure sur la CA
  permanente prévaut. Contrat **Proposed**, pas promotion implicite.
- **QA planner initial** : plan de réception/parité et clarifications de gaps,
  sans exécution. Ce travail antérieur n'est pas une évaluation du nouveau profil.
- **Auteurs workloads/builds/registre** : migration des déclarations et corrections
  ciblées ; les chiffres de tests de leurs sous-lots ne constituent pas une
  qualification globale. Le parent intègre et publie, pas les derniers messages seuls.
- **Revues Bake/Calico et tests** : défauts de sélection/export artifacts, overrides
  et scopes migrateurs identifiés puis dispositions bornées. `bake --print` ou
  `helm template` prouvent le plan/rendu, pas l'image ou l'installation réelles.
- **Sécurité registre/PKI/Redis/Bao** : conservation des vérificateurs TLS et
  garde-fous privés. Le défaut ACL de staging Windows est reproduit puis corrigé
  sans élargir les SID. Les PASS statiques ne prouvent pas un handshake ou une API
  fournisseur réels. Certaines reprises de revue n'ont pas de nouveau verdict final.
- **Emergency-thinker** : cause ciblée du second `Set-Acl` PowerShell 5.1 ;
  proposition de persister la DACL via l'API .NET sans toucher SACL/owner ni élever
  les privilèges. Preuves natives synthétiques, pas campagne runtime.
- **Envoy debugger** : crash localisé à la validation RE2 d'une route avant bind,
  pas à TLS/DNS ; correction bornée par chemins exacts retenue par le parent.
- **Kubernetes/Docker** : publication active perdue malgré PortBindings conservés,
  puis réservation Windows bloquant le port API. Un export kubeconfig ne répare
  pas ce hop. Le redémarrage WinNAT/control-plane a exigé un accord spécifique ;
  cet historique n'autorise pas de nouvelle mutation.
- **Infra-verifier** : contrôles statiques séparés des sondes réelles ; reprise
  du 11 novembre confirmée avec trois nœuds Ready et 13 actifs disponibles,
  connecteurs nominaux optionnels à zéro conservés. Ni contenu SQS ni intégrité
  exhaustive des données prouvés. Cette disponibilité n'est pas le PASS k6 du 12.

## Preuves finales et dispositions

- **54a19ba** : run k6 `e3337dff24eb470881233fdfad6f6022`, 34 checks réussis,
  SAN303→503→303, OAuth26, IAM+7/ext-authz+4/fournisseur+3/PKCE+3,
  users0→1/outbox0→0 ; cleanup18/18, DB supprimée, enfants/forwards terminés.
- **fc5e29b** : seule constante `REDIRECTS` retirée. Des scripts hors Git utilisent
  encore le groupe lifecycle ; callbacks et appels textuels restent vivants.
  Régressions : 195 k6 PASS/8 POSIX skips +9 IdP PASS ; **pas de nouvel E2E**.
- **f2d3ebf** : skill `aiforall-k6` ajouté et skill de preuves raccordé ; pas de
  skill k6 antérieur retrouvé dans les emplacements examinés.
- **24d2a52** : planner et consommateurs recentrés ; modèles/permissions inchangés,
  YAML/diff et contrôles de contenu validés, **pas de benchmark comportemental**.

Le lab reste en keepalive explicite à la dernière observation de campagne, pas
par une nouvelle sonde de cette ingestion. CI, retrait du legacy et nouveaux
parcours password/refresh ne sont pas livrés. Les statuts ADR/contrats ne sont
pas réécrits. Les contenus privés, outils et raisonnement ne sont pas publiés.

## Sources durables de contrôle

- `docs/missions/20261109-ops-declarative-migration.md` (jalon historique).
- `docs/guides/ops-declarative-boundaries.md` ; `ops/deploy/kind-minimal.md`.
- `docs/missions/20261110-kind-local-operator-cycle.md`.
- `docs/missions/20261112-k6-three-native-results.md` et son evidence JSON.
- `docs/missions/20261112-python-dead-code-cleanup.md`.
- `.agents/skills/aiforall-k6/` ; `.opencode/agents/qa-planner.md` et consommateurs.

Les projections visibles filtrées restent dans un dossier privé propriétaire
hors dépôt. La couverture concerne les messages `user` et `assistant` accessibles,
pas les sessions supprimées/non reliées ni tous les événements natifs. Les blocs
de code et queries d'URL sont omis ; les valeurs privées ne sont pas des notes wiki.
