# ADR-0001 : Le binding Apparatus est une extension 1:1 de ProjectComponent, propriété Manifesto

- Statut : Accepted
- Réalité : Partial
- Date : 2026-09-10
- Décideurs : Architecture AIForAll — ratification orchestrée du 2026-09-10
- Jalon concerné : P0 (contrat d’identité), P1 (persistance)
- SuperSède : aucune — affine le modèle composant de `docs/project/Archi.md` sans le remplacer
- SuperSédée par : —

## Contexte

Manifesto attache déjà un `ProjectComponent` (UUID, `project_id`, `component_type`, statut, unicité par projet) et des ACL d’instance OpenFGA. Le catalogue vivant est un HTTP externe (`ComponentServicePort`), pas une release à digest.

La tentation est de créer une ressource publique `ProjectApparatusBinding` avec un second UUID, un nouveau type FGA, ou un microservice « catalogue Apparatus ». Cela dupliquerait l’identité d’installation et imposerait une migration des grants.

## Décision

1. **Manifesto** reste le propriétaire métier des métadonnées du catalogue, du binding et du consentement. Il n’est pas, par cette propriété, l’autorité d’admission : enregistrer une release ne la rend ni `VALID` ni installable. Ce n’est pas un nouveau service RustyCog en V1.
2. L’identité d’installation **est** `project_components.id`. Le desired state Apparatus (release/digest, génération, consentement, source `legacy|managed`) s’ajoute en relation **1:1**, sans second UUID public.
3. Les tuples OpenFGA `component:{id}` et les événements `ComponentAdded` / `ComponentRemoved` sont **conservés**. Pas de nouveau type FGA pour le binding V1.
4. **Un** Apparatus canonique du même type par projet, conformément à l’unicité SQL actuelle. Plusieurs versions peuvent coexister **entre** projets, pas en plusieurs instances dans un même projet.

L’identité immuable d’une release et l’interdiction de `latest` relèvent d’ADR-0002. L’admission et l’installabilité relèvent d’ADR-0005. Le détail desired/observed, lease et fencing reste dans la note bindings ; une ADR dédiée devra le figer avant P2.

## Conséquences

- P1 : migration additive réversible ; backfill `source=legacy` avec release non résolue et contrôleur off ; collision de deux anciens types mappés vers le même Apparatus = rejet, pas de fusion silencieuse.
- `component_type` n’est pas réécrit pendant le backfill. Un éventuel `apparatus_id` est un attribut distinct ; le mapping legacy doit être injectif.
- Les routes `/components` et `component_id` restent valides. De nouvelles routes binding sont un alias, pas une seconde ressource.
- `apparatus-events` (statut `project_id + component_type`) reste le chemin **legacy**. Un événement dépourvu de `binding_id` et de génération n’affecte jamais un binding managed.
- Factory, registry OCI et host UI peuvent être d’autres processus ; ils ne deviennent pas propriétaires du binding.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Nouveau type FGA `apparatus_binding` | Migration des grants, double ACL, risque de divergence |
| Microservice Catalogue dès V1 | Le propriétaire métier est déjà Manifesto ; l’admission demeure une autorité séparée de la simple écriture du catalogue |
| N instances du même Apparatus par projet | Casse l’unicité actuelle ; hors V1 (note « évolutions conservées ») |

## Non décidé ici

- `APP-02` (qui publie / installe).
- `APP-07` (changement d’organisation ou déplacement du projet). Le transfert d’un rôle owner déjà livré ne réécrit pas l’identité du composant.
- Worker, lease, fencing (P2).
- Conservation d’une intention de nettoyage avant la cascade SQL, exigée avant l’activation de tout workload.

## Références

- Wiki : `apparatus-bindings-and-lifecycle`, `apparatus-platform`, `apparatus-implementation-plan`, `component-instance-permissions`
- Code : `Manifesto/domain/src/entity/project_component.rs`, `openfga/model.fga`, `apparatus-events/`
- Preuve partielle : UUID, unicité SQL et ACL `component:{id}` existent ; l’extension binding 1:1 Apparatus n’existe pas encore.
