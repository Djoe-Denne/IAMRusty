# Rapport 81 — revues persistées, complément compatibilité borné

## Verdicts acquis

Relecture Rust2002Z persistée/indexée : HTTP RP-1/RP-2 résolus HIGH_CONFIDENCE statique, GO_WITH_COMMENTS ; budgettightening/driverpanic restent suggestions non bloquantes, pas nouveaux defects. T4 paquets2006/2015 persistés/indexés : **C-1 original clos ciblé sur owner et13preuves relues**, BLOCK C-2/C-3. Les anciennes limites de non-réception des logs sont levées ; aucune relecture/rejeu demandée pour documentation.

Précision lead conservée au paquet2015 : owner est byte-identiqueb7→9e, lib.rs ne l'est pas (diff format seulement, exports identiques). Ne pas propager la formule erronée de T4 «owner/src-lib vide» comme preuve.

## Travail prêt et dépendances

QA complémentaire persistée `.cursor/qa-plans/20261026-http-compatibility-c2-c3.md` : auteur tests pour témoinH2setting+ExtendedCONNECT, vrai callerport0 ou proposition seam minimal à auteurproduction, et trois assertions RunningApp0/2 avec oracleHTTP possédé/joignable conservé. SourcesSDK9e isolées et entrées locales à fournir avant mutation. Pas de codeproduction/harness/feature/depsnouveau ni Docker/IT dans cette première étape.

Lot correctif production dépend des témoins/contrainteport0 réellement intégrés : il reste au plan, non publié à des consommateurs isolés avant leurs entrées. C-2/C-3 appartiennent déjà à la garantie de compatibilité autorisée, pas nouvelle extension de backend/API. Si oracle exige un nouveau feature/harness ou helper non réalisable dans la surface, arbitrage AVANT modification, pas affaiblissement silencieux.

Les preuves C-1/RP acquises restent valides dans leur portée ; ne pas relancer13tests au seul motif des paquets persistés. Après correction, tests impactés et scénarios existants bloqués par cardinalités à vérifier honnêtement ; runtime fixtures uniquement après préflight/inventaire/bail. QA proportionnée inchangée, pas chaîne de reviewers supplémentaire. C-2/C-3, publication et pin restent ouverts. T5 documentaire reste préservée et non intégrée prématurément.
