# Cycle local durable — up / stop / status

## Mandat validé

Suite du jalon images `0db3172`, SDK `93b890f` inchangé. L'utilisateur a validé
trois commandes simples sur les outils existants, sans nouveau framework : reprise
du même environnement, arrêt sans suppression et statut. Il a explicitement rappelé
que **la volatilité d'OpenBao est volontaire en local** ; preuve coffre/auth/métier
reportée à l'E2E, persistance cloud traitée avec le futur adaptateur.

## Livrables

- Recettes minces `just local-up/local-stop/local-status` vers `deploy-local-full.py`.
- `local_full_environment.py` : état privé stable hors Git référençant le bail
  original, moteur Docker, identité d'ancrage, registre optionnel et source GitHub
  privée readonly. Pas de valeur/hash de secret dans cet état. Premier up nécessite
  un bail ancré d'une installation explicite ; aucun cluster créé/adopté ici.
- Autorité `LocalClusterTargetAnchor` : opérations fermées start/stop par IDs,
  validation offline des ports configurés lorsque Docker masque les ports effectifs
  à l'arrêt, puis UID Kubernetes original après start. Journal attempted/completed
  avant/après chaque power op ; préservation des ressources enregistrées depuis
  l'instanciation de l'ancre. Les API ordinaires refusent toujours un CP arrêté.
- Même seed Job GitHub non-root/CAS factorisé pour installation et reprise.
  Nouvelle epoch OpenBao ⇒ seed nécessaire puis annotation CAS des seuls snapshots
  Lazaret/IAM/GitHub. Aucun apply global, DB reset, changement d'image/CA ou seed
  métier. État d'une récupération interrompue conservé pour inspection explicite.
- Échec de up : arrêt réversible des seuls nodes/registre auparavant arrêtés que
  cet appel a tenté de démarrer ; ressources déjà actives non arrêtées. Pas de force.

Usage : [guide opérateur](../../ops/deploy/README.md#cycle-quotidien--up--stop--status).

## Vérification à ce stade

- **42 tests ciblés finaux PASS, 1,048 s** : 9 nouveaux lifecycle (dont sous-cas
  d'annulation Ctrl-C), régressions ancrage,
  refresh/images et absence d'inputs historiques dans les sources nominales.
- **30 régressions Anchor/OpenBao PASS** dans un premier run où seul l'import du
  nouveau test manquait du sys.path ; correction puis module lifecycle vert.
  La PKI synthétique du test existant était incluse ; aucun certificat personnel lu.
- `just --list`, CLI `--help`, syntaxe des Python modifiés PASS. Aucun Rust modifié,
  Cargo applicatif, build Linux ou E2E lancé pour ce lot.
- Bail original des trois nodes full retrouvé par le pointeur de campagne connu :
  task `fcfd0b2884e14748a56f1e0c9d8c1602`, version2 anchored, mêmes IDs originaux,
  kubeconfig/engine/API/HTTP concordants. Vérification offline réelle PASS.
- Vérification manuelle bornée du **moteur power PASS** : reprise → no-op → stop
  → reprise, mêmes trois IDs, UID Kubernetes original, nodes Ready aux deux reprises,
  mêmes UID des quatre PVC et même backing volume du seul PVC initialement lié.
  Finally : **0/3 nodes leased running**. Preuve non secrète
  `.tmp/local-environment-power-proof-2.json`, source `local_full_target.py`
  SHA256 `638611f368eb8afe2bdba3b6b0c01278aaaffc3c5de003725297368137a79164`.
  Après cette preuve, seul le traitement d'annulation du moteur a été étendu à
  Ctrl-C (cleanup identique, succès inchangé), couvert par le test final offline.
  Aucun déploiement/seed/patch applicatif dans cette vérification ; pas de preuve
  récupération OpenBao/auth déduite d'un nœud Ready.
- Le premier passage a réussi reprise/UID/nodes Ready, puis son oracle supposait
  à tort `volumeName` présent sur chaque PVC ; il a échoué sur un PVC non lié.
  Cleanup 0 running confirmé. Oracle corrigé sans toucher à la production/au
  stockage : tous les UID sont comparés, volumes seulement pour les PVC déjà liés.
- **Limite de l'environnement ancien :** `postgres-data`, `zot-data` et
  `admission-store` sont restés **non liés** aux deux observations. Aucun effacement
  ni provisionnement compensatoire n'est lancé. Ce PASS power ne qualifie donc
  ni les données de ces trois volumes ni la disponibilité applicative complète.
- Référence durable préparée sous `$HOME/.aiforall-local-full/`, via la même
  inscription explicite, vers le bail original et le registre possédé existant.
  **`just local-status` et `just local-stop` exécutés réellement : exit0**, état
  arrêté, sans nouveau fichier de bail ni nouvelle mutation API. Le parcours
  applicatif complet `local-up` n'est pas déclaré PASS sur cette ancienne installation
  partiellement provisionnée. La réparation initiale du stockage reste distincte.
- Inventaire final : **3/3 nodes explicitement repris revenus à l'arrêt**, registre
  et helpers possédés **8/8 arrêtés**, aucun nouveau conteneur inconnu. Le node
  `aiforall-local` préexistant reste running/non visé. Volumes/images/caches/PKI et
  quatre PVC conservés, aucun prune/rm/reset. Desktop/WSL partagés conservés, RAM
  non mesurée. Aucun agent, branche ou worktree de mission créé.

## Limites explicites

La première installation reste explicite ; `up` n'est ni un installeur cloud ni
un moyen d'adopter un cluster inconnu. Bail/kubeconfig supprimés ⇒ refus, pas
reconstruction des credentials. Un ancien profil sans snapshots gérés demande une
mise à niveau explicite. Les entrypoints inchangés peuvent exécuter leurs migrations
idempotentes au restart normal ; aucun Job de migration supplémentaire n'est lancé
par up. L'état DEV OpenBao/Transit perdu au stop reste attendu, pas un incident de
persistance à corriger dans ce lot. Pas d'E2E auth/métier ni d'acceptation globale ADR.
