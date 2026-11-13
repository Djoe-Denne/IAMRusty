# Profil CI Ubuntu — préparation, pas encore une campagne exécutable

Le mandat porte sur un job `k6-e2e` autonome, parallèle aux tests. Le
[contrat Proposed](../../../../docs/contracts/k6-three-scenario-ubuntu-ci.md)
décrit le résultat attendu. Aucun succès des tests ci-dessous ne qualifie
Chromium/Linux ou le parcours E2E.

## Éléments livrés

- `evidence.py` : réception fermée des cinq résumés existants, 34 checks,
  codes natifs explicites, deltas OAuth et causalité/restauration SAN.
- `inputs.py`, `config/*.toml.in` : secrets neufs et configurations publiques
  fermées, sans import de configuration opérateur. Ces templates sont validés
  syntaxiquement ; leur chargement par les vrais services reste à prouver.
- `config/envoy.yaml` : profil navigateur TLS, upstreams mTLS vérifiés,
  ext-authz fail-closed, stripping principal et exemptions exactes OAuth/health.
- Extension opt-in `ciEnabled: true` du chart natif, namespace
  `aiforall-local-full` : alias IAM isolé et vrai workload ext-authz. Sans cette
  option, l'inventaire natif reste inchangé à dix objets. L'option ne constitue
  pas une autorisation d'appliquer ces objets au lab existant.

## Validation hors cluster

Outils existants explicites : `AIFORALL_HELM` et `AIFORALL_K6`, sinon recherche
PATH (replis Windows existants conservés pour les tests locaux). Les outils ne
sont pas téléchargés par les tests. Les scripts CI devront vérifier les pins
avant de définir ces variables ; leur présence seule ne prouve pas la version.

```text
python -B -m unittest discover -s ops/tests/k6/tests -p "test_ci_*.py" -v
```

Les tests de rendu utilisent réellement Helm ; ceux du projecteur exécutent
réellement k6 sans requête réseau ni lancement Chromium. Un outil absent donne
un skip explicite, jamais une preuve de qualification.

## Restant avant raccord CI

Le parent Ubuntu `campaign.py`, la PKI et confiance NSS privées, le bootstrap
Kind/backings/OpenFGA, la provenance des images du checkout, les reçus IDs/UID,
les forwards, la mutation/restauration SAN et le cleanup failure-safe restent à
implémenter et tester. Le job parallèle ne doit pas annoncer un E2E vert en
exécutant seulement les modules présents ici. Qualification finale : vrai run
GitHub des cinq phases, puis vérification de ses artifacts allowlistés et du
cleanup. Aucun workflow historique réactivé, aucun secret local à transférer.
