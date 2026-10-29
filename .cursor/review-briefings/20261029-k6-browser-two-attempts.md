# Diagnostic spécialisé après deux essais Chromium non concluants

## Objectif, autorité et périmètre

Feuille emergency-thinker READONLY, pas coordination ni sous-agent. Mission1 k6 uniquement : outils/Linux et navigateur sandboxé about:blank, aucune application/OAuth. Conserver pins k6v2.3.0/CfT156.0.8078.73, contraintes sandbox/confidentialité et contrat Accepted/Partial. Pas de source edit, runtime, CI dispatch/retry, paquet, wrapper, debug CDP, sysctl/AppArmor, élargissement permissions ou nouveau harness. Le lead garde publication, baux et autorité.

## Deux essais déjà effectués

1. Run natif GitHub `40963859987`, commit919f1bd, source5d79557 : tests/acquisition/versions-inspect success ; browser exit2 SANDBOX_UNPROVEN, cleanupPASS, zéro artefact publié.
2. Après correctif de diagnostic fermé, run `40965632254`, commit4e82a36 : mêmes prérequis success ; browser exit2 SANDBOX_UNPROVEN avec browser_native_diagnostic NATIVE_CAUSE_UNCLASSIFIED, cleanupPASS, uploadskipped. Le signal nouveau n'identifie pas une cause. Pas de troisième essai autorisé sans hypothèse différente étayée et expérience discriminante bornée.

Échec WSL distinct : Chrome --version exit127, libnspr4.so absente. Aucun navigateur local qualifié, aucun paquet installé. WSL ARTIST-Ubuntu actuellement arrêté, bail local libéré ; Docker Desktop/son conteneur local préexistants étrangers et préservés. Ne pas réveiller runtime pour cette analyse.

## Sources et preuves disponibles

- HEAD canonique/publé4e82a365077c505bb6681bbbb3018f3869f995b7, source ops/tests/k6/**. ADR0504 et docs/contracts/k6-functional-linux.md Accepted/Partial. Reports12–14 et ledger `.cursor/handoffs/20261029-k6-01-socle.md`.
- `.github/workflows/k6-functional-smoke.yml` utilise ubuntu24.04 natif, actionsSHA, contentsread, même lanceur, pins revalidés, acquisition explicite ; pas de secret/application.
- `Child.execute` lit /proc, exige témoin externe renderer : UID courant, userns/pidns distincts, Seccomp2, NoNewPrivs1, filtres supplémentaires. Aucun témoin n'est déclaré acquis par l'exitk6.
- Sources officielles k6 v2.3.0 : `internal/js/modules/k6/browser/chromium/browser_type.go` flags439–478 sans no-sandbox ; `common/browser_process.go` parse206–278 peut ignorer FATAL et remplacer la cause par une erreur générique.
- Classifieur safety.py browser_diagnostic : code inconnu pour message non reconnu/nonzero ; pas attribution AppArmor. Il ne publie aucun natif. La confidentialité pourrait retenir certains messages mais le verdict observé n'est pas PRIVACY_FAILED.
- Test56 Windows lead :49exercés/7skips, exit0 ; étape testsLinux native du dernier run success. Ancien replay50Linux complet pour5d79557. Aucun de ces tests ne qualifie Chromium.
- S-1HIGH (substitution de chemins) corrigé et revue indépendantePASS, voir briefing20261029T2206Z ; ce n'est pas le blocage navigateur courant.

## Questions techniques bornées

Examiner les faits accessibles (sources officielles/pins, scénario, options et mécanisme d'observation, logs CI déjà publics filtrés via ContextMode). Distinguer défaut du scénario/lanceur, incapacité de démarrage et impossibilité de prouver le confinement. Ne pas conclure sur une simple absence de témoin ou une doc générale d'Ubuntu.

Proposer soit une cause étayée, soit une expérience réellement discriminante qui reste dans les gardes existantes, avec ce qu'elle permettrait de conclure ; sinon nommer le plus petit prérequis/autorité manquant. Aucun payload hostile ou relâchement pour obtenir vert. Le résultat attendu est une analyse avec preuves/limites et budget de tentatives, pas une reprise de mission. Rendre un rapport complet au lead, ne pas écrire le ledger partagé.
