# Revue indépendante — socle k6 Linux et CI minimale

## Mandat READONLY

Examiner le diff intégré, sans édition, test runtime, Docker/Kind/WSL, installation, Cargo ou commande vers une application. Deux passes ciblées défensive et adversariale selon le profil. Pas d'audit produit général, ni prise en charge de la coordination. Rendre constats avec fichier/ligne, scénario concret, contrat violé et preuve ; distinguer bloqueurs, suggestions et hors périmètre.

## Diff et entrées canoniques

- Base : 3f2e8fbe625b47fbc4093e673dc2804bcf9a3023.
- HEAD intégré : db16aa726ca0f44876180ac45b4922110c030c09, master.
- Périmètre : ops/tests/k6/** et .github/workflows/k6-functional-smoke.yml, quatorze nouveaux fichiers. Aucun changement Rust/service/identité/gitlink/CI existante dans ce commit.
- ADR0504 et docs/contracts/k6-functional-linux.md Accepted / Unimplemented pour V1 mission1, y compris l'addendum du trajet18080 protégé en source. Missions OAuth/droits/charge exclues.
- Policy approuvée révision1 : .opencode/missions/team_mvtz6p47_000q_4wmht0/model-policy.json ; QA proportionate dix commandes/seize critères. La policy prime sur les anciennes descriptions de préparation.
- Racine canonique C:/Users/djden/source/repos/AIForAll ; les outils MCP source ignorent les worktrees non intégrés, mais ces fichiers sont maintenant dans le checkout canonique au HEAD indiqué.

## Faits disponibles, sans conclusion auteur

- Commande lead réellement exécutée au checkout intégré : C:/Python313/python.exe -B -m unittest discover -s ops/tests/k6/tests -p test_*.py ; 37 tests, exit0,5 skips POSIX,32 exercés. Ce n'est pas la commande Linux et ce n'est pas une preuve HTTP/Chromium.
- git diff --cached --check exit0 et hook réel cargo fmt --all -- --check exit0 lors de la capture du commit intégré. Aucun résultat k6 version/inspect/run ou Chromium --version/browser acquis.
- Aucun runtime ou ressource Docker créé/redémarré par la mission. Les fakes locaux de commandes et résumés ne prouvent pas un endpoint réel.
- La CI est nommée prérequis, exécute provision-linux.py puis outils/browser et publie quatre fichiers explicites. Aucun run GitHub observé.
- Le lock contient k6v2.3.0 et un candidat Chrome for Testing156.0.8078.73. La métadonnée officielle versionnée https://googlechromelabs.github.io/chrome-for-testing/156.0.8078.73.json confirme version, révision1706539 et URL linux64. Les empreintes du navigateur restent à examiner indépendamment avant qualification ; aucune exécution d'un binaire acquise par cette métadonnée.
- Le code source Envoy conserve ext_authz sur /iam/, le backend refuse sans Bearer ; pas de403 live mesuré, cible Linux publique non établie. Aucun affaiblissement d'auth ni nouvelle cible libre autorisé pour obtenir un vert.

## Risques modifiés à examiner

1. Sélection cible, schéma/configuration, overrides et environnement/proxy, ordre des gardes et absence d'appel interdit.
2. Verdicts observables : exits, complétude, fraîcheur, métriques manquantes, interruption et chemins d'erreur. Relier aussi les tests aux risques, sans introduire une infrastructure de mocks HTTP.
3. Confidentialité : stdout/stderr natifs, versions/inspect, résumés, sentinelles, artefacts, limites et absence de publication avant contrôle.
4. Possession et fermeture des enfants : identité PID/UID/début de vie, descendants/reparenting, signaux, processus préexistants et preuve effective de sandbox. Pas d'arrêt d'un processus inconnu.
5. Acquisition/provenance/archives et pins réellement utilisés : extraction, chemins, modes, charge mémoire/sorties et séparation acquisition/exécution.
6. Permissions/événements/actions GitHub, code PR non fiable, secrets, artefacts et distinction prérequis/E2E.

## Sortie attendue

Rapport borné avec CRITICAL/HIGH bloquants selon profil, suggestions séparées et limites non exécutées. Le lead conserve l'auteur pour les corrections et persiste les findings avant reprise. Ne pas passer en APPLY SECURITY FIXES et ne pas modifier le ledger partagé. Ne pas confondre absence de cible ou commande non exécutée avec test ROUGE produit.
