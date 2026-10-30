# Revue indépendante du canal de site privé — code intégré

## État de ce briefing

Le code est intégré dans `968592f2efec114d6f7c0c1c52c84fb07b505cab` ; cette revue
est préparée avant toute correction. Examiner le checkout principal ou ce commit,
pas le worktree auteur ancien. Aucune permission de collecte réelle ne découle
de cette revue ou du succès des tests. Le présent briefing sera intégré avant
l'admission du reviewer fresh READONLY.

## Objectif et canon

Mission1 seulement : un diagnostic Chromium local ne doit exposer aucun contenu
natif inconnu dans les sorties de l'agent, le chat ou le dépôt GitHub public.
Le lecteur désigné est l'utilisateur, localement dans son instance AIForAll.

Lire `docs/contracts/k6-chromium-causal-observation.md`, addendum du consommateur
local Accepted avant code dans ca0c994 ; l'ancre/projection publique reste Proposed.
ADR0504/V1 Accepted/Partial. Contrats et faits vérifiés sont fournis, pas les
conclusions de l'auteur. QA proportionnée : cette revue est bornée au canal,
aux permissions, aux refus et aux risques effectivement modifiés.

## Préconditions réelles

L'hôte dédié est livré sur I:, GUID22ce910b-45a3-42a4-bbab-dc56a1766035, compte
aiforall non-root, HOME privé sur ext4 et ACL Windows limitées. Son provisionnement
a été vérifié indépendamment. Il est actuellement arrêté, outils conservés.
Le navigateur local et un direct ont échoué, mais leurs enfants ont été nettoyés.
Aucun relevé privé ni couple source inconnu n'a encore été créé ou fourni à un agent.

La préparation de code et tests est admise. Toute collecte réelle attend cette
revue, les contrôles du parent et un créneau de lecture local effectivement convenu.
L'autorisation d'installation dans WSL ne déroge pas aux règles de confidentialité.

## Questions de sécurité à trancher

1. L'opt-in est-il local, explicite, désactivé par défaut et rejeté hors diagnostic
   nominal24, en comparaison22 ou avec marqueurs CI/Actions avant scrub et enfant ?
   L'absence de marqueur n'est pas présentée comme preuve d'autorité locale.
2. L'extraction exige-t-elle deux flux complets, UTF-8 strict, confidentialité et
   absence d'overflow sticky, un FATAL unique complet, identités attestées et cleanup
   réussi ? Les erreurs prioritaires ne sauvent-elles jamais un préfixe ?
3. Les seules données natives sont-elles chemin relatif et ligne, sous schéma
   fermé et bornes précises, sans normalisation ou ouverture du chemin source ?
   Inspecter ambiguïtés, faux formats et doubles enregistrements adversariaux.
4. La création protège-t-elle toute la chaîne de parents, les identités, liens,
   modes, propriétaire, hors Git et FS natif ? Les collisions sont refusées sans
   adoption ou chmod correctif ; primitives absentes refusées sans fallback.
5. Corps, couple inconnu, repr, exception, hash et chemin du relevé restent-ils
   absents de toutes les sorties, reçus, logs et publishers, même sur erreurs ?
   Les oracles négatifs vérifient-ils le câblage réel, pas seulement un helper ?
6. Le retrait ne vise-t-il que le relevé possédé avec identité vérifiée, après
   lecture ou au plus60min ? Examiner partiel, interruption, remplacement du fichier,
   retrait impossible et absence de lecture. Pas de daemon ou effacement promis.
7. Les résultats fonctionnels, exits non-zéro, pins, arguments, scénario et
   superviseur sont-ils conservés ? Le raccord au refus local ne fabrique-t-il
   aucune preuve d'autorité ni équivalence avec le reçu22 ou la panne GitHub ?

## Exécution de la revue

Fresh, READONLY, deux passes sécurité défensive et adversariale. Aucun runtime,
Chrome, k6, WSL, CI, Cargo ou collecte privée par le reviewer. Les fixtures de
tests sont connues ; ne pas ouvrir un hypothétique site.json réel. Un test ou un
flag n'est pas une autorisation de produire un relevé réel.

Rendre les findings bloquants séparés des suggestions non bloquantes et gaps hors
périmètre, avec fichiers/lignes, contre-exemple et correction attendue. Les
CRITICAL/HIGH bloquent. Ne pas demander un réaudit du dépôt ou un nouveau harness.
Le lead persistera le rapport avant correction puis ne fera reprendre que les
findings utiles ; les permissions et l'architecture restent aux contrats canoniques.

## Entrées intégrées et preuves factuelles

- Base du diff : `1c0036f4f2b7bb8d7f306ba3e1a1222787a686d1` ; source intégrée :
  `968592f2efec114d6f7c0c1c52c84fb07b505cab`.
- Quatre fichiers seulement : run-linux.py et README.md modifiés ; nouveaux
  lib/private_site.py et tests/test_private_site.py sous ops/tests/k6.
- Les18autres fichiers de base sont byte-identiques, dont Child complet, workflow,
  lock, scénarios et provisionneur. Le diff exact reste à examiner indépendamment,
  notamment les trois points d'entrée arguments/main/direct_chromium et le module.
- Lead Windows : `C:/Python313/python.exe -B -m unittest discover -s
  ops/tests/k6/tests -p test_*.py`, temp hôte approuvé ;162tests/0.945s/exit0,
  8skipsPOSIX,154exercés. Ces résultats ne prouvent pas les primitives Linux.
- `git diff --check` normal et staged exit0 ; commit source au hook normal réussi.
  Aucune capture ou lancement navigateur/k6/WSL/CI effectué pour ce delta.
- Le parent réel `parent_check` et le créneau de lecture ne sont pas fournis par
  les callbacks de fixtures. Ils restent indispensables avant activation réelle ;
  examiner si l'API conserve honnêtement cette frontière et les refus, sans
  supposer qu'une validation externe existe déjà.
- Les vérifications Linux de fixtures connues pourront avancer indépendamment
  de cette revue, sur ce même code gelé. Elles n'admettent aucune donnée native
  inconnue et ne constituent pas une capture Chromium ou un changement infra.
