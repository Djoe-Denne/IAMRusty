# ADR-0005 : Officiel et communautaire partagent le même protocole ; admission, VALID, VERIFIED et installabilité sont distincts

- Statut : Accepted
- Réalité : Partial
- Date : 2026-09-10
- Décideurs : Architecture AIForAll — ratification orchestrée du 2026-09-10
- Jalon concerné : P0 (pas d’exception dans les contrats), P4–P6 (preuves)
- SuperSède : aucune
- SuperSédée par : —

## Contexte

Le premier Apparatus sera probablement « officiel ». La tentation est un chemin `trusted` : in-process, sans gateway, sans digest, sans conformance — plus rapide, et un second protocole à maintenir. `VERIFIED` (confiance éditoriale) se confond alors avec `VALID` (preuve technique automatisée).

Le runtime ne doit consommer que des descripteurs **admis**. Les étapes Git→build→conformance→signature→OCI sont P4 ; la *politique* de confiance et la séparation des autorités doivent être dans les contrats dès P0.

## Décision

1. **Un seul protocole** backend/UI/capacités pour tous les publishers. `VERIFIED` n’ajoute pas de méthode, ne saute pas la gateway (ADR-0004) et n’autorise pas le chargement in-process (ADR-0003).
2. **Admission** : seul un résultat d’admission indépendant du publisher, du plugin, du harness et de la confiance éditoriale peut attester un digest. Enregistrer une release dans Manifesto ne l’auto-admet pas. Le déploiement exact de cette autorité relève d’`APP-01`.
3. **`VALID`** : digest + version de politique de conformance + rapport attesté par l’admission. Ce n’est ni « sûr », ni « sans exfiltration », ni « installable partout ».
4. **`VERIFIED`** : signal éditorial supplémentaire, distinct et révocable. Ce n’est ni une admission, ni un droit d’installer, ni un bypass.
5. **Installabilité** : le runtime n’installe qu’un digest admis (ADR-0002), puis l’autorisation exige encore le consentement du binding et la politique locale.
6. Aucun bypass officiel dans les contrats, le SDK ou le harness de référence. Le harness et `apparatus dev` ne produisent jamais `VALID` ou `VERIFIED`.
7. L’Apparatus de référence emprunte le **même contrat wire** que le catalogue communautaire en P0, puis le même chemin de publication et d’exécution dès que celui-ci existe.

Le détail des attestations OCI, SBOM et de l’autorité de signature relève de `APP-01` / P4.

## Conséquences

- Les DTO et le manifeste n’ont pas de champ `trusted_skip_gateway`.
- P6 : ne pas ouvrir le catalogue tiers tant que le confinement n’est pas démontré ; `VERIFIED` ne remplace pas cette preuve.
- Révocation d’une release : empêche les **nouvelles** installations et les upgrades vers ce digest. Les bindings déjà `ready` deviennent immédiatement non invocables depuis le desired state DB, sans attendre OpenFGA ni la destruction du workload.
- Le drain des appels en cours, la destruction du workload et la rétention des données restent une ADR P4/P6 ; ils ne peuvent pas rouvrir de nouvelle invocation.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Apparatus officiels in-process / UI `component` trusted | Second modèle de menace, exception permanente |
| `VERIFIED` ⇒ skip conformance | Confiance éditoriale ≠ artifact testé |
| `VALID` auto-déclaré par Manifesto, le publisher ou le harness | Catalogue métier ou exécution locale ≠ admission indépendante |
| Tags Git consommés par le contrôleur | Non reproductible (ADR-0002) |

## Non décidé ici

- `APP-02` (qui a le droit de soumettre / d’être `VERIFIED`).
- Étapes concrètes Factory et format OCI (note factory).
- Drain, destruction et rétention après révocation d’un binding déjà installé.

## Références

- Wiki : `apparatus-factory-and-distribution`, `apparatus-platform`, `apparatus-implementation-plan` (P6)
- Preuve d’implémentation (2026-09-10, P1 2026-09-12) : l’Apparatus de référence emprunte le même contrat wire `manifesto-apparatus/1` que le catalogue cible. Les DTO et le manifeste n’ont aucun champ `trusted_*`. Le harness et les tests P0 ne produisent ni statut `VALID`, ni `VERIFIED` ; aucun pipeline d’admission n’existe encore. P1 : 0 `VALID`/`VERIFIED`, 0 second UUID, 5 routes `/components`, FGA 5 types inchangés (T5 4/4 FGA 5 types ; T3+T6 0 second UUID ; T7 3/3 : 0 `VALID`/`VERIFIED` quotés, 5 routes) → `Partial` maintenu.
