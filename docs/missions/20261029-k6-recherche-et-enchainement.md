# k6 — recherche et trois ordres de mission

Préparation documentaire demandée le2026-10-29. Base Git observée :
`0c9d4c9925006ee5ca5130e3238e508807a2f30a`. Revalider HEAD/diff au lancement.
Aucun k6/Chromium installé, test lancé, runtime démarré ou code applicatif modifié.
OHA et performance exclus. Les trois missions sont futures : config/start natifs
à leur admission, pas héritage du GO de l'ancienne campagne authentification.

1. [Socle et smoke simple](20261029-k6-01-socle-smoke.md).
2. [Portage de la campagne authentification](20261029-k6-02-portage-authentification.md),
   après intégration du socle et acceptation de ses preuves.
3. [Propagation et reprise des droits sous Linux](20261029-k6-03-droits-linux.md),
   après intégration des deux premiers lots.

Décision structurante proposée par architecte : [ADR0504](../adr/0504-validation-fonctionnelle-linux-k6.md),
[contrat](../contracts/k6-functional-linux.md). Pas de promotion des ADR existantes.

## Documentation officielle effectivement consultée

- [Fonctionnel](https://grafana.com/docs/k6/latest/examples/functional-testing/) :
  exemples de scénarios fonctionnels, pas seulement charge. Localisé via recherche
  officielle ; ancien chemin `/testing-guides/functional-testing/` retourne404.
- [k6/browser](https://grafana.com/docs/k6/latest/javascript-api/k6-browser/) et
  [guide navigateur](https://grafana.com/docs/k6/latest/using-k6-browser/) : véritable
  moteur Chromium, contextes/cookies isolés, API inspirée de Playwright sans
  garantie de compatibilité. Chromium doit être disponible ; ne pas recopier le
  défaut `no-sandbox` des exemples Docker dans une campagne de sécurité.
- [Checks et thresholds](https://grafana.com/docs/k6/latest/using-k6/thresholds/) :
  les checks seuls ne font pas échouer l'exit status. Thresholds fonctionnels et
  preuve d'exécution de chaque cas obligatoires ; pas de seuils de débit/latence.
- [Cycle de vie](https://grafana.com/docs/k6/latest/using-k6/test-lifecycle/) : setup,
  exécution, teardown ; sorties/cleanup externes à superviser, pas garantie après kill.
- [Release stable v2.3.0](https://github.com/grafana/k6/releases/tag/v2.3.0) :
  API GitHub officielle `/repos/grafana/k6/releases/latest` consultée ; stable,
  publiée2026-09-21. Base candidate à épingler, pas installation effectuée.
  `--once` conserve la configuration navigateur d'un scénario ; les raccourcis
  CLI VU/iterations peuvent remplacer la configuration des scénarios. V1 préférera
  la déclaration explicite1VU/1itération ; pas de flags supposés d'une autre version.

Les pages `latest` sont mobiles : confronter les API utilisées à la version2.3
retenue et archiver les références pertinentes en mission1. Aucun Playwright
Node, GrafanaCloud, k6Operator, dashboard ou extensionxk6 n'est requis par principe.

## Points à ne pas perdre

Une batterie k6 n'est pas automatiquement E2E parce qu'elle enchaîne des HTTP.
La preuve indique les vrais services traversés, les mocks/simulations et les
observations métier. OAuth navigateur reste Chromium ; droits effectifs restent
services/événements/OpenFGA réels. Les contrôles privilégiés restent au parent.
Les tests Rust natifs Windows conservent leur stratégie ; les campagnes E2E et
leurs services sont Linux. Pas de chantier de compilation de toute la suite Manifesto.

La décision utilisateur de dette HSM acceptée est reprise dans le contrat : pas
bloquante pour ces missions, pas oubliée, pas équivalente à un HSM qualifié.
L'ADR0309 interdit actuellement Implemented sans HSM réel : conserver Partial,
ou proposer séparément un amendement structurel plutôt que le changer implicitement.
