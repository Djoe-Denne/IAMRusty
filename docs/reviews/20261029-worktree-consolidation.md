# Consolidation des worktrees — revue de conservation

## Résultat et portée

Revue indépendante statique de onze instantanés de livraison contre la version
principale `052817d61885e9585e1ecc491974915bf85b7199` : **GO_WITH_COMMENTS**,
aucun défaut fonctionnel bloquant établi. Ce verdict n’est pas un PASS global
de sécurité, de tests ou de runtime, ni une approbation de fusion aveugle.

Les versions finales déjà intégrées doivent être conservées plutôt que remplacées
par leurs anciens instantanés. La réserve documentaire a été traitée dans
[deux propositions AuthZ conservées](20261029-authz-proposals-carried-forward.md),
sans modifier les contrats Accepted.

## Preuves de conservation

- Les sélections SDK `74e8a89` et `72845d9` sont des ancêtres du pin publié
  `02e5db23aac6016aab062c3078696d5f7fd464ef` : aucun downgrade justifié.
- Les livraisons de configuration IAM du laboratoire et d’alignement des routes
  Envoy sont identiques au canon, octet pour octet.
- Manifesto : setup et tests identiques ; les autres écarts se limitent aux fins
  de ligne. Le diff normalisé est vide. Leur intégration a nécessité une
  normalisation Git conforme à `text=auto/eol=lf`, sans différence fonctionnelle.
- Instrumentation Python : aucune fonction ou méthode de test historique n’est
  disparue dans les comparaisons AST. Les corps modifiés ont été examinés : ce
  constat ne se limite pas à une comparaison de noms de fichiers.
- Corrections ultérieures à préserver : Admin plafonné à128Kio, mapping du port
  local, décodage conntrack fermé, postimage liée à l’ACK du PATCH plutôt qu’au
  premier GET. Sources examinées : `ops/deploy/local_full_target.py` lignes375,
  507 et743 ; `ops/deploy/deploy-local-full.py` lignes637–650, dans la base citée.
- Différences IAM/Hive sous-jacentes examinées : filtrage du lien par organisation,
  autorité OAuth configurée, fixtures et outbox corrigées. Revenir aux anciens
  instantanés serait régressif.
- Les contrats ratifiés et états actualisés priment sur les propositions restées
  dans les copies auteur. L’alternative ADR0315 n’est pas une décision manquante
  à promouvoir.

## Réserve documentaire et disposition

Le contrat Proposed de l’instantané `fedca587` conserve deux précisions absentes
textuellement du contrat principal : liaison durable storeFGA/autorité plateforme
(ligne43), invitations UUID account-only et oracle associé (lignes64 et101).
Leur absence ne démontre pas un bug ; elle empêche néanmoins de conclure à une
équivalence documentaire complète sans disposition explicite.

L’architecte a qualifié les deux propositions par rapport à ADR0314 et au contrat
ratifié. Le lead a ratifié **leur conservation documentaire non normative**,
pas leur acceptation technique. Elles ne deviennent ni migrations, ni garanties
runtime, ni nouveaux tests obligatoires par cette consolidation.

## Références des instantanés

- Contrat principal proposé : `fedca587fc278177c396064d83b6ab5a46281305`.
- Sélections SDK : `d784fbb8e67c5f7139a95f2c9d05aa81d6ba60ce`, `fd168524ed66d6bfc9af5e6876bd6fbf4fb306fa`.
- Contrats secondaires : `eae63ce0054de819276f3a9009d413fbc27e56f1`.
- Instrumentation/tests : `d4942857dad35eb73811f6b9ddbbecc621886075`, `3d1f2358ec2263eb1051b3fce92566a0027dde26`, `23fbaaa1c300ca9370d8738916de07df27c54204`, `e91c1073bbc19c48cd69acd0e6bd754213f32ef2`.
- Configuration/Manifesto/Envoy : `c2cb4d2ad37027b353ad69800a03b03dac1bcd07`, `78e777821c4e60142c54773151ca2ff0624589c0`, `fe3b41a2b05d1008cc5c36ed6035472d0d4c0606`.

Ces références identifient les copies historiques conservées localement avant
retrait des worktrees ; leur accessibilité depuis un clone distant n’est pas
promise. Le contenu retenu et les propositions utiles sont dans la branche
principale. Une sauvegarde Git incrémentale locale a été vérifiée avant retrait.

## Vérifications et limites

Le reviewer a utilisé `git show/diff/ls-tree`, l’ascendance SDK, la comparaison
normalisée et une analyse AST sans exécuter les modules. Il n’a lancé aucun Cargo,
fmt, test, runtime, écriture ou publication. La conservation des tests ne prouve
ni leur couverture ni leur réussite. Aucune ressource Docker créée.

Le contrôle `cargo fmt --all -- --check` de la source canonique a été exécuté
séparément par le lead avec succès. Les fmt des chemins historiques ont rencontré
des erreurs Windows206 ou une exécution indéterminée ; ils ne sont pas déclarés
réussis. Les preuves fonctionnelles antérieures restent bornées par
[l’état qualifié](../contracts/20261029-auth-security-qualified-state.md).

## Résolution et archivage natifs

Après conservation des propositions utiles, les dernières branches déjà
incorporées ont été rapprochées par les outils natifs avec une résolution Git
de conservation (`ours`) : préserver la version finale revue, pas réintroduire
les instantanés remplacés. Ce choix n’est pas applicable à une livraison nouvelle
non examinée. Aucun merge Git manuel ni changement du plugin n’a été nécessaire.

Les options Git locales temporaires ont été retirées après l’opération. Les
permissions de réparation ajoutées aux seules sessions de préparation ont aussi
été retirées, sans changer leurs autres règles. Le cleanup natif a terminé les
dix derniers rapprochements, puis confirmé la mission archivée ; les33tâches
étaient terminées. Les23enregistrements de worktree de ce lot ont disparu de
l’inventaire natif ; aucun worktree Git de ce lot ne subsiste.

L’avertissement générique du cleanup signalait un chevauchement sur le sous-module
rustycog. Vérification après opération : pin canonique02e5 inchangé, fichier Serena
étranger et workspace Obsidian identiques à leurs empreintes préalables. L’arbre
versionné est resté exactement celui du commit de conservation6bf7bf1, sans nouveau
delta fonctionnel. Les autres anciens worktrees ne sont pas supprimés par ce lot.

Les contrôles de sauvegarde et de propreté ont précédé les retraits. Les cinq
clones SDK imbriqués étaient propres, sans fichiers ignorés/non suivis et sur des
ancêtres du pin publié ; ils ont été désinitialisés sans force, pas leurs versions
historiques réintroduites. Aucune ressource runtime créée ou supprimée, aucune
purge d’archives et aucun force-push.

## Profils de projet inclus sur demande explicite

Des modifications de24profils sous `.opencode/agents/` ont été observées pendant
la clôture, puis explicitement demandées au commit et à la publication par
l’utilisateur. Contrôle du diff : seuls des ajouts `edit` / `resource: "*"` /
`effect: allow`, sans changement de modèles, de prompts ou des autres règles.
Les24profils sont acceptés par le parseur natif OpenCode, et cette règle est leur
dernière règle générique d’édition effective. Commit5966821.

Ces permissions autorisent techniquement l’écriture, y compris pour les profils
de revue. Elles ne constituent pas un mandat d’édition pour une tâche explicitement
READONLY ; cette frontière reste à respecter. Ce changement local persistant est
distinct des autorisations externes temporaires de réparation, qui ont été retirées.

Contrôles finaux exécutés séparément, Windows natif et séquentiels :
`cargo fmt --all -- --check` et
`cargo fmt --manifest-path rustycog/Cargo.toml --all -- --check`, tous deux exit0.
Pas de nouvel essai fonctionnel, de build Linux ou de qualification supplémentaire.
