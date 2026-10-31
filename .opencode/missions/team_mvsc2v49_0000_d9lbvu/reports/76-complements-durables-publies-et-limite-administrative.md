# 76 — Compléments durables publiés, traces temporaires préservées

## Résultat

Après la clarification utilisateur, les conclusions de revues restées locales et
les décisions d'intégration utiles sont désormais publiées sous `docs/`, avec
liens depuis le handbook et le wiki. Une règle réutilisable distingue livrables
des worktrees, instrumentation permanente et sorties jetables de campagne.

Commit `052817d61885e9585e1ecc491974915bf85b7199`, publié sur master de
`github.com/Djoe-Denne/IAMRusty`. HEAD local et référence distante identiques,
index vide et zéro commit non publié. Sept fichiers sélectionnés explicitement.

Documents principaux :

- `docs/guides/livrables-durables-et-worktrees.md` ;
- `docs/missions/20261029-auth-security-publication.md` ;
- `docs/reviews/20261029-auth-security-final-evidence.md`.

## Preuves et portée

Les constats du rapport75 sont repris sans surinterprétation : pas de chemin de
code manquant identifié dans les worktrees de mission examinés ; deux documents
absents appartiennent à une alternative explicitement rejetée. Les versions
anciennes ne sont pas fusionnées sur les corrections ultérieures. Une présence
de chemin ou une comparaison CRLF ne garantit pas l'équivalence fonctionnelle.

Complément : recherche des sources/docs ignorées dans les répertoires durables.
Six candidats hors caches trouvés : cinq manifests de campagne sous ops/certs,
et un README de certificats IAM obsolète HS256/RS256. Ils ne sont pas promus.
Pas d'autre candidat source/doc ignoré trouvé dans ce périmètre à cet instant ;
pas une garantie sur tous types de données ni tous anciens worktrees du dépôt.

Liens Markdown locaux des quatre documents handbook/compléments validés sans
erreur. Manifeste wiki JSON valide et six hashes récents conformes, dont les deux
nouvelles sources. `git diff --check` et `git diff --cached --check` réussis.
Aucun Cargo, test fonctionnel, Docker, Kind ou nouveau worker lancé. Aucune
modification applicative, aucun nouveau résultat de sécurité/runtime prétendu.

État étranger préservé : workspace Obsidian modifié, réglage Serena du SDK dirty
sans changement de gitlink, worktrees et état natif local. Aucun reset, suppression,
fusion aveugle ni nettoyage forcé. Les sorties privées restent privées.

## Frontière administrative restante

Le rapport72 a été relu : trente tâches terminées ; fermeture native sans toucher
les arbres conservés non identifiée par l'enquête précédente. Le réveil système
n'apporte ni nouvelle commande de clôture ni permission de contourner le plugin.
`team_cleanup` peut intégrer des branches et bloque sur les arbres non committés :
il n'est donc pas lancé comme raccourci administratif.

Travail autorisé restant après cette publication : aucun défaut ou livrable
concret supplémentaire identifié dans ce lot. Condition de clôture administrative :
opération native permettant de terminer la mission sans fusion/suppression des
arbres préservés, ou mandat distinct pour leur traitement. Ne pas rouvrir la
campagne auth, lancer k6 ou modifier l'état du plugin pour satisfaire le réveil.

### Nouveau réveil après publication

Message persistant relu via `team_results`, puis statut natif contrôlé une fois :
30 tâches sur30 terminées, mission toujours `long_run/running`, récupération
`wake_attempted` (dernier contrôle natif affiché18:09:49Z). Aucun nouveau livrable,
finding, message de worker ou admission à traiter ne ressort de ce réveil.
La condition administrative décrite ci-dessus reste inchangée. Pas de nouvelle
publication, relance de QA, suppression ou réécriture d'état pour faire taire
ce mécanisme ; le travail utile publié reste au commit052817d.
