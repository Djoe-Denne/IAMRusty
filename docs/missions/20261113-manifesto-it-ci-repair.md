# Manifesto IT — collision de port et supervision Tokio

## Échec observé

CI [43473245852](https://github.com/Djoe-Denne/IAMRusty/actions/runs/43473245852),
job132053039776, commit741d04359a332de78802ef59cd8b2d1480297787 :
`t6_post_get_components_contract_bytes_frozen` reçoit404au lieu de201.
Le log prouve que **wiremock**, et non Manifesto, reçoit le `POST` sur le port39407.

Le port HTTP aléatoire de l'ancienne instance avait été mis en cache, puis libéré.
Au test suivant, une fixture avait repris ce port. Le contrôle de disponibilité TCP
acceptait ce mauvais listener avant même la fin de l'initialisation Manifesto.

Un second défaut masquait l'erreur de démarrage : `Application::run` attendait à
nouveau le `JoinHandle` déjà consommé par `select!`, provoquant la panique Tokio
`JoinHandle polled after completion`. Le chemin sans tâches de fond avait le
même défaut. Aucun défaut d'autorisation métier ou de sérialisation T6 n'a été établi.

La CI suivante43479756786a passé les jobs Manifesto avant correction : l'incident
est intermittent. Ce PASS ne remplace pas la régression déterministe de collision.

## Correction

- **RustyCog** : nouveau port HTTP éphémère résolu après le démarrage des fixtures
  pour chaque nouvelle instance ; URL mémorisée avec la tâche pour réutiliser une
  instance vivante. Le port explicite est transmis au descripteur de service.
  Aucun changement du cache de configuration de production ni de l'API publique
  `spawn_test_server`. Les paramètres TLS/CA/certificat client restent issus de la
  configuration du service : seule la valeur du port HTTP est substituée.
- **Manifesto** : les trois branches sélectionnant une tâche marquent son résultat
  consommé. Seules les autres tâches sont annulées puis attendues au nettoyage.
- **Lanceur IT** : ajout du couple exact `manifesto / apparatus_p1_t6_http` dans
  l'allowlist existante, avec sa régression de sélection. Même harness, mêmes
  fixtures réelles et même suivi des ressources ; aucune nouvelle orchestration.

Les assertions HTTP et les règles d'authentification T6 restent inchangées.
Pas de faux certificat, canal permissif, délai arbitraire ou retry du `POST` métier.

## Publication SDK

RustyCog **1feb766fe270744849e8674905d6c6874b60f76b**, publié et référence
`origin/main` vérifiée avant mise à jour du sous-module AIForAll.

## Validations

Windows natif,24jobs configurés, un Cargo à la fois, target surE:

```text
cargo test --locked --manifest-path ../rustycog/Cargo.toml --features full,testing --lib testing::common::test_server::tests
cargo clippy --locked --manifest-path ../rustycog/Cargo.toml --features full,testing --lib --message-format=json
powershell.exe -NoProfile -File ops/tests/test-it-native-selection.ps1
```

- **2/2régressions SDK passent**, dont ancien port désormais occupé par une fixture
  et maintien d'un port fixe occupé (pas de réaffectation silencieuse).
- Clippy SDK réussit ; aucun diagnostic attaché aux deux fichiers modifiés.
- Régression PowerShell du sélecteur IT réussie, sans runtime.
- **4/4 tests de supervision Manifesto passent**, quatre autres tests filtrés.
- Clippy ciblé setup `--lib --tests`, pedantic/nursery, réussit. Les diagnostics
  existants sur le constructeur de l'application et `readiness_probe` restent
  hors périmètre ; aucun diagnostic sur le nouveau helper ou les nouvelles unités.
- **4/4 cas T6 passent**, aucun ignoré ni filtré (trois cas HTTP live et un contrôle
  statique préexistant). Le cas ayant échoué en CI conserve son attente201et passe.
- Formatage Rust ciblé et `git diff --check` réussis avant commit.
- Aucun PASS global ni résultat de CI corrective n'est encore revendiqué.

```text
cargo test --locked -p manifesto-setup --lib app::runtime_supervision_tests
cargo clippy --locked -p manifesto-setup --lib --tests --message-format=json -- -W clippy::pedantic -W clippy::nursery
powershell.exe -NoProfile -File ops/tests/it-native.ps1 -Suite manifesto -TestBinary apparatus_p1_t6_http
```

Les logs locaux sont sous `E:/cargo-target/AIForAll/manifesto-ci-repair-20261113/`.
La première compilation a échoué faute d'espace disque (`os error112`) avant
exécution : erreur d'environnement, pas test rouge. Après autorisation, une partie
du seul cache `debug/incremental` a été supprimée. À84,5Gio libres, la tentative
d'arrêter le nettoyage n'a pas satisfait son filtre de commande : il a continué,
puis s'est terminé en signalant un répertoire non vide, à205,32Gio libres.
Il ne faut donc pas prétendre que nettoyage et reprise Cargo furent exclusifs.
Les compilations et tests ont néanmoins réussi ; aucun nouveau nettoyage n'est
lancé. Sources, logs et preuves préservés ; le cache résiduel est régénérable.

## Ressources et limites

Avant IT : quatre conteneurs préexistants (trois nœuds Kind full-profile en cours,
un SMTP arrêté), non possédés par ce lot et à préserver. Aucun nouveau Kind,
Docker Desktop ou WSL n'a été démarré. Les IT utilisent les fixtures testcontainers
existantes ; le lanceur enregistre les IDs et vérifie leur nettoyage.

Campagne `it-c907197730884d5ab6b81b01cfe86550` : **2 créés / 2 retirés,
0 possédé en cours / 0 nouveau non possédé**, aucune erreur de nettoyage.
Inventaires avant/après :4conteneurs, mêmes IDs et états préexistants.
Preuves : `.tmp/itfixtures-it-c907197730884d5ab6b81b01cfe86550/` et
registre append-only `.tmp/docker-resources-20261113.log`.
Docker Desktop/WSL restent actifs pour les workloads étrangers ; aucun arrêt global.

Le correctif supprime la réutilisation déterministe d'un port éphémère périmé ;
il ne transforme pas l'allocation par bind/libération en réservation atomique
jusqu'au bind du service. Une concurrence externe sur cet intervalle reste une
limite du harness, non une preuve d'identité par TCP. Aucun nouveau contrat public
de listener prélié n'est introduit dans cette réparation bornée.

Pas de worktree, branche temporaire ou équipe créé pour ce lot. Les modifications
étrangères du wiki AIForAll et le fichier non suivi `nul` du SDK sont conservés.
Ledger : `.cursor/handoffs/20261113-manifesto-it-ci-repair.md` ; QA approuvée :
`.cursor/qa-plans/20261113-manifesto-it-ci-repair.md`.
