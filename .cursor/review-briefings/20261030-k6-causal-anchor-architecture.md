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

## Suivi après la classification effective — sans nouvelle série d'essais

La proposition est intégrée `c2329c8`, toujours Proposed/Unimplemented, avec seule
précision lead sur les tailles (32KiB document, 256KiB budget artefacts, pas stdout).
Source classifier `1c0036f`, publiée `6c27e17` ; run40985953377 :140Linux OK sans
skip22/24, outils/inspectPASS. Direct22 corrélé rapporte stdoutEMPTY,
stderrPRESENT, CHROMIUM_FATAL_REPORTED et SIGABRT avant cleanup, privacy/cleanupPASS.
Pas de site/cause connus ; aucun artefact publié. La condition d'arrêt est atteinte :
pas d'extension des reconnaisseurs ni de répétition identique.

Diagnostic d'autorité effectué par lead : `gh repo view` confirme
`Djoe-Denne/IAMRusty` PUBLIC, isPrivatefalse. Les journaux Actions existants ne sont
pas un canal privé. WSL avait échoué sur --version faute de libnspr4, sans paquet
installé ; un Linux utilisateur a été proposé auparavant par l'utilisateur, mais
aucune session/hôte Linux privé prêt n'est prouvé disponible. Ne pas l'inventer.

Suivi demandé dans le même nouveau contrat uniquement : préciser l'option privée
minimale concrète et les véritables préconditions/autorités manquantes, avant toute
question humaine. Comparer un relevé local structuré, opérateur désigné, stockage
privé hors Git et rétention bornée, à une autre voie compatible sans nouveau service,
tunnel, clé/secrets CI ou publication de donnée inconnue. Ne pas proposer un faux
canal privé à partir de logs publics ou d'une simple empreinte/encodage. Vérifier si
le mandat délégué suffit pour les choix techniques ou si un accès/choix de
divulgation réellement indispensable manque ; ne pas demander un GO générique.

Garder le site inconnu privé : aucun corps, assertion/chemin absolu/PID/URL natif
exporté ; limiter précisément les champs collectés et les risques résiduels. Pas
de promesse de cause depuis un site rapporté et pas de nouvel essai de validation
tant que la voie et ses prérequis ne sont pas établis. Rendre une décision courte,
des options réalistes et le plus petit arbitrage nécessaire s'il subsiste.
Ce suivi n'autorise ni implémentation, runtime, package, CI, collecte, nouvelle
permission ou modification du contrat Accepted. Source et auteur13 gelés.
