# Revue indépendante ciblée — consommateur privé du refus k6

## Mandat fresh READONLY

Source exacte `d47b80508f71cfffbbd5d6caf2146f949997a954..0601f6f73a93a2fedb9b8717450d06acd6a5fa67`, limitée aux sept fichiers runner/README/workflow et tests direct/comparaison/provisionnement/nouveau test_direct_followup.py. Les commits documentaires intermédiaires sont le contexte, pas du code à réauditer. Aucun changement du lock, scénario, provisionneur ou safety.py ; préfixe job24 strictement inchangé.

Risque distinct justifiant cette revue : le parent consomme désormais son résultat privé de comparaison pour autoriser un diagnostic direct séparé22. Vérifier les gardes de chemin/identité/mode/taille, schéma fermé/fraîcheur/corrélation et anti-répétition, confidentialité et verdicts. Deux passes bornées, pas réaudit global de safety.py ou des campagnes antérieures.

## Canon et faits vérifiés

ADR0504/V1 Accepted/Partial ; comparaison22 seule Accepted avant code dans b316, direct22 facultatif séparé déjà admis après échec browser. Le lead le retient après run40976916763, recherches tracées rapport27 et briefing d'implémentation166c235 avant reprise de la même session auteur. AppArmor reste Proposed/non autorisé. Aucun nouveau harness, privilège, paquet, cible HTTP ou qualification admis.

Le run historique40976916763 a109tests Linux OK sur22/24, prérequis PASS, k6 pré-cas sur22 malgré restriction0, cleanup enfants PASS. Il ne valide pas ce nouveau consommateur. Le prochain essai doit différencier l'échec natif Chromium22 de celui de k6, pas fabriquer un PASS ni prouver AppArmor par un seul réglage.

Lead : sept fichiers récupérés par patch et byte-identiques au candidat. Replay Windows Python3.13 natif118tests/0.638s/exit0/skips7 (111exercés), temp hôte approuvé. Diff-check normal/staged et vrai hook fmt exit0. Aucun nouveau test Linux/native/CI de ce delta. Les fichiers de test non suivis dans l'ancien arbre ont été récupérés explicitement, pas supprimés selon un diff trompeur contre son HEADb316.

## Contrôles attendus

- Direct22 avant tout enfant de versions ou navigateur : refusal privé0400/parent privé hors Git, bornes et JSON strict existants ; mêmes métadonnées/UID/versions/ELF, timestamps/durée120s, résultat incomplet/courant de k6 avec outils/cleanup/confidentialité PASS et SANDBOX_UNPROVEN. Pas de confiance dans un manifeste comme propriété runtime.
- Refus des entrées absentes/tronquées/hostiles/périmées ou contexte différent, privacy/cleanup/prérequis échoués et cas complet. Marqueur exclusif empêchant une seconde tentative, y compris après échec ultérieur ; pas de substitution par symlink/chemin ou arrêt étranger permis.
- Même pin/profil neuf/argv/env/Child/sandbox/privacy/cleanup pour le direct ; exit natif0 jamais qualifiant, aucune publication de fichier ou flux natif. La table de codes k6v2.3.0 reste fermée et distincte des codes Chromium.
- Signal privé émis seulement après recontrôle des flux complets, résumé actuel et résultat admissible ; aucun PASS de confidentialité depuis un préfixe tronqué. Ce fichier privé de coordination n'est pas un artefact publié ni un nouveau bail.
- CI22 : un browser puis au plus un direct séparé après échec réel du step, autorisé par le parent ; aucune permission/secret/installation/dispatch/fallback/retry ou upload d'échec ajouté. Le job24 est inchangé et son échec reste rouge.

## Retour

Findings démontrés par scénario et lignes, CRITICAL/HIGH bloquants selon le profil ; suggestions/nonblocking et gaps de test distincts. Ne pas déduire Linux/Chromium PASS des simulations ou relancer les118tests uniquement pour remplir le rapport. Pas de correction, runtime, Cargo, téléchargement, CI, délégation ou ledger partagé. Rendre Localisation/Impact/Limites et contrôles réellement faits, sans conclusions d'auteur prises comme oracle.
