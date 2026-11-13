# Contributions des quatre sous-agents

Inventaire natif récursif, sessions poursuivies sous le même identifiant.
Les constats intermédiaires restent datés par leur contexte ; une revue statique
n'est pas rétroactivement convertie en preuve live.

## Architecte — gates runtime

`ses_e2eeb63ddffeSSUgdV60NB2LQy` — 46 messages, 8 textes visibles.

Réconcilie les six gates avec P4-core/ADR-0012 et les interfaces ratifiées.
L'ancien scan de sous-chaînes attrape `release` comme `lease` et des types/ports
de contrôle légitimes. Préconise des exceptions localisées, sans déplacer
l'exécution produit dans Manifesto. Revue de la traduction : 19 règles/20 cas,
mais deux trous concrets, Unicode (`KubernetesClaimsΩ`) et invocation directe
après un fragment de champ autorisé. Contre-exemples négatifs exigés ; le mémo
reste Proposed, aucune décision Accepted créée. Travail final RO, pas Cargo.

## Architecte — E2E Ubuntu et seam de packaging

`ses_e2bb54211ffetS1Myw6ZEFwhS5` — 62 messages, 6 textes visibles.

Définit la frontière binaires existants → bundles → packaging → campagne :
Ubuntu 24.04/ABI cohérente, IAM migration/OAuthConnect explicites, flags et
provenance contrôlés. H et T séparés en mode manuel, aucune reconstruction
consumer. Détecte l'absence de garde de fermeture indépendante sur annulation :
un `finally` seul ne suffit pas. Demande `always()` par reçus/IDs et inventaire
final. Conformité de packaging ne prouve pas encore E2E ; les résultats réels
ultérieurs sont portés par le parent et le vérificateur indépendant.

## Emergency-thinker — diagnostic Chromium

`ses_e2ae5518fffec0q1hbdwyLsuvc` — 68 messages, 10 textes visibles.

Intervient sur le blocage de démarrage, sans créer d'enfant. Diagnostic fermé
avant toute correction de politique : permissions des exécutables, dépendances,
Crashpad, stderr expurgé et `NO_USABLE_SANDBOX`. Refuse l'assimilation automatique
à AppArmor. Met en évidence trois angles morts de l'audit : timestamp de
réception vs événement, filtrage kernel excluant audit et absence silencieuse
d'outils. Demande compteurs bornés, états de capture explicites, identité
boot/PID/UID/start et données kernel ; horodatage source seul reste diagnostic.
La proposition finale ne revendique aucune cause AppArmor démontrée et n'autorise
aucune exception globale. Le profil seccomp final et les campagnes réussies sont
des étapes postérieures du parent, pas une exécution de ce sous-agent.

## Infra-verifier — réceptions successives

`ses_e2b39f576ffe6sWFVATRaMqC3i` — 67 messages, 58 textes visibles.

Revue indépendante du packaging distant, des essais Chromium puis des campagnes
manuelle et automatique. Conserve les distinctions : ABI seulement, diagnostic
sandbox seulement, puis vrai parcours OAuth/SAN/health. Examine sources exactes,
API GitHub, artefacts et receipts ; les tests locaux du parent ne sont pas rejoués.

Réception auto `43473245852` : E2E après dernier build et parallèlement aux
couvertures ; H=T=packaging `741d0435`, SDK `93b890ff`, bundles même run/attempt.
Trois probes fermées, lifecycle navigateur/nœud/volume complet, owned_running 0.
Quatre witnesses renderer UID 1001, cinq processus exit 0/cleanup, 34 checks.
Campagne locale/publiée : 14/14 SHA256 égaux. Bundle public : 111 entrées,
35 test-result, zéro attachment ; scan borné JWT/bearer/ghp_/sentinelles sans
détection. Status live identique au bundle, badge vert 34/34 et Allure 35 passes.

Limites conservées : garde Pages attestée indirectement par le bon artefact
déployé et par revue statique, pas par ses logs ; scan de confidentialité borné,
pas preuve universelle ; 130 tests locaux rapportés par le parent, non rejoués ;
CI globale rouge sur Manifesto, non diagnostiquée ; aucun 90 % Sonar annoncé.
