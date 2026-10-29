# Revue sécurité indépendante — socle k6 Linux intégré

## Verdict et périmètre

PASS statique ciblé, aucun finding de sécurité démontré sur 3f2e8fb..db16aa7. Ce résultat ne qualifie ni HTTP, ni Chromium/Linux, ni sandbox/cleanup réels. Revue fresh READONLY de quatorze nouveaux fichiers ops/tests/k6/** et .github/workflows/k6-functional-smoke.yml, contrat Accepted/Unimplemented et briefing lead. Aucun delta local sur ces chemins au contrôle final.

Source du rapport : security-reviewer pool-5, session ses_e767aa95effe5s3xLdh1pnBgQs, candidat result_mvu1wlvj_001j_r2dtzt/message msg_1898c2068001D1R7ajKJeydY2u, reçu et relu par le lead. Aucun correctif demandé.

## Frontières examinées

Entrées de campagne et environnement ambiant vers parent ; ancre de cible vers enfant k6 ; archives officielles vers binaires exécutables ; stdout/stderr et métriques vers projections publiées ; code PR non fiable vers runner GitHub. Actifs à préserver : cible non détournée, credentials/URL sensibles non publiés, processus étrangers non arrêtés, absence de faux PASS. Aucun audit général du produit ni accès runtime autorisé à ce reviewer.

## Passe défensive

- run-linux.py:26–74 : schéma fermé, origine fixe, ancre historique réelle suivie d'un refus HTTP inconditionnel ; le manifeste n'autorise pas la cible.
- safety.py:116–123 et run-linux.py:160–183 : environnement enfant minimal sans K6/proxy/loader ambiants, config et sorties imposées.
- iam-health.js:24–27 : méthode/route fixes, timeout, aucune redirection suivie.
- run-linux.py:77–104,186–220 : exits, résumé courant, types/checks/cardinalités exacts, absence de métrique non assimilée à zéro.
- safety.py:199–363 : identités des descendants, TERM gracieux, survivants contrôlés ; témoin sandbox exige namespaces, seccomp supplémentaire et NoNewPrivs.
- run-linux.py:226–244,402–418 : projections contrôlées avant écriture, diagnostics fermés et pas de flux natif publié.
- Workflow:14–67 : permissions minimales, actions SHA, credentials non persistés, aucun secret/cible applicative et quatre artefacts explicites.

## Passe adversariale

Aucun contournement démontré pour origine hostile avec proxy ambiant ; ancien bail et observations auto-attestées ; résumé périmé avec checks verts ; erreur native contenant une sentinelle ; PR non fiable recherchant cible privilégiée ou publication assimilée à une preuve HTTP.

L'assainissement de l'enfant k6 ne qualifie pas globalement l'environnement de l'autorité runtime historique. Toute future activation HTTP exige une revue du raccordement et de ses preuves ; ne pas lever le refus par simple déclaration.

## Provenance et attribution

Le reviewer a acquis indépendamment en mémoire l'archive officielle CfT156.0.8078.73 et son ELF, recoupé la métadonnée versionnée/r1706539 et confirmé les empreintes du lock sans extraction disque/exécution ; texte de checksums k6 également vérifié.

Complément factuel attribué au lead, non transformé en exécution : acquisition indépendante en mémoire des deux archives/ELF, quatre empreintes concordantes, HTTPS officiel, hôtes finaux fermés et absence de proxy hérité. Cela prouve concordance des octets/provenance d'acquisition, pas les versions renvoyées par binaires, compatibilité ou sandbox.

## Vérifications et limites

Reviewer : git diff --check exit0 avec avertissement CRLF sur mémoire Serena étrangère ; diff-check du commit intégré exit0. Preuve Windows du lead réutilisée : 37 tests, exit0,5 skips POSIX. Aucune réexécution redondante.

Non exécutés par le reviewer : python3 unittest Linux, k6 version, Chromium --version, deux k6 inspect, validate/run/browser/selftest. Tests négatifs lus utilisant notamment des mocks, aucune preuve live déduite. Aucun runtime, ressource, branche/worktree créé ou source modifiée.

Bloquants démontrés : aucun. Suggestions non bloquantes : aucune. Restent les qualifications Linux/outils/Chromium/cleanup et, pour HTTP, une exposition publique existante autorisée, un amendement ratifié et les négatifs réels après admission. Aucun PASS global.
