# Arbitrage Proposed — ancre causale minimale sans fuite native

## Objectif et faits

Mission1 k6 : Chromium Linux sandboxé et un GET public IAM, aucune mission auth/charge/service/SDK/déployeur/tunnel. ADR0504/V1 Accepted/Partial ; diagnostics parents et comparaison22 Accepted, AppArmor durable Proposed NON autorisé. Lire le canon actuel dans le checkout principal, pas la proposition22 non ratifiée encore présente dans l'ancien arbre architecte.

Source783 publiée6b58108, run40982576736 :129tests Linux OK sans skip22/24, outils/versions/inspectPASS ; k6 pré-cas/SCRIPT_ABORTED.24 directNoUsableSandbox/privacyFAIL ;22 direct corrélé SIGABRT/NATIVE_OBSERVED_BEFORE_CLEANUP/privacyPASS/cleanupenfantsPASS, aucune cause lexicale reconnue. Parent runtime/HTTP/Chromium INCONCLUSIVE. Le propre cleanup Child ne cause pas le signal primaire observé. Statut/signal seuls ne donnent ni site, instruction ni émetteur.

Suivi expert complet `result_mvu9thwl_0000_1wcyyv`, même session, persisté `20261030-k6-signal-expert-result.md`. Petite famille de formats fermés possible dans le contrat existant, mais risque de rester OTHER/NO_TEXT ou FATAL sans cause. L'implémentation de cette famille est une branche indépendante, autorisée sans aucune ancre source. Ne pas attendre ni modifier cette branche ; aucun essai supplémentaire n'a eu lieu depuis40982576736.

## Décision demandée à architecte avant toute implémentation d'ancre

Proposer la plus petite observation qui puisse identifier une cause de démarrage **sans** exposer un flux natif ou dériver en framework/probes/longue liste spéculative. Comparer une ancre publique source/assertion strictement fermée/versionnée à une autre voie réellement compatible. Le contrat actuel n'autorise pas implicitement texte même court, chemin/localisation ou condition d'assertion ; ne pas appeler cela un simple label pour contourner cette frontière.

Définir précisément données admises/refusées, canal/projection/publication, confidentialité (URL/credentials/sentinelles/UTF8/overflow), provenance du binaire et de l'ancre, bornes, refus sur inconnu, conditions d'exécution, contrôles/cleanup inchangés et oracles positifs/négatifs. Un hash arbitraire ou un nom encodé ne contourne pas l'interdiction d'export. Ne pas promettre l'identification d'une cause depuis SIGABRT ou FATAL seul.

Chercher et recouper les sources officielles156/k62.3 pertinentes avant recommandation. Ne pas construire un corpus massif Chromium ni ajouter des dépendances/serveurs/sondes pour résoudre ce diagnostic ; simplicité et reproductibilité dans l'architecture actuelle. Si aucune ancre sûre et utile n'est possible, le dire et exposer le plus petit arbitrage humain réellement nécessaire, pas une demande générale de permissions déjà déléguées. Les décisions nécessaires dans mission1 sont déléguées au lead, mais aucune relaxation silencieuse d'une contrainte ou nouvelle frontière hors mandat.

## Scope d'écriture et sortie

Arbitrage documentaire uniquement, pas décision Accepted. Écrire une proposition courte dans **un nouveau fichier** `docs/contracts/k6-chromium-causal-observation.md` ; ne pas modifier le contrat ancien de votre arbre, la source, le README, le ledger ou la CI. Le lead intégrera puis ratifiera seulement ce qui est compatible ; le consommateur d'ancre ne sera pas publié avant cette barrière.

Pas runtime/essai/installation/Chrome/Linux/Cargo/CI/privilege/strace/unshare/téléchargement exécutable ou sous-agent. Pas de profil AppArmor, helper SUID, wrapper, no-sandbox, changement de pin/argument ou déployeur/tunnel. Réutiliser les preuves et rapport33, pas redécouverte exhaustive. Livrer options écartées et recommandation, frontières/autorité exacte, sources et diff-check réel ; signaler les limites sans PASS inventé.
