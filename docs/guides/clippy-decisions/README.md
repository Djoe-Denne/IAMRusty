# Registre des décisions Clippy / Sonar

Ce registre conserve les nouveaux arbitrages de correction pour les services
AIForAll, afin que des modèles différents réutilisent les mêmes décisions quand
le contexte le permet. Il complète le skill
[`aiforall-sonar-policy`](../../../.agents/skills/aiforall-sonar-policy/SKILL.md),
qui conserve les consignes des campagnes précédentes. Il ne constitue ni une
baseline de warnings, ni une liste d'exemptions, ni un remplacement des ADR.

## Recherche avant correction

1. Lire les tables du skill : `Judgment — idiome Rust`, `Skips invalidés` et
   `Campagne 2026-10-08 — règles traitées`.
2. Chercher la règle exacte et le contexte dans l'index ci-dessous ; lire les
   décisions correspondantes, pas seulement leurs titres.
3. Appliquer uniquement un précédent accepté et applicable. La même règle
   Clippy peut nécessiter des solutions différentes selon les API, le coût des
   clones, les frontières production/tests ou les versions de dépendances.
4. En absence de précédent, utiliser [TEMPLATE.md](TEMPLATE.md). Indexer la
   proposition et faire valider son adoption comme règle partagée. En cas de
   contradiction ou de choix contraire à une consigne existante, demander un
   arbitrage explicite avant cette correction.

Les corrections mécaniques déjà couvertes se réfèrent à leur section du skill
dans la PR ou le compte rendu ; ne pas recopier chaque occurrence ici. Les
anciens arbitrages restent dans le skill : ne pas les migrer ou les modifier
implicitement au nom de ce nouveau registre.

## Index des nouveaux arbitrages

Aucune nouvelle décision métier/technique enregistrée à l'initialisation du
registre (2026-11-13). Les tables existantes restent applicables.

Ajouter une puce par entrée, triée par ID, contenant :
`[ID](ID.md) — statut — règle(s) — contexte court`.
Format d'ID : `CLIPPY-YYYYMMDD-sujet-court` ; vérifier l'unicité avant création.

## Statuts et preuves

- `Proposed` : proposition documentée, **pas** une règle acquise. Les tests verts
  ou une PR ouverte ne valent pas approbation. Ne pas auto-promouvoir une
  proposition au motif qu'elle a déjà été implémentée.
- `Accepted` : validation explicite par l'utilisateur ou un reviewer mandaté
  pour l'arbitrage ; consigner qui, quand et où, ainsi que les validations du
  correctif. Un auteur ne s'accorde pas seul cette approbation.
- `Superseded` : historique conservé, lien vers le successeur approuvé. Ne plus
  appliquer la décision remplacée comme précédent courant.

Documenter les contextes exclus et les contre-exemples pour éviter une
application aveugle. Sans preuve d'une vérification, écrire « non exécuté » ou
« à confirmer ». Utiliser des références durables (issue, PR, commit, fichier,
test), sans secrets ni logs bruts. Un changement de politique se valide
explicitement ; une entrée ne peut neutraliser le skill ou une ADR.

En mandat read-only, restituer une proposition sans écrire de fichier. En
parallèle, chaque membre transmet son delta ; le lead est seul propriétaire
de l'index. Le registre est partagé par les copies `.agents` et `.cursor` du
skill, sans duplication des entrées ni modification automatique du wiki.
