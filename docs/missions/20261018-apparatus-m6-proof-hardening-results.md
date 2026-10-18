# Apparatus m6 — résultats du 18 octobre 2026

**Campagne terminée : tests et preuves validés ; O-LIVE-1 accepté explicitement par l'utilisateur.**

Mission : [ordre de mission](20261018-apparatus-m6-proof-hardening.md).
Code exécuté : `fd9c4125ffd47c722d6df6080bd4eebfb4c93753`.
Pas de changement production, Cargo/lockfile/configuration ou pin rustycog
(`41a86dba0a24844f900783e19ef2405319d58b3f`).

## Résultats réellement exécutés

- Cohérence staging : six tests verts,108 mutations internes ; trois producteurs corrigés.
- Oracles fixture/preuve : **10 +11 tests verts**, zéro skip ; Clippy strict des deux cibles vert.
- IT finales Windows natives : **t6 9/9**, **m1 1/1**, zéro failed/ignored/filtered.
- E2E réel `kind-aiforall-local` : **m6 1/1**,51.07s de test, zéro failed/ignored/filtered.
- Binaire m6 exécuté depuis `E:/cargo-target/AIForAll/debug/deps/` ; un Cargo à la fois.
- Aucun run m5 Kind, gold path PowerShell, quinze anciennes suites Kind ou transport historique supplémentaire revendiqué.

La chaîne réelle a produit quatre marqueurs uniques : catalogue/enveloppe/CRI,
admission VALID et Scheduled, invoke403/200 avec hops0/1 et hostname lié au catalogue,
canary HTTP404 positif puis timeout GNU wget borné. Le Pod canary a été corrélé
indépendamment à l'API Pod avant le positif et avant/après le négatif.
Le pass atteste ces assertions, pas une causalité exclusive NetworkPolicy, le
dataplane Service/DNS, ni tout le mesh. Body404/stderr GNU bruts ne sont pas archivés.

## Preuves et confidentialité

Racine locale privée : `.tmp/m6-proof-hardening-20261018/`.

- Correction : `n3-r1/`,13 fichiers avec provenance/logs/patch ; source et destination des six fichiers identiques par SHA256.
- Runs : `runtime/t6-20261018T170538Z/`, `runtime/m1-20261018T170851Z/`, `runtime/m6-20261018T171224Z/`.
- Capture fermée : `runtime/H1-H4.live.log`, SHA256 `2b8efea746ae280be137ef690d973709875d5296f7a3b1f0c4520605ca4d21f7`.
- Manifeste allow-list : `runtime/LIVE-MANIFEST.json`,14 entrées, SHA256 `2d2a0d0ee298cfc44c0bb09392d11045e76231e9ee6c660c7d8d92c59d7cfbbb` ;14/14 tailles et empreintes revérifiées par reviewer.

H1 partage sa ligne avec le préfixe libtest : extraction au marqueur puis validation
intégrale du schéma et de l'unicité, sans réécrire l'original. Confidentialité du
corpus désigné validée par revue ciblée ; aucune garantie universelle sur `.tmp`.
**Jamais d'archive récursive du dossier runtime.** Le kubeconfig temporaire privé,
exclu du manifeste, a été supprimé après libération du bail d'audit ; le kubeconfig
partagé n'a pas été modifié. Aucun credential publié ni fuite publique constatée.

## Revues finales

- Correctness : **PASS observable**, attentes BusyBox404/GNU timeout levées par run réel.
- Tests : **PASS**, counts9/1/1, liens de preuve et14/14 empreintes vérifiés.
- Sécurité : **PASS preuves/confidentialité ciblée**, S1/S2 du diff fermés ; réserve O-LIVE-1 ci-dessous.
- Infra : **PASS technique**, Node/CNI prêts, canary/plugin opérationnels, fixtures Docker exited0. Warnings control-plane transitoires dans fenêtre du test, résolus à l'audit ; aucune panne utilisateur démontrée.

Briefings détaillés et ledger local : `.cursor/review-briefings/20261018-m6-proof-hardening-{correctness,tests,security,infra}.md`
et `.cursor/handoffs/20261018-m6-proof-hardening-preparation.md`.

## O-LIVE-1 — MEDIUM, comportement accepté explicitement

Le harness a supprimé/recréé **deux objets Kubernetes préexistants** :
Pod `plugin-98ba747fc572de29d76dfd08f9253778` (UID52881667…→3cc98b5a…)
et AdmissionRecord `sha256-98ba747fc572de29d76dfd08f92537782bf0353b652adcace10200020ee560cf`
(UID7c9312fe…→0cfef8de…). Les noms dérivés du digest ne prouvent ni propriété
ni autorisation. Les autres UID inventoriés sont stables ; leurs specs/labels
ne sont pas réputés intégralement inchangés pour autant.

Le GO ciblé Docker n'établit pas explicitement le droit de supprimer ces objets
Kubernetes non possédés. Owner : lead. Impact : état antérieur remplacé et
interruption possible d'un usage partagé, même local ; aucune panne réelle démontrée.
Le vert fonctionnel n'est donc pas un quitus opérationnel global.

Après explication des effets et risques en local partagé comme en environnement
distant, l'utilisateur a décidé : **« Alors on accepte ce comportement. »**
Décision du 18 octobre 2026 : comportement du harness de tests local accepté,
owner lead ; clôture de cette réserve par acceptation explicite, sans prétendre
que le GO initial couvrait déjà les remplacements. Les deux suppressions/recréations
restent des faits documentés, pas une préservation intégrale de l'état antérieur.

Cette décision n'autorise ni usage en production, ni nouveau run, restauration ou
suppression supplémentaire. Réouverture : environnement distant/production,
changement de périmètre, données à préserver ou usage concurrent incompatible ;
obtenir alors un accord ciblé ou proposer un garde-fou dans un lot séparé.

## Ressources et dettes conservées

Registre append-only : `.tmp/docker-resources-20261018.log`,119 événements mission.
Deux dernières fixtures possédées arrêtées par ID : Zot `896378075e7c86d71f62bc67454ee9757ec8f8c18c17668ee9b0e0da4eb81dbd`,
Transit `75944ce8c579dbeeb98516cdd53430d18384301ac896d7edb26087e62d29a938`.
Helpers terminés ; six nouveaux volumes et deux images conservés `ask`, sans prune.
Nœud Kind/Desktop/WSL préexistants non possédés conservés ; aucune RAM globale libérée revendiquée.

- Strict Clippy m6 production : **ROUGE,27 diagnostics différés explicitement par l'utilisateur**, pas un gate strict passé.
- T2/T4 et G-A/G-B : acceptations bornées dans QA/briefings (owner lead, impact/raison/réouverture) ; aucun effacement silencieux.
- Résidus credentials hérités et sept tests cfg(unix) restent hors périmètre.
- O-LIVE-1 accepté : aucun arbitrage de clôture restant ; aucune nouvelle campagne nécessaire pour valider les preuves acquises. Les dettes explicitement différées ci-dessus ne sont pas réputées corrigées.
