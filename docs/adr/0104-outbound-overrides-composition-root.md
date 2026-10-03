# ADR-0104 : Overrides sortants = sac typé local au setup consommateur (`*OutboundOverrides`)

- Statut : Proposed
- Réalité : Partial
- Date : 2026-09-27
- Décideurs : Djoé Denne (plan accepté 2026-09-27) — ratification `Accepted` non encore prononcée
- Jalon concerné : architecture actuelle (hors P-Apparatus)
- SuperSède : aucune ([0102](0102-setup-composition-root.md) clarifiée, **pas** SuperSédée)
- SuperSédée par : —
- Related : [0102](0102-setup-composition-root.md), [0306](0306-hive-iam-configuration-signature.md), [0307](0307-workload-identity-port.md), [0404](0404-runtime-microservices-et-monolithe.md)

`Proposed` fige la cible plateforme pour l’injection d’adapters sortants au composition root. `Réalité : Partial` : sacs `HiveOutboundOverrides` / `LazaretOutboundOverrides`, `AppBuilder::with_outbound`, façade Manifesto `binding_grant_snapshots()`, ponts InProcess dans `runtime/monolith/` (IAM signer + binding grants) et fail-closed monolithe sont livrés. Pas encore `Accepted` ; les autres ports sortants futurs restent hors preuve.

## Contexte

Le dual runtime ([0404](0404-runtime-microservices-et-monolithe.md)) exige que le même port sortant soit câblé en **HTTP** (standalones) ou en **InProcess** (même process, `oodhive-monolith`). La tentation est un framework DI, un locator, une crate partagée inter-hexagones, un booléen `in_process`, ou une `HashMap`/`Any`.

[0102](0102-setup-composition-root.md) fixe `setup/src/app.rs` comme **seul** composition root du service : le binaire microservice charge la config et appelle `Application` / `run` — il **ne câble pas** les adapters. Il manquait une règle plateforme pour **comment** l’hôte monolithe fournit des adapters sortants **sans** second root ni DI.

Instance déjà visible (0306) : setter nommé `with_iam_organization_signer_client` + pont InProcess dans `runtime/monolith/` — cas Hive→IAM, pas encore le motif générique documenté ici.

## Décision

1. **Un sac typé d’overrides sortants**, local à chaque crate `*-setup` **consommateur** (ex. `HiveOutboundOverrides`, `LazaretOutboundOverrides`). Pas de crate partagée inter-hexagones pour ce motif.
2. **Un seul objet** entre dans le composition root : champs nommés `Option<Arc<dyn Port>>` du domaine **consommateur**. `Default` = tout `None` = client HTTP construit par le setup.
3. **`app.rs` reste le seul assembleur** ([0102](0102-setup-composition-root.md)). API : `AppBuilder::with_outbound(overrides)` ; les setters nommés restent du **sucre** qui remplissent le sac.
4. **Clarification 0102 (sans SuperSéder)** : le binaire service ne câble pas ; l’hôte [0404](0404-runtime-microservices-et-monolithe.md) (`oodhive-monolith`) **peut** construire des adapters-pont InProcess dans `runtime/monolith/` à partir des façades des setups **fournisseurs**, puis remplir le sac du consommateur avant `build`.
5. **Binaire microservice** : sac vide (`Default`). **Seul** `oodhive-monolith` construit et injecte les ponts InProcess.
6. **InProcess = capability** (`Arc<dyn Port>`) — **pas** de token interne. **HTTP** (standalones **et** routes internes nestées) = token fail-closed ([0306](0306-hive-iam-configuration-signature.md), [0307](0307-workload-identity-port.md)). Credential 0307 **seulement** sur le chemin HTTP.
7. Le « générique » est le **motif** (un sac par consommateur), **pas** un framework. Pas de DI container, pas de service locator, pas de booléen `in_process` de config.

**Hors décision** : mesh AuthN JWT ([0308](0308-mesh-authn-jwt.md)) ; remote signer ([0309](0309-remote-signer.md)).

## Conséquences

- Chaque nouveau port sortant cross-hexagone consommateur ajoute un champ `Option<Arc<dyn …>>` au sac du setup concerné — pas une nouvelle crate ni un registre global.
- `runtime/monolith/` connaît les hexagones qu’il compose ; les crates `*-application` / `*-infra` / `*-setup` consommateurs **n’importent pas** les crates fournisseurs.
- Les setters nommés (ex. Hive→IAM 0306) convergent vers le sac ; la politique de transport 0306 (InProcess vs HTTP) **reste inchangée**.
- Les IT et le binaire service restent sur HTTP par défaut (sac vide).

## Alternatives rejetées

| Option | Pourquoi pas (maintenant) |
|---|---|
| N setters sans sac | Pas de point d’entrée unique ; explosion d’API builder ; hôte monolithe difficile à composer |
| DI container | Contredit 0102 ; second root implicite ; coût et indirection hors RustyCog |
| `HashMap` / `Any` | Perte de typage ; erreurs au runtime ; opaque pour les reviews |
| Crate partagée Hive+IAM ou Lazaret+Manifesto | Frontière hexagone cassée ; InProcess doit vivre dans l’hôte seulement |
| Booléen config `in_process` | Mélange config et composition ; risque de faux InProcess hors monolithe |

## Non décidé ici

- Ordre exact de migration des setters nommés existants (0306) vers `with_outbound` (détail d’implémentation).
- Liste exhaustive des ports Lazaret→Manifesto (ou autres) à placer dans chaque sac.
- [0308](0308-mesh-authn-jwt.md) et [0309](0309-remote-signer.md) — hors substitution InProcess/HTTP de ce motif.

## Références

- Composition : [0102](0102-setup-composition-root.md) ; dual runtime : [0404](0404-runtime-microservices-et-monolithe.md)
- Instance transport Hive→IAM : [0306](0306-hive-iam-configuration-signature.md) ; credential HTTP : [0307](0307-workload-identity-port.md)
- Preuve d’implémentation (`Réalité : Partial`) :
  - `services/Hive/setup/src/app.rs` — `HiveOutboundOverrides`, `with_outbound`, sucre `with_iam_organization_signer_client`
  - `services/Lazaret/setup/src/app.rs` — `LazaretOutboundOverrides`, `resolve_binding_grant_snapshots`, `with_outbound`
  - `services/Manifesto/setup/src/app.rs` — getter `binding_grant_snapshots()`
  - Ponts : `runtime/monolith/src/in_process_iam_signer.rs`, `runtime/monolith/src/in_process_binding_grant.rs`, câblage fail-closed `runtime/monolith/src/runtime.rs`
- Setter nommé Hive (sucre) : toujours l’instance 0306 ; le motif plateforme est cette ADR
