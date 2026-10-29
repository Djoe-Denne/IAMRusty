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
