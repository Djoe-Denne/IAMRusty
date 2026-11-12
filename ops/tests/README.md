# Tests end-to-end

La voie locale courante est la [campagne k6 native](k6/native/README.md) : santé
HTTPS, OAuth navigateur et identité SAN Envoy. Le parent historique
[`run-windows.ps1`](k6/AUTH-WINDOWS-PARENT.md) reste un consommateur legacy
distinct ; `auth-inspect` inspecte ses scénarios sans les exécuter.

Les anciens pilotes E2E Python, shell et navigateur autonome ont été retirés
sur [décision utilisateur](../../docs/missions/20261106-k6-only-cleanup.md).
Les tests Python présents ici valident l'outillage : ils ne sont pas une autre
campagne E2E applicative. Les auxiliaires Python du parent k6 restent nécessaires.

Les trois scénarios natifs sont qualifiés localement au 12 novembre 2026 ; leur
exécution en CI reste reportée. Aucun fallback automatique vers l'ancien moteur
n'est disponible.

## Contrôles offline de la CI

`infra-static.sh` vérifie la syntaxe shell et le YAML source, rend les huit
inventaires Kustomize déclarés, puis contrôle le chart Kind minimal avec Helm.
Les répertoires `templates/` et `files/` d'un chart identifié par `Chart.yaml`
sont des entrées du renderer, pas du YAML brut ; ses métadonnées et valeurs
restent vérifiées comme YAML.

Les deux tests `FixtureRenderTests` existants rendent aussi le chart k6 natif
avec des entrées publiques synthétiques et vérifient ses refus. Ils utilisent
`AIFORALL_HELM`, sinon Helm sur le PATH, sinon le binaire Windows local connu.
Cela ne lance ni k6/Chromium, ni le parent `prepare/run/clean`, ni un runtime ;
aucun secret GitHub, kubeconfig réel ou certificat privé n'est nécessaire.
