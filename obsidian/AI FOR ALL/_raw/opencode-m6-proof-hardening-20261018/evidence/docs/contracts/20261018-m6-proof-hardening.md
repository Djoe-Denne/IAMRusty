# Contrat — M6 proof hardening

- **Statut : Proposed**, relu par le lead pour implémentation test-side ; aucune promotion d'ADR ni attestation de réalisation.
- Auteur : architecte pool-2, tâche `task_mvdvonh8_0000_1fw9wn`, livraison round 2 sur base `da09876`. Version intégrée resserrée par le lead, avec précisions de sécurité et de portée ci-dessous ; livraison détaillée conservée dans le worktree et le ledger de mission.
- Canon primaire : ADR 0200–0202 ; ce contrat est secondaire et ne modifie aucune frontière de production. Le port sert les tests du fixture ; il ne remplace aucune infrastructure de la chaîne réelle.
- Sources : `docs/missions/20261018-apparatus-m6-proof-hardening.md`, plan `.cursor/qa-plans/20261018-m6-proof-hardening.md` et son amendement normatif §5, ledger `.cursor/handoffs/20261018-m6-proof-hardening-preparation.md`.
- Aucun run m5/gold path, full, lab 0412, ancienne campagne Kind, credentials, SDK ou mesh. Pas de modification du gitlink rustycog ni de `.cargo/config.toml`.

## 1. Garde de cible — PH-2/3

La seule cible Kind autorisée est `aiforall-local`. Son contexte kubectl `kind-aiforall-local` est distinct et n'est pas une valeur autorisée de `APPARATUS_KIND_CLUSTER` : pas de mapping implicite.

Résolution pure puis garde en **première étape du vrai flux ensure**, avant Docker, résolution de binaire, probe ou toute autre commande externe :

1. Env absente : défaut `aiforall-local`.
2. Env explicitement présente : trim ; vide/whitespace seul => erreur « cible vide ».
3. Noms protégés `kind-apparatus-p4-it`, `apparatus-p4-it`, `rancher-desktop` => erreur « cible protégée ».
4. Nom hors `^[a-z0-9][a-z0-9-]{0,62}$` => erreur « cible invalide » ; cela inclut chemins, traversal, espaces internes, majuscules et points.
5. Tout autre nom que `aiforall-local` => erreur « cible non autorisée ».

**Précision lead** : une valeur d'env non Unicode est une entrée présente invalide, jamais une absence. Ne pas convertir `env::var(...).ok()` en défaut silencieux ; préserver cette distinction à la frontière (par exemple `var_os` puis conversion contrôlée).

Les catégories d'erreur sont fixes, sans écho de la valeur brute : ni secret, ni CRLF, ni texte arbitraire dans les logs. Seul un nom déjà validé peut être cité après la garde. Padding non vide nominal : `"  aiforall-local  "` est accepté et normalisé.

Oracle : appeler le corps réel porté, pas une copie de la garde ou un mock d'ensure ; chaque entrée refusée produit Err et **zéro invocation externe**, y compris Docker/version. Le défaut et le nom normalisé poursuivent le flux nominal.

## 2. Port sortant test-side et temps — PH-4/8/10/16

`ensure()` garde sa signature publique et délègue à un corps unique `ensure_with` recevant la cible (absence distincte d'entrée invalide), l'override de binaire Kind et le port. Ce même corps interprète les résultats pour l'adaptateur réel et pour les tests scriptés.

Forme minimale retenue par l'architecte :

```rust
pub struct ExecOutcome {
    pub success: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub trait FixturePort {
    fn run(&mut self, program: &str, args: &[&str], env: &[(String, String)])
        -> Result<ExecOutcome, String>;
    fn sleep(&mut self, duration: std::time::Duration);
    fn monotonic_ms(&mut self) -> u64;
}
```

Les types exacts peuvent rester privés au fixture ; pas de dépendance ou crate publique nouvelle. Le port transporte commandes et temps, sans déplacer les décisions métier du flux. Err de spawn se distingue d'un process exécuté avec code non nul. L'enregistreur conserve programme, argv, env et résultat ; les traces complètes ne sont pas des captures publiques.

Adaptateur réel : `Command`, horloge monotone `Instant`, attentes réelles. Adaptateur scripté : sorties ordonnées et horloge virtuelle avançant à chaque sleep. Conserver les délais réels existants (notamment readyz 180 s, pause 2 s) ; l'oracle de timeout termine rapidement sans boucle murale de 180 s. Le nombre d'itérations est borné et testé selon les bornes inclusives/exclusives réelles, sans imposer arbitrairement un compte nominal de commandes.

Port borné au flux ensure : Docker info, résolution Kind/kubectl, liste des clusters, export kubeconfig et kubectl. `tests/fixtures/mod.rs` reste inchangé ; Docker info passe par le port plutôt que par son helper global. Builds, ORAS/Cosign, services et Pod final restent réels dans m6. Aucun test ne remplace ensure entière par une réponse préfabriquée.

## 3. Invariants de trace et probes

- Garde avant tout ; Docker info avant les autres commandes.
- Kind : override `APPARATUS_KIND_BIN` vers un fichier => résolution directe, sans probe kind ; sinon `kind version` client-only puis `kind get clusters`.
- `kind get clusters` avant export du kubeconfig dédié ; export avant commandes serveur kubectl.
- Kubectl : **`version --client`**, jamais `version` nu ; résolution répétée selon le corps partagé actuel. Probes indépendants d'un kubeconfig utilisateur ou serveur fonctionnel.
- Toutes les invocations kubectl utilisent le kubeconfig temporaire dédié, probes inclus ; ne jamais modifier ni dépendre de `~/.kube/config` ou de l'env globale du process de test.
- Lectures kindnet/Calico avant rollouts/waits CNI, eux-mêmes avant apply SSA ; apply avant wait CRD. Ready API avant opérations serveur qui le requièrent.
- Répétitions limitées aux boucles bornées readyz, re-checks CNI existants et probes client-only ; pas de remédiation cachée.
- Aucun create/delete/install/force-conflicts/force dans la trace, aucune commande corrective après erreur. Les argv exacts sont vérifiés par famille à la frontière, pas par recherche de sous-chaînes dans une chaîne globale.

Oracles obligatoires sans Docker/cluster réel : nominal complet ; cible refusée ; cible **autorisée absente** ; kindnet présent ; Calico absent ; erreur API/RBAC distincte de not-found ; échec de chaque commande/attente ; readyz expiré via temps virtuel ; binaire absent vs exit non nul ; override Kind fichier vs absent ; kubectl client-only. Toutes les branches d'erreur s'arrêtent sans mutation corrective.

Le fallback `WINDOWS_KIND` et ses stats FS réels sont une limite nommée, à contrôler sans prétendre une herméticité inexistante. Utiliser un fichier temporaire propre au test pour l'override. Les scans source restent des backstops, jamais la preuve comportementale principale.

**PH-16 : aucun re-run réel « bogus » ni autre cible hors allow-list**, même si les messages ou l'ordre changent. Une cible hors liste teste le rejet ; seule la sortie scriptée de `get clusters` teste la branche cible autorisée absente.

## 4. SSA et préservation — PH-5/9

Apply nominal : `apply --server-side --field-manager=apparatus-it-fixtures -f <manifests>` ; jamais force/force-conflicts. Un conflit simulé => exactement un apply, erreur propagée, aucune autre commande après, aucun retry forcé ni suppression/recréation.

**Précision lead** : comparer l'état des **objets simulés** avant/après, pas la file de réponses qui est nécessairement consommée. Ceci prouve la réaction du client, pas le comportement serveur Kubernetes. Les inventaires réels lecture seule avant/après N5 corroborent la préservation des objets tiers observés ; ils ne constituent pas une garantie globale et ne prouvent pas un conflit serveur non exercé. Ne pas dégrader le cluster partagé pour provoquer ce conflit.

## 5. Canary causal et classification — PH-7

Amendement post-revue sécurité S-1 : **Endpoints n'est pas la source indépendante d'identité du Pod**. Lire le Pod réellement référencé et vérifier namespace/name/metadata.uid, correspondance de l'IP choisie avec status.podIP/podIPs, Ready et absence de suppression, avant le contrôle positif puis à chaque revalidation avant/après wget. Même Endpoints stable avec IP falsifiée, UID/Ready/IP réelle discordants => échec sans H4 ; avant le négatif, pas de wget. Les tests régressent explicitement le scénario positif PodA / EndpointsB / timeout IPB et le remplacement du Pod sans changement d'Endpoints. Ne pas utiliser une seconde relecture du même Endpoints comme substitut à cette vérification Pod API.

Fait vérifié par l'architecte : le port-forward existant m6 cible le Pod plugin, **pas** le canary. Le positif canary est donc à ajouter, pas à déclarer déjà présent.

Même orchestration canary partagée entre m6 réel et les tests négatifs scriptés ; petit seam test-side dédié si nécessaire, distinct du port ensure. Ordre fail-loud :

1. Pod canary Ready ; wait asserté avant le wget.
2. Service canary avec au moins un endpoint prêt ; endpoints absents/vides => erreur.
3. Contrôle positif HTTP avec réponse applicative attendue par une voie autorisée, par exemple port-forward canary dédié lié à `127.0.0.1`. Le forward créé est possédé, arrêté et attendu à toute sortie, y compris erreur ; ne pas arrêter un forward tiers.
4. Ensuite seulement exec/wget du plugin vers la cible canary ; résultat cible atteinte distinct du refus et des erreurs d'outillage.

Le positif via port-forward prouve la vie du canary et l'accès par le tunnel API/Pod ; **pas** kube-proxy, le dataplane Service, le DNS du plugin ou la causalité exclusive NetworkPolicy. Readiness + endpoints + positif ne dispensent donc pas de classifier le négatif.

Précision d'implémentation : positif et négatif visent exactement le **même endpoint Pod/IP/port prêt et observé** ; un forward Service choisissant potentiellement un autre Pod ne suffit pas. Une IP numérique issue de l'API et validée peut retirer l'ambiguïté DNS ; revalider la cible si elle a changé. HTTP 404 ne vaut positif que s'il est la réponse attendue explicitement vérifiée du canary httpd vide sur ce Pod. Vérifier le type de wget réellement embarqué : les codes/diagnostics GNU et BusyBox ne sont pas interchangeables. Si l'oracle utilise GNU, locale C limitée à la commande, timeout/tries bornés, marqueurs exec début/fin/code stricts ; le code réseau 4 seul ne prouve rien. Toute réponse HTTP (y compris 404) dans le négatif prouve une cible atteinte et doit faire échouer l'attestation. Toute ambiguïté sur l'endpoint, le processus exec ou le type d'outil reste un échec non probant.

**Précision lead par rapport au round 2** : `connection refused`, `can't connect`, `network is unreachable`, `no route to host`, échec DNS, wget absent, échec exec et timeout du transport kubectl sont **non probants**. Ne jamais les rebaptiser DROP prouvé. Un signal timeout wget explicitement identifié, avec cible vivante, exec valide et préconditions prouvées, peut attester l'observation bornée de blocage ; pas une certification globale de NetworkPolicy ou de tout le mesh. Si la sortie ne permet pas cette distinction, fail-loud « preuve non probante » ; pas de succès par défaut.

Négatifs test-side minimaux : readiness échouée, endpoints vides, positif échoué, cible atteinte, DNS/outillage/exec/routage non probants. Chacun bloque le succès et son marqueur ; les trois premiers empêchent même le wget. Le nominal partagé seul peut émettre le marqueur après toutes les préconditions et le classement accepté. Helpers de signal dans un module test-side neutre réutilisé par réel et oracle, sans duplication du flux m6.

## 6. Captures H1–H4 et confidentialité — PH-6

Amendement post-revue S-2 : les égalités API image/digest gardent leur force mais doivent passer par une assertion/helper commun à m6 et à ses régressions, avec catégorie FAIL fixe et sans Debug des opérandes API. Sentinelles synthétiques dans les tests : le panic doit conserver la catégorie et l'échec mais jamais refléter image/digest reçu. Aucune extension de la campagne transportCLI/credentials de production.

- H1 : image et digest `sha256:` suivi de 64 hex.
- H2 : digest et statuts observés `VALID`, `Scheduled`.
- H3 : HTTP 403/200, hops, hostname attendu.
- H4 : résultat positif canary et catégorie bornée du négatif ; marqueur de succès seulement après les étapes du §5.

Marqueurs à liste fermée, valeurs observées réelles et assertions associées ; `--nocapture` seul n'invente aucune valeur. Métadonnées libtest/Cargo, comptages, durées et diagnostics sûrs restent permis. Conserver les échecs/FAILED et leurs catégories, sans les filtrer en succès. Pas d'argv complets, env, kubeconfig, config privée, clés, headers/auth, tokens ou mots de passe dans les archives ; pas de dump brut de wget/Command/trace port.

Les erreurs et panics elles-mêmes doivent respecter ce protocole ; l'obligation de conserver un FAIL n'autorise pas l'archivage d'un secret. Si une sortie fuit, ne pas la publier : garder une preuve d'échec assainie avec limite explicite, interrompre l'archivage non sûr et corriger la source. Archivage avant livraison, manifeste/empreintes et résultats libtest ; contrôle sécurité indépendant de présence H1–H4 et absence de secrets.

## 7. Ownership et validation

- N2 : staging m5, script staging PowerShell, YAML fixture de référence, trois sites ops docs et nouveau test de cohérence des trois producteurs.
- N3 : fixture Kind, chain.rs, m6 réel, nouveau module neutre et nouvelles cibles d'oracle ; aucun test ajouté dans la cible m6 réelle, qui reste exactement un test.
- `tests/fixtures/mod.rs` hors scope ; production hors scope. Un bug prod révélé reste rouge et requiert un lot approuvé séparé.
- L'architecte a vérifié l'auto-détection des cibles et les dev-deps non optionnels ; zéro édition Cargo attendue. Les oracles ne doivent pas tirer chain.rs et ses modules gated. Si une feature/manifeste s'avère nécessaire : STOP et arbitrage lead ; **N3 seul** propriétaire séquentiel du manifeste, N2 ne l'édite pas. Pas de code dupliqué pour éviter une dépendance.
- Worktrees isolés disjoints ; un seul Cargo Windows à la fois, slot tenu par lead. `CARGO_TARGET_DIR=E:\cargo-target\AIForAll` env-only et chemin effectif prouvé. Pas de modification config Cargo ou gitlink.
- Compilation/unités d'abord, IT finales ensuite, E2E m6 après ces gates et accord runtime explicite pour cette mission. Aucun run bonus historique ; registres contemporains et cleanup des seules ressources possédées.
- Pas de contournement du hook de commit : livraison documentaire worktree bloquée os-206 intégrable dans le chemin canonique avec commit normal ; aucune réparation globale de hook/harness implicite.

Exception utilisateur explicite (2026-10-18) : **un unique commit de livraison N3 avec `--no-verify`** est autorisé, après les vérifications ciblées, pour les huit fichiers test/fixture seulement et uniquement à cause de l'erreur Windows os206 du hook dans le chemin long. Aucun hook/config global modifié. Cette exception ne rend pas verte la dette Clippy m6 différée. Le lead vérifie empreintes/diff, intègre la livraison et fait le commit canonique avec le hook normal ; aucun autre contournement n'est autorisé.

## 8. Portes et portée

Relecture lead autorisant les lots test-side seulement après commit de cette version ; politique de modèle approuvée séparément, sans override. Pas d'ADR forward nécessaire pour ce seam local et réversible ; si une frontière durable de production apparaît, STOP et décision architecte/utilisateur.

Les hypothèses N0 d'entrées `[[test]]` obligatoires et de buffer re-run bogus sont caduques. Les accepts initiaux PH-9/10/16 ne dispensent pas des oracles de ce contrat. Tous les autres résidus restent nommés, avec propriétaire, impact, raison et condition de réouverture.
