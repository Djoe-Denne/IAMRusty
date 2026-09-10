# Architecture Decision Records

Décisions d’architecture **acceptées ou proposées**, distinctes des notes de conception.

| Couche | Rôle | Où |
|---|---|---|
| Conception | Vision, protocoles, écarts code, plan de livraison | Wiki Manifesto (`obsidian/AI FOR ALL/projects/manifesto/`) |
| Décision | Un choix irréversible (ou coûteux à changer), avec alternatives | **Ici** (`docs/adr/`) |
| Handbook | État actuel du code et recettes d’implémentation | `docs/` (hors `adr/` et `reviews/`) |

`docs/project/Archi.md` est l’ADR historique du Project Service (cible 2024–2025, en partie caduque). Ne pas y empiler Apparatus.

## Règles

1. **Une décision par ADR.** Pas un dossier de conception recollé.
2. **Le titre est la décision**, pas le thème (« Le binding est une extension 1:1 de ProjectComponent », pas « Bindings »).
3. **Statut** : `Proposed` → `Accepted` (revue explicite, typiquement PR) → `Superseded` / `Rejected`. Une note wiki `status: proposed` n’est pas une ADR acceptée.
4. **Ne pas ADR** : matrices de tests, noms de crates, SLO, moteur d’isolation, qui peut publier, rétention — tant que ça reste un arbitrage ouvert (`APP-01` … `APP-07` dans le plan wiki).
5. **Les notes wiki restent.** Une ADR cite ; elle ne remplace pas bindings, capabilities, Factory, UI.
6. **Décision et réalité sont indépendantes.** `Accepted` signifie que la cible est ratifiée ; `Unimplemented`, `Partial` ou `Implemented` décrit ce que réalise le dépôt. Une ADR acceptée n’autorise pas à présenter la fonctionnalité comme livrée.
7. **Accepter tôt les invariants qui figent P0.** Reporter les mécanismes qui dépendent de P2/P4 (worker, lease/fencing, moteur d’isolation, pipeline OCI).
8. **Traçabilité non circulaire.** La baseline Apparatus est le document utilisateur et le wiki commité le 9 septembre 2026 (`d0664e3`). Un hub wiki mis à jour après une ADR ne constitue pas une preuve indépendante de son acceptation.

## Vague 1 — Apparatus (Accepted, 2026-09-10)

| ID | Décision | Réalité | Notes wiki |
|---|---|---|---|
| [0001](0001-apparatus-binding-owned-by-manifesto.md) | Binding = `ProjectComponent` 1:1, propriété métier Manifesto | Partial | bindings, plan |
| [0002](0002-apparatus-contract-first.md) | Contrats + Apparatus KV de référence avant Factory et host | Unimplemented | platform, factory, UI, plan P0 |
| [0003](0003-apparatus-untrusted-plugin.md) | Code auteur hors des processus privilégiés | Unimplemented | platform, capabilities, factory |
| [0004](0004-apparatus-capability-gateway.md) | Gateway seule I/O ; KV plateforme ; pas de bearer IAM | Unimplemented | capabilities |
| [0005](0005-apparatus-same-protocol-valid-verified.md) | Même protocole ; admission, `VALID`, `VERIFIED` et installabilité distincts | Unimplemented | factory, platform |

Ratification : revue orchestrée de douze avis indépendants (deux passes sur cohérence, sécurité et implémentation), puis arbitrage explicite. Cette ratification fixe des **invariants cibles** ; elle ne valide ni Kubernetes, ni une Factory, ni une gateway ou un runtime déjà opérationnels.

## Traçabilité de la baseline du 9 septembre

| Recommandation documentée | Décision canonique | Encore ouvert |
|---|---|---|
| Propriété métier Manifesto | ADR-0001 | Autorité d’admission concrète (`APP-01`) |
| Processus privilégiés séparés du plugin | ADR-0003 | Déploiement des workers |
| Migration 1:1, UUID et ACL `component` conservés | ADR-0001 | Migration P1 |
| Un Apparatus canonique du même type par projet | ADR-0001 | Plusieurs instances hors V1 |
| Identité d’installation immuable, jamais `latest` | ADR-0002 | Admission et artifacts P4 |
| Desired/observed, génération, lease et fencing | — | ADR avant P2 |
| Gateway = ACL ∩ consentement ∩ politique | ADR-0004 | Implémentation P3 |
| Runtime managed, plugin isolé | ADR-0003 | Moteur et plateforme (`APP-01`) |
| Host à créer ; UI schema ou bundle statique | ADR-0002 (contrat seulement) | Sécurité du host avant P5 |
| KV par binding ; secrets par référence | ADR-0004 | Rétention, quotas et produit secrets |
| Git → artifacts → conformance → admission | — | ADR avant P4 |
| Même protocole ; `VALID` distinct de `VERIFIED` | ADR-0005 | Publishers (`APP-02`) |

Hub wiki : `obsidian/AI FOR ALL/projects/manifesto/decisions/index.md`.

## Comment en ajouter une

1. Copier [template.md](template.md) → `NNNN-verbe-court.md` (prochain entier libre).
2. Remplir Contexte / Décision / Conséquences / Alternatives. Lier les notes wiki et, si ça en remplace une, l’ADR superédée.
3. Ajouter la ligne dans ce README et dans l’index wiki.
4. Ouvrir une PR. **Accepted** seulement après accord explicite sur le texte final, pas parce qu’une note wiki le cite.

## Hors vague 1 (pas d’ADR tant que le jalon n’ouvre pas)

| Sujet | Quand |
|---|---|
| Génération, lease, fencing du contrôleur | Avant P2 |
| Pipeline Git → OCI, builders, registry | Avant P4 ; bloqué par `APP-01` |
| Host UI, origines iframe, MessageChannel | Avant P5 |
| Moteur d’isolation, CPU, budget (`APP-01`) | Avant tout runtime réel |
| Politique catalogue / publishers (`APP-02`) | Avant soumissions tierces |
| Révocation : drain/destruction des bindings déjà `ready` | Avant P4/P6 ; le refus de nouvelles opérations est déjà fixé par ADR-0005 |
