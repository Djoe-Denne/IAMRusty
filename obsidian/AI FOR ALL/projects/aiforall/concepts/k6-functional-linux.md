---
title: "k6 — socle Windows qualifié, Linux en attente"
category: concepts
tags: [architecture, security, visibility/internal]
sources:
  - docs/adr/0504-validation-fonctionnelle-linux-k6.md
  - docs/contracts/k6-functional-linux.md
  - docs/missions/20261029-k6-recherche-et-enchainement.md
  - docs/contracts/k6-windows-functional.md
  - docs/contracts/k6-windows-browser-version.md
  - docs/contracts/k6-windows-code-custody.md
  - docs/missions/20261029-k6-01-socle-smoke.md
  - ops/tests/k6/WINDOWS.md
  - .opencode/missions/team_mvtz6p47_000q_4wmht0/reports/99-cas-minimal-windows-verifie.md
  - .opencode/missions/team_mvtz6p47_000q_4wmht0/reports/107-nettoyage-et-archivage-verifies.md
  - docs/archives/k6-wsl-20261030/README.md
summary: "Cas minimal k6/Chrome Windows PASS et 34 tests ; HTTP et Linux non qualifiés, WSL suspendu. Version via API et nettoyage ne valent pas attestation du sandbox."
provenance:
  extracted: 0.95
  inferred: 0.05
  ambiguous: 0.0
created: 2026-10-29
updated: 2026-10-31
---

# k6 : un socle Windows qualifié, pas une preuve globale Linux

## État vérifié au 31 octobre 2026

L'état initial « Proposed / Unimplemented, aucune exécution » est dépassé. ADR 0504
et le contrat Linux V1 sont **Accepted / Partial** ; ils ne sont pas qualifiés par
un résultat Windows. Le raccord fonctionnel Windows est **Accepted / Partial**,
avec un cas minimal navigateur **PASS**. Son contrat de version effective est
**Accepted / Implemented et vérifié**. Le wiki photographie ces décisions sans
changer leur statut canonique.

Le vrai cas `about:blank` vérifie `6 * 7 === 42` et la version Chrome for Testing
**156.0.8078.73** via l'API dans le même cas ; k6 annonce **2.3.0**. Le résumé strict
contient un cas attendu, démarré et achevé, une itération, zéro requête k6/http et
trois checks nommés, chacun avec une réussite et aucun échec. Exit 0, outillage,
Chromium fonctionnel, nettoyage et confidentialité sont PASS. Le dossier du run est
retiré ; aucun processus k6/Chrome n'est observé avant ou après. Source : rapport99.

Le lead a aussi exécuté **34 tests Windows, 42.651 s, zéro skip**. Le mode tools
n'a pas été relancé séparément : version k6 et deux inspections étaient déjà
exercées dans le mode browser. Chrome `--version` reste NOT_EXECUTED/INCONCLUSIVE ;
la preuve ratifiée est le check API obligatoire, pas un autre lancement.

**HTTP et ses selftests restent INCONCLUSIVE**, faute de cible Windows existante
admise. Aucun endpoint factice, OAuth/authz, seed, POST, charge, déploiement ou tunnel
n'est ajouté. **WSL est suspendu ; Linux reste INCONCLUSIVE.** Les deux jobs Linux
sont littéralement désactivés dans un workflow SUSPENDED. Ni un job ignoré ni ce
PASS Windows ne qualifie Linux, IAM ou le sandbox.

## Ce qui a été corrigé — et ce qui a été abandonné

Deux défauts réellement rencontrés ont conduit à des corrections ciblées : le
préfixe natif `k6.exe` dans la sortie de version, puis le tag système `group`
nécessaire au rapport détaillé des checks de k6 2.3.0. La commande Windows conserve
`name,check,scenario,group`, sans changer le JavaScript partagé, HTTP ou `summary.js`.
Un objet de checks vide est rejeté par SCHEMA_INVALID ; il ne devient jamais PASS.

Les diagnostics d'échec parent restent privés et bornés. Le reporting est ainsi
une partie de l'oracle fonctionnel, pas seulement une présentation du résultat. ^[inferred]

La révision3 Accepted retire les verrous personnalisés exécutables/DLL, contrôles
ACL et attestations renderer/token/sandbox. La proposition code-custody est
**Abandoned / Superseded, jamais ratifiée**, et ne bloque plus le lancement. Les
protections natives Chromium, pins/provenance/SHA256, profil séparé, timeout et
nettoyage raisonnable des processus créés restent conservés. Ce choix ne transforme
pas un PASS fonctionnel en certification du sandbox.

## Frontières à préserver

`k6/http` porte les scénarios API ; `k6/browser` utilise Chromium réel pour cookies et redirections OAuth. Une succession de requêtes HTTP n'est pas une preuve navigateur. La campagne réutilise les oracles de [[projects/aiforall/references/auth-security-closure-20261029]] et les routes exactes de [[projects/iamrusty/decisions/0412-oauth-transaction-browser-binding]], sans changer l'autorité pour faciliter le runner.

Le parent opérations conserve le contrôle du laboratoire, des perturbations, observations et du nettoyage. Pas d'API administrative générique ajoutée pour k6. Setup/teardown ne garantissent pas le nettoyage après interruption. Les contrôles fonctionnels doivent avoir des thresholds qui rendent l'exécution fautive en cas d'échec, avec preuve de complétude et de non-vacuité ; les checks seuls ne suffisent pas.

## Trajectoire Linux et authentification

Les trois ordres initiaux restent distincts : socle et smoke, portage des preuves
d'authentification, puis propagation/reprise des droits Linux. Seul le cas minimal
Windows décrit ici est qualifié ; le portage OAuth et les campagnes de droits ne
sont pas admis par cette ingestion. Les tests Rust restent natifs Windows selon la
stratégie du dépôt. OHA, performance, UI, HA exhaustive et Grafana Cloud restent exclus.

Les 24 sources de l'enquête WSL sont des archives inactives, dont cinq sources
`ldd-only` tardives non exécutées par le lead. Leur préservation ne relance ni sonde
ni seconde mesure. Le nombre de dépendances manquantes du run historique reste
inconnu ; la qualification Linux sera une mission distincte demandée par l'utilisateur.

La dette matérielle HSM reste acceptée et non bloquante pour ces campagnes, mais n'est pas qualifiée ; ADR 0309 reste Partial. Les résultats attendus ne deviennent pas des résultats observés par leur inscription dans le wiki.

## Clôture et conservation

Les dix copies temporaires sont retirées après sauvegarde/restauration vérifiées :
79 fichiers working et 141 versions/restaurations, plus quatre patchs publics.
L'équipe est archivée, ses 23 membres arrêtés ; le ticket Linux restant est annulé
sans publication ni exécution. Les onze références de sauvegarde supprimées par
le nettoyage natif ont été restaurées aux noms et commits exacts. Cette clôture
administrative n'ajoute aucune preuve fonctionnelle. Sources : rapport107 et
[[projects/aiforall/concepts/orchestrator-agent-harness]].

Le code fonctionnel est publié dans `201ec05b`, les archives tardives dans `a24a7ce5`
et le bilan de clôture dans `410d8168`, sur master, référence distante vérifiée.
Les données WSL, outils, sous-module et modifications étrangères sont conservés.

Références canoniques : contrats Windows/Linux, guide `ops/tests/k6/WINDOWS.md`,
ordre mission1 et rapports99/107 cités en frontmatter. Projet : [[projects/aiforall/aiforall]].
