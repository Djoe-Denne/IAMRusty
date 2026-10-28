# 152. ADR réconciliées avec la livraison Telegraph et SDK

## Résultat

La photographie canonique correspond désormais à la livraison intégrée :

- ADR0313 : **Proposed / Implemented**, dans son périmètre Telegraph.
- ADR0503 : **Proposed / Implemented**, API additive et adoption HTTP dans
  Telegraph standalone et le listener du monolithe.
- ADR0301 : **Accepted / Partial**, écart IAM inchangé. Une note de suivi
  distingue les faits actuels de sa photographie historique ; son corps historique
  est conservé intégralement.

Index ADR, en-têtes et états des deux contrats actualisés. Les mentions de code
encore absent ou de gates avant implémentation sont replacées dans la chronologie.
Les exigences contractuelles, interfaces et critères AC1–AC7 ne changent pas.

Les références différencient les témoins HTTP exécutés sur les SHA successifs,
les revues statiques, le composant après lease et la verticale SQS/OpenFGA/HTTP.
La fusion des branches principales n'est jamais transformée en preuve runtime.
Les réserves Clippy/Sonar/CI, le timeout109 non expliqué et les hors-scope
exactly-once/fencing/backfill sont conservés.

## Autorité et substitution

L'architecte natif a été bloqué avant travail par quota fournisseur (rapport151).
L'utilisateur a explicitement autorisé « Réconcilier directement » après exposé
de la substitution. Le lead a appliqué les instructions architecte localement
pour cette mise à jour factuelle bornée, sans nouveau modèle/override de politique,
nouvelle décision structurelle ou ratification Accepted. Trois digests Serena
ont été écrits via MCP dans le même tour, racine AIForAll vérifiée.

## Vérifications réellement effectuées

- Lecture ciblée des ADR, index, contrats et preuves finales déjà intégrées,
  notamment rapports77/79/95/139/141/150 ; pas de réexploration Rust nécessaire.
- Six documents contrôlés ;128 liens locaux résolus, aucun manquant.
- Trois digests cohérents avec les champs Statut/Réalité.
- Corps de0301 depuis sa section Contexte strictement identique à HEAD avant
  réconciliation ; les changements se limitent à une note de suivi factuelle.
- `git diff --check` réussi, aucun fichier applicatif modifié.
- Script de vérification documentaire syntax-checké et exécuté avec succès.

Pas de nouveau Cargo, IT, Docker, Kind, WSL, Sonar ou CI. Travail documentaire
seulement, sans revue indépendante ajoutée. La panne fournisseur n'est ni un
test rouge ni une tentative technique infructueuse de correction applicative.

## Publication

Les six documents, trois digests et rapports151/152 constituent le commit
documentaire sur master demandé par l'utilisateur. Les fichiers locaux exclus
(Obsidian workspace, configuration Serena du sous-module et worktrees imbriqués)
restent hors commit. Le SHA de publication et la vérification distante sont
consignés dans le ledger après push, pour éviter une auto-référence dans ce rapport.
