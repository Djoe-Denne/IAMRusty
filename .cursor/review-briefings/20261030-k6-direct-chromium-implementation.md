# Implémentation bornée — diagnostic parent Chromium direct

## Entrée et objectif

Lire le nouvel addendum **Accepted 2026-10-30** dans docs/contracts/k6-functional-linux.md et le rapport19, avant code. V1/ADR0504 Accepted/Partial, modèle/QA de mission inchangés. Source actuelle ops/tests/k6 au commit08f6c03, publiée dans f8df62f ; les commits ultérieurs jusqu'à b637fd7 sont documentaires. Le lead fournit le commit intégré de cette admission avant publication de la tâche.

Cause propre aux trois runs k6 toujours inconnue ; dernier run40968668360 : 70tests Linux verts, versions/inspect verts, native k6 NONZERO/summary courant valide/expected1/started0/completed0/iterations1/requests0, aucun renderer observé. Recherche avant nouvel essai : restrictions AppArmor/CfT connues et documentées ; parseur k6 perd certains FATAL. Rapport18 contient les sources consultées. L'hypothèse à discriminer est une défaillance native de Chromium masquée par le parseur, pas une correction du scénario.

## Ownership

Un auteur unique pour `ops/tests/k6/**` et, seulement si nécessaire, `.github/workflows/k6-functional-smoke.yml`. Pas ADR/contrat, CI existante, service, identité ou SDK. Précédents auteurs arrêtés et leurs arbres sauvegardés ; nouveau worktree propre/fresh, aucun reset/rebase/écrasement de ces arbres. Lead seul pour ledger, exécution runtime/CI, publication et baux. Pas Cargo/hook worker (Windows206 connu cheminlong, ne pas reproduire/bypass).

## Livrable minimal

- Une commande explicitement diagnostique du lanceur unique, distincte de `browser`, sans fallback dans celui-ci. Au plus un Chromium direct par invocation, binaire identique revalidé par pinned_tools et identités, versions avant usage, même environnement fermé et superviseur Child existants. Ne pas dupliquer ces gardes ou créer un nouveau framework/exécuteur.
- Arguments fixes vérifiés contre la documentation officielle Chromium/headless : about:blank seulement, headless, profil privé neuf, sortie minimale bornée et trafic d'arrière-plan prévenu. Aucun wrapper, port/debug CDP, UI, capture, flag sandbox/TLS affaibli, URL/argument/exécutable libre.
- Oracles fermés distincts : source `CHROMIUM_DIRECT`/`K6`, catégorie native/lexicale, confidentialité, cleanup. Réutiliser les classes existantes. La table de codes k6 n'est nécessaire que si utile et jamais appliquée aux exits Chromium.
- Le diagnostic direct **ne qualifie jamais** HTTP/Chromium : même exit natif zéro/renderer aperçu/témoin observé reste Chromium INCONCLUSIVE. Ne pas neutraliser erreur native/privacy/cleanup/sandbox pour obtenir un succès. Aucun fichier natif ou artefact d'échec publié.
- Métadonnées hôte facultatives mais utiles à la comparaison : seulement les fichiers/valeurs précis admis à l'addendum, lecture bornée et projection typée/fermée ; absent/refusé/illisible/zéro distincts, jamais valeur brute arbitraire. Aucune commande OS supplémentaire, unshare, sudo, énumération de profils/journaux ou modification hôte.
- Si raccord CI utile : appel **explicite séparé** du parent sur le même job/UID/pins/environnement et racine privée que le cas k6 comparé. Préserver son échec et le skip d'upload ; aucun continue-on-error, dispatch, déployeur ou secret. Corrélation privée documentée, différences d'arguments explicites. Une expérience comparative contient un cas k6 et au plus un direct, pas de boucle.

## Vérification

Recherche officielle versionnée avant les choix d'arguments, sources retenues/écartées documentées brièvement. Tests d'abord ROUGE puis GREEN Windows natif : argv fixe/profil neuf, pins/identités avant Child, zéro sans qualification, native erreur/timeout/interruption, sentinelles/invalides/dépassement, refus métadonnées et sorties forgées, cleanup et non-publication. Réutiliser fixtures Python existantes, pas serveur HTTP/mockruntime. Les tests Linux manquants restent absents jusqu'à exécution du lead.

Rendre diff exact sur le HEAD fourni, commandes et résultats réels, invariants et limites, aucun PASS global. Préférer un petit raccord à la supervision/projection existantes ; pas d'abstraction générale de diagnostic ni de collection d'options. Tout besoin de modifier profil AppArmor/SUID/sysctl/pin/hôte ou une méthode de preuve remonte avant code.
