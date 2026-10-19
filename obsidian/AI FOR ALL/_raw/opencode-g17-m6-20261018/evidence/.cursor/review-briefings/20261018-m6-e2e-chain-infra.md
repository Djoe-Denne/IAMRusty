# P6 — vérification infra indépendante post-run4

```yaml
reviewer: infra-verifier
member: pool-17
head_sha: e66b736
documentation_head: e7e6630
scope: kind-aiforall-local / m6
verdict: PASS
checks: 13 PASS, 1 INCONCLUSIVE
persisted_by: lead
```

## Sources et périmètre

Rapport complet reçu via `team_results` de pool-17 (tâche `task_mvdo2ptm_000x_bdtv4e`). Lectures Kubernetes avec kubeconfig exporté, contexte explicite `kind-aiforall-local`, `--request-timeout=15s`. Docker : inspect/ps/images/network/events bornés. Tous les registres des 05/06/07/10/17/18 octobre lus. Aucun Secret, exec, test, apply, mutation, arrêt, création ou suppression par le vérificateur.

Le vérificateur déclare des observations à partir de 10:15Z le 18 octobre. Son horaire final annoncé 10:37Z n'est pas corroboré (horloge lead à 10:27:22Z lors de la persistance) ; cette borne n'est pas retenue comme preuve temporelle. Les horodatages objets/événements ci-dessous sont distincts des horaires déclarés du rapport.

## Résultats

1. **Nœud PASS** : `aiforall-local-control-plane` Ready ; UID `3a0105cc-f7e3-4ae4-854e-6e7b7156c16b`, créé le 2026-10-10T12:17:49Z (antérieur au lot).
2. **CNI PASS** : Calico node/controllers Ready, CoreDNS Ready, aucun kindnet (DaemonSet ni Pod).
3. **CRD PASS** : `admissionrecords.apparatus.aiforall.dev`, NamesAccepted et Established True.
4. **Namespaces PASS** : UIDs system `fd6057ef-f9db-4f96-a8b8-c761a2fa0743`, plugins `a82cd62c-ec37-4198-8e72-36d9a2aecd06`, créés à 08:48:34Z ; absents avant le lot, labels fixture présents, labels J3 préexistants des autres namespaces inchangés. Ce cas n'exerce pas une collision SSA sur des namespaces J2 préexistants.
5. **SA/RBAC/NetworkPolicy PASS** : quatre SA apparatus-* ; Roles et RoleBindings admit/controller dans system, controller dans plugins ; `plugins-isolate` et `system-deny-plugins` conformes aux manifests attendus.
6. **Pod/pin PASS** : `plugin-98ba747fc572de29d76dfd08f9253778`, UID `52881667-cd41-4228-b9ff-2062f6fc6248`, créé à 09:10:11Z, Running/Ready, 0 restart. Image et imageID : `apparatus-reference-kv@sha256:9e932a31035db933f69508497be7ff72b2296dcb8d2f9f7ed25c41c48adebc47`. Labels binding `db31c8dd-eabd-4096-b6c0-e0ea30656ae0`, project `843a8f89-bc2c-4bea-9405-1b2fee582b8c`. SA default observé, pas de contrat contraire dans le lot.
7. **AdmissionRecord PASS** : UID `7c9312fe-cc94-4301-8d5c-cf25be44cb9b`, créé à 09:10:10Z, phase VALID ; catalogue `sha256:98ba747fc572de29d76dfd08f92537782bf0353b652adcace10200020ee560cf`, même pin CRI et mêmes labels que le Pod. PolicyVersion `apparatus-p4-conformance/v1`.
8. **Canary PASS** : deployment 1/1, Pod `apparatus-system-canary-686c5b4fd5-nsc5s` Running/Ready ; Service `10.96.113.93:8080`, endpoint `10.244.171.50:8080` correspondant au Pod. Présence d'une cible réelle confirmée après run ; pas de nouveau probe exécuté.
9. **Events K8s INCONCLUSIVE** : listes vides après expiration du TTL. Chronologie reposant sur creationTimestamps et événements Docker ; aucune histoire d'événements K8s inventée.
10. **Fixtures Docker PASS** : zot `c2b476f95c7dfd7f816f7a6a1a2fc20e24073b2976c189460920f03bbabdb439`, OpenBao `57c891c8e722fb950c769dde109576e556791636abd191c71b5a7646850b1912` exited, code 0 ; fin à 10:09:28.6Z / 10:09:29.5Z, SIGTERM puis stop/die (gracieux), conteneurs conservés.
11. **Helpers PASS** : cinq IDs de helpers du registre-18, create/die exit 0/destroy entre 09:10:05Z et 09:10:11Z ; aucun helper restant dans ps -a.
12. **Process PASS** : aucun cargo/kubectl/oras/cosign/m6 résiduel ; pas de port-forward.
13. **Registres PASS avec écarts** : enregistrement tardif du 18 explicite ; IDs actuels concordants ; images opaques UNKNOWN+ask, aucune suppression. Compléments historiques ci-dessous à consigner sans réécrire les anciennes lignes.
14. **Nœud tiers PASS** : runtime préexistant, aucune intervention du vérificateur ; running depuis le 17 octobre à 15:08:12Z.

## Écarts de registre découverts

- **G-1** : la paire zot/OpenBao du run m1 (699122b5433e / 7a349e813790), enregistrée arrêtée et préservée, a été remplacée par les fixtures lors d'un run ultérieur du 17. La paire 19c22d92efaa / 1ffd36d2a884 n'a pas été enregistrée et est restée active environ quinze heures avant remplacement par run4 (arrêt SIGTERM/exit 0 puis destroy). Le `prior_state=absent` du registre-18 décrit les nouvelles identités, pas l'état du nom logique de fixture : ce dernier était déjà occupé par une paire active.
- **G-2** : tag mutable `apparatus-reference-kv:m1` reconstruit ; ID actuel `sha256:52c3d7faf7eb0f0744a4f6203a16f4ec0db0bc00f416b0e24d9d5f2ccdcade62` différent de l'ancien ID enregistré 6b629795e389. Pas d'attribution de création sans preuve ; image conservée.
- **G-3** : réseau `apparatus_operator_test-net` non enregistré. Vérification complémentaire du lead : ID observé `1295189f15a6f57ac033145d024fd8933d3a2c5ef29c2140f0f6b2552b65f49e`, créé le 2026-09-20T17:27:29.085089284Z, donc antérieur à cette mission. Non possédé, aucune suppression.

## Limites et suite

- Pas de réexécution : preuve run4 = libtest vert 311,79 s. Captures brutes autonomes H1–H4 absentes ; conclusion de couverture réservée à P6 tests.
- Conformité au target E: non démontrée (target du worktree sur C:), hors vérification infra.
- Verdict **PASS de l'état courant**, pas une approbation de toute la sémantique du fixture ni de toute la tenue historique des registres.
- Rapports correctness/tests encore requis pour la clôture du lot. Aucun nettoyage destructif demandé.
