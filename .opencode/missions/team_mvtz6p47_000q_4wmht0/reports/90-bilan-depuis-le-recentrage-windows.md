# Bilan depuis le recentrage Windows

L'objectif est de vérifier k6 et l'unique cas Chrome minimal sous Windows natif.
**Le socle et ses protections ont avancé ; aucun k6 ou Chrome Windows réel n'a
encore été exécuté. La qualification Windows n'est donc pas acquise.**

## Avancées vérifiées

- Lanceur PowerShell5.1 Windows intégré, avec pins Windows distincts pour k6 2.3.0
  et Chrome for Testing156.0.8078.73. Archives officielles acquises, empreintes et
  exécutables64bits vérifiés, copies privées préparées sans installation globale.
- Première campagne lead :21tests réussis sansskip ; après correctifs,22tests
  ciblés réussis sansskip. Ce sont des campagnes de portée différente, pas43tests
  uniques ni une qualification des vrais binaires.
- Risque de nettoyage d'un dossier concurrent corrigé et fermé par relecture.
  Contrôle individuel du code Chrome extrait contre l'archive corrigé et fermé
  au préflight. Les vrais fichiers passent ce contrôle statique sans être lancés.
- Vérification de version Chrome raccordée à l'API normale k6 dans le même cas,
  avec tests négatifs ; Chrome--version n'est pas essayé sur une hypothèsePOSIX.
- TravailWSL suspendu, données et outils conservés,19sources utiles archivées
  inactives. Suspension des deuxjobsLinux intégrée localement avant publication ;
  aucun effet distant ou résultatCI nouveau n'est prétendu.

## Points encore non résolus

1. **Blocage du lancement : protection du code après défaillance du lanceur.**
   Si un processus survit, les verrous du lanceur ne suffisent plus après sa fin.
   Une protection persistante par permissions dans la frontière de confiance
   initiale est en formalisation/vérification. Elle n'est pas encore ratifiée,
   prouvée et fermée en revue ; le gel reste actif.
2. **Limite de qualification : isolation effective Chromium non prouvée.**
   Le témoin primaire est insuffisant. Même un futur JS/version réussi conservera
   ChromiumINCONCLUSIVE dans ce cadre. Aucune évasion ou désactivation du sandbox
   n'est démontrée ; aucune instrumentation ou élévation nouvelle n'est ajoutée.
3. **HTTP conditionnel : aucune cible existante autorisée n'est encore admise.**
   GET et selftests réels restent non exécutés, sans serveur factice ou déploiement.

Publication, retrait des arbres temporaires et archivage final ne sont pas faits.
L'inventaire exhaustif d'un premier essai ancien de fixtures reste non prouvé ;
cela ne démontre pas une fuite actuelle, ni ne devient un PASS par les campagnes
ultérieures terminées. Les sources et ressources étrangères restent préservées.

## Suite dans le mandat

Fermer précisément la protection persistante et sa preuve ciblée, puis reprendre
les seules commandes réelles admissibles : versions/inspections et cas navigateur
unique, avec leurs verdicts séparés. Pas de nouveau GO de jalon demandé, de reprise
WSL, de runtime tiers ou de qualification globale anticipée.

Références : [tests et préflight](87-correctifs-integres-et-22-tests-cibles-reussis.md),
[relecture complète](88-relecture-complete-une-frontiere-restante.md),
[piste persistante](89-protection-persistante-piste-a-verifier.md). Preuves existantes
réutilisées pour ce bilan, aucun contrôle ou test relancé.
