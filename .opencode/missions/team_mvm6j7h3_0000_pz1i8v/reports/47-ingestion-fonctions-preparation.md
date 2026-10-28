# 47 — Skill local d'ingestion de fonctions : préparation

## Mandat
Le « go » autorise la création du skill de capitalisation post-mission proposé.
Travail direct, aucun nouvel agent/session. Livrer la procédure manuelle et son
outillage d'inspection ; ne pas ingérer automatiquement les anciennes campagnes.

## État vérifié
- La bibliothèque existante contient3helpers et un modèle de candidate ; pas de
  fonction issue des campagnes publiée. Préserver catalogue et allowlist.
- HEAD93e376c5530a439aaafe61a64d3e881865c1acf1 ;112entrées Git préexistantes,
  AGENTS.md/opencode.jsonc déjà modifiés. Pas de commit/stash/reset.
- .agents/skills déjà découvert par le client, pas de configuration à changer.
- Documentation officielle V2 skills consultée : disable-model-invocation=true
  masque l'auto-invocation, mais le skill reste explicitement chargeable par ID.
- Documentation API V2 disponible ; pagination correcte connue : order seulement
  sur premièrepage, cursor seul ensuite. Aucun accès direct à la base OpenCode.

## Périmètre
- Nouveau skill local aiforall-context-ingest, manuel, avec modes inspect/prepare ;
  inspect par défaut. Préparation des candidates séparée de leur promotion.
- Références courtes depuis AGENTS et le skill de fonctions existant ; pas de
  politique globale, hook, daemon, serveur, modèle ou autorisation modifiés.
- Inspecteur Node sans dépendance : entrée JSON explicite de traces tool/text,
  inventaire métadonnées/hashes seulement, pas de code ou log brut imprimé,
  pas d'exécution/import des scripts de la trace ni d'écriture de candidates.
- Attribution conservatrice : completed/fulfilled ne vaut pas succès métier ;
  Code Mode imbriqué, alias et boucles exigent une revue manuelle des résultats.

## Vérification prévue
Tests Node positifs/négatifs de l'inspecteur, absence d'exécution/fuite de données,
formats supportés, limites, pagination, statuts et doublons exacts. Smoke sur une
trace réelle déjà collectée (métadonnées seulement), sans extraire/promouvoir une
fonction. Vérification des liens/frontmatter et chargement explicite du skill.
Pas de source métier explorée, tests applicatifs, Cargo, Docker/Kind/WSL ou Go.
