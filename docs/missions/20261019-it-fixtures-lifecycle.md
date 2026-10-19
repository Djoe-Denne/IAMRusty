# Ordre de mission — Fiabiliser le cycle de vie des fixtures testcontainers

**PROPOSITION — préparation seulement.** Cette mission n'est pas lancée : préparer
la policy avec `/ensemble-mission config`, présenter, puis attendre le
`/ensemble-mission start` utilisateur avant agents/classification/implémentation.
Aucun runtime ni nettoyage pour préparer cet ordre.

## 1. Demande utilisateur

Corriger le problème des conteneurs dans les tests d'intégration, constaté et
mesuré le 2026-10-19 lors de la mission
`docs/missions/20261018-ops-kind-runtime-validation-separation.md` :

1. **Churn massif** : un `cargo test -p <pkg> --tests` crée des dizaines de
   containers simultanés (pic mesuré **35** pour un seul binaire ;
   **97 running** accumulés après une campagne de 5 packages — 45 postgres,
   39 openfga, 9 sqs, 3 singletons nommés) ; RAM hôte au pic **3,5–4 Go
   libres / 32 Go** ;
2. **Flakes de setup prouvés** (2 événements, même signature) :
   `Failed to create test OpenFGA: opaque fixture start failure; identity
   UNKNOWN` — corrélés à la charge du daemon (~28 paires simultanées) ;
   discriminants : reruns ciblés verts (12/12 en `RUST_TEST_THREADS=2` ;
   2/2 en 7s). Zéro assertion en échec ;
3. **Pas de stop automatique en fin de campagne** : les fixtures (singletons
   nommés `lazaret_test-*`/`telegraph_test-smtp` + pairs par binaire) restent
   running après les runs ; 93 ont dû être arrêtées **manuellement** par le
   lead (IDs complets depuis les ledgers `fixture-*.jsonl` — piège : `docker
   ps` tronque les IDs, exiger `--no-trunc`).

## 2. Ancrage et lectures

- Dépôt : `C:/Users/djden/source/repos/AIForAll`. Revalider le HEAD au start.
- Mémoire canonique : `.serena/memories/architecture/it-testcontainers-resource-churn.md`
  (ratifiée utilisateur, commité `c72163e`) ; wiki :
  `obsidian/AI FOR ALL/projects/aiforall/concepts/testcontainers-resource-churn.md`.
- Preuves brutes : `.tmp/itfinal2-20261019115845/` (ledgers fixture, inventaires
  avant/après), registre append-only `.tmp/docker-resources-20261019.log`
  (373 lignes).
- `.agents/skills/creating-testcontainer-fixtures/SKILL.md` (design des
  fixtures, singletons `OnceLock<Arc<Mutex<...>>>`, `port = 0` +
  `cleanup_existing_*_container`) ; `.agents/skills/local-runtime-lifecycle/SKILL.md` ;
  ADR 0200–0202 (IT réelle, herméticité) ; décision 2026-10-07 (IT natives
  Windows, Docker tiers uniquement, un seul Cargo à la fois).
- Le lanceur IT livré par la mission ops : `ops/tests/it-native.ps1` (délégation
  pure, target E: restauré à l'invocation) — point d'insertion naturel du stop
  de fin de campagne.

## 3. Contraintes impératives

- **Herméticité non négociable** : état neuf par test, pas de pollution croisée.
  L'objectif est le **coût** (ressources, durée, fiabilité), pas l'isolation.
- **Framework crate** : tout changement de `rustycog-testing` vit dans le
  submodule `rustycog` (workflow `rustycog-submodule` : publier côté
  `Djoe-Denne/rustycog` d'abord, bump gitlink ensuite, CI vérifie le SHA).
  Ne jamais éditer le checkout du submodule comme s'il était le fork.
- **Comportements actuels à ne pas casser** : `cleanup_existing_*` au preflight,
  registre à la création, fail-loud des fakes, ledgers de fixtures.
- Un seul `cargo` à la fois ; target E: ; aucune suite IT sous Kind.

## 4. Résultat attendu

1. Une campagne IT complète (`it-native` 5 packages ou équivalent) laisse
   **zéro container running** à sa fin (stop possédé par IDs tracés) ;
2. Le pic de containers simultanés est **borné** (cap de parallélisme
   documenté et reproductible) — les flakes de setup par pression daemon
   deviennent rares ou absents ;
3. Les containers `Exited(0)` possédés sont retirés en fin de campagne, pas
   seulement au preflight suivant ;
4. Les ledgers de fixtures et le registre Docker restent append-only et
   exploitables ; aucun rm/prune par nom ;
5. Les suites restent vertes aux mêmes oracles (aucune assertion affaiblie).

## 5. Lots proposés (scores à valider par PM, grille déterministe)

- **F1 — Stop de campagne dans le lanceur** (implementer, ~58/55) :
  `it-native.ps1` collecte les IDs créés (runtime mode local), stop gracieux
  en fin de batch (et sur échec), retrait des `Exited(0)` possédés, rapport
  d'inventaire ; registre tenu à la création. Zéro changement framework.
- **F2 — Cycle de vie des fixtures dans rustycog-testing** (hard-implementer,
  ~68/60 ; submodule) : la herméticité conservée, choisir et implémenter
  l'arrêt automatique des singletons à l'exit du process (guard dédié plutôt
  que Drop de static) + mutualisation par-binaire là où l'isolation le permet
  ; bump du pin après publication rustycog.
- **F3 — Cap de parallélisme par défaut** (implementer, ~52/48) : borne
  reproductible des tests (config `test-threads`/env dans le lanceur ou la
  config Cargo), documentée au README runtime ; corrélation flake re-testée.
- **F4 — Règles et wiki** (mechanical-worker, ~35/25) : mise à jour
  `.cursor/rules/docker-resource-registry.mdc` / AGENTS.md (stop de campagne
  obligatoire, piège `--no-trunc`), sync wiki.

Ordre : F1 (victoire rapide sans framework) → F2 → F3 → F4. F2 est le seul
écriteur du submodule ; F1/F3 touchent des fichiers de mission distincts
(`it-native.ps1` = ownership I-validation historique, un seul écrivain).

## 6. Gates et preuves de sortie

1. Statique d'abord : parsing PS/shell, tests hermétiques verts (S-11 18/18,
   source-guard 22/22), inventaires avant/après sans secrets ;
2. Natif Windows : un Cargo à la fois, tests touchés puis IT ciblées
   (testcontainers) — les IT ne dépendent pas de Kind ;
3. Campagne de preuve : 2 packages au moins rejoués complet avec mesure du
   pic et de l'inventaire final (zéro running possédé à la fin) ;
4. Review correctness/tests proportionnée au diff ; registre append-only à
   jour ; aucun cleanup au-delà des ressources possédées.

## 7. Politique de mission et hors périmètre

Préparer une grille native (mêmes identités catalogue vérifiées que la mission
ops : luna/glm/sol high et xhigh, grok-4.7 alternatif) — à représenter avec
`/ensemble-mission config`, pas autorisation implicite. Pas de JEV/helper
implicite.

Hors mission : prod/cloud/HA, suppression de la herméticité, migration
d'infrastructure RustyCog au-delà de `rustycog-testing`, changement de la
décision 2026-10-07 (IT natives), extinctions de runtimes partagés
(`aiforall-local` m6 non possédé), réparation du harness OpenCode. Aucun
commit/stash/reset des changements préexistants ; push uniquement sur GO.
