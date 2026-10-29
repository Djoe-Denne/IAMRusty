# Diagnostic causal Chromium admis, sans changer la qualification

## Objectif et décision

L'architecte confirme qu'un lancement diagnostique séparé du même Chromium épinglé reste dans la responsabilité du parent. Le lead ratifie cet addendum avant code : il ne crée ni second framework de tests, ni nouvelle preuve de réussite navigateur.

L'expérience recherchera l'erreur native que k6 peut filtrer. Elle sera corrélée au cas k6 sur le même runner, le même UID et les mêmes octets vérifiés. Le profil sera neuf et privé, la navigation limitée à about:blank ; arguments fixes, environnement fermé et superviseur existant. Les sorties resteront privées et bornées ; seuls des codes fermés seront visibles.

## Invariants

Ce diagnostic ne produit jamais HTTP PASS ou Chromium PASS, même en cas d'exit0 natif. Il garde distincts message rapporté, politique configurée et cause démontrée. Les lectures éventuelles de métadonnées hôte sont précises et en lecture seule. Aucun sudo, unshare, journal hôte, profil AppArmor, helper SUID, paquet, modification sysctl/seccomp, wrapper, debug ou déploiement.

## Mise en œuvre prévue

Un seul propriétaire outillage pour lancer, tests et éventuel raccord CI couplés. Réutiliser les pins/identités, Child et environnement existants ; aucun fallback automatique du mode browser. L'appel direct sera explicite et au plus une fois dans l'expérience comparative. Les recherches précédentes restent tracées au rapport18 ; aucun nouvel essai n'a encore été lancé.

## Preuves et limites

- Addendum architecte dans docs/contracts/k6-functional-linux.md, Accepted après ratification lead le2026-10-30 ; ADR0504 et V1 restent Accepted/Partial.
- Candidate `result_mvu5exmt_0002_s0lf6k`, session `ses_e761c08c9fferiab1yLWGU5Ka6`, un fichier/+74lignes avant admission, diff-check auteur PASS.
- Récupération exacte puis sauvegarde SHA256 vérifiée `.tmp/k6-diagnostic-contract-88a0e61702264d5d95ae275c1332ae13.md`, aucune source applicative touchée.
- Cause exacte encore inconnue, navigateur et HTTP non qualifiés ; pas d'installation ou relaxation implicite parmi les solutions trouvées sur Internet.
