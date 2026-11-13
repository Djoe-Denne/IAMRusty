---
title: Session CI, k6 Ubuntu et Allure — 12–13 novembre 2026
category: references
tags: [reference, testing, ops]
sources:
  - docs/history/ci-k6-reporting-20261113/README.md
  - docs/history/ci-k6-reporting-20261113/coverage.json
  - docs/history/ci-k6-reporting-20261113/agents.md
  - docs/missions/20261113-k6-auto-reporting-results.md
summary: >-
  Session et quatre sous-agents : pin Rust, gates CI, catalogue IAM/OAuth, packaging Ubuntu, diagnostic sandbox et E2E 34/34 avec Allure/Pages ; CI globale rouge sur une IT Manifesto.
created: 2026-11-13
updated: 2026-11-13
---

# De cargo fmt à une campagne Ubuntu publique

La session `ses_e2f50a8f4ffegiKcR00QyAu8Tb` démarre sur un décalage Rustfmt,
pas sur un projet de dashboard. L'ingestion couvre **cinq sessions**, dont
**quatre sous-agents directs**, sans descendant supplémentaire : 1 519 messages
natifs avant la demande d'ingestion et 287 textes utilisateur/assistant retenus.
Raisonnement interne et sorties d'outils exclus ; projections privées non publiées.

## Décisions intégrées

- **Même outil local et CI** : Rust 1.99.1 épinglé, lu par l'action commune ;
  ne pas confondre stable installé et stable courant. Le rendu Helm doit être
  validé comme rendu, pas parsé comme template YAML brut.
- **Connaissance métier indépendante** : pilote IAM/OAuth à quatre fonctionnalités
  et treize critères, assertions exactes et liens explicites ; voir
  [[projects/aiforall/concepts/functional-catalogue-and-test-evidence]].
- **Gates fidèles aux contrats** : exceptions lexicales fermées pour interfaces
  ratifiées, contre-exemples Unicode/appel ajoutés au raisonnement de revue ;
  pas de fichier exempté globalement ni promotion du mémo Proposed.
- **Ubuntu autonome** : binaires CI non instrumentés, migration IAM/OAuthConnect
  explicites, ABI et empreintes liées à run/attempt/SHA ; pas de second Cargo
  consumer. La fermeture `always()` complète le `finally` de packaging.
- **Preuve navigateur réelle** : diagnostics rouges conservés, AppArmor non
  accusé sans corrélation, sandbox kernel attesté puis parcours OAuth réel.
  [[projects/aiforall/concepts/k6-functional-linux]] distingue cette campagne
  de la slice Windows du 12 novembre.
- **Publication honnête** :
  [[projects/aiforall/concepts/e2e-allure-pages-reporting]] sépare 34 checks,
  contrat parent, code coverage et état global de la CI.

## Les quatre voix, sans écraser leurs limites

1. `ses_e2eeb63ddffeSSUgdV60NB2LQy` — architecte : six gates, exceptions
   contextuelles et deux contre-exemples ; mémo Proposed, pas décision produit.
2. `ses_e2bb54211ffetS1Myw6ZEFwhS5` — architecte : seam Ubuntu/ABI/provenance,
   absence de cleanup indépendant sur annulation détectée avant qualification.
3. `ses_e2ae5518fffec0q1hbdwyLsuvc` — emergency-thinker : diagnostic Chromium,
   angles morts journald et identité temporelle ; AppArmor reste une hypothèse.
4. `ses_e2b39f576ffe6sWFVATRaMqC3i` — infra-verifier repris : revues statiques,
   puis réception live de packaging, sandbox, campagne et Pages. Ne rejoue pas
   les 130 tests du parent et n'affirme pas une CI globale verte.

Détails des contributions et conclusions intermédiaires dans `agents.md`,
empreintes et borne dans `coverage.json` ; pas seulement les réponses finales.

## Résultat et limite ouverte

Campagne manuelle `43461703946` PASS ; automatique `43473245852` sur push/master
PASS E2E/Allure/Pages, source `741d0435`, SDK `93b890ff`. 34 checks et contrat
parent donnent **35 résultats Allure**, pas 35 critères fonctionnels. Les
14 fichiers de campagne sont bit-identiques ; cleanup possédé zéro actif.
Le premier run automatique `43470457080` était bloqué avant E2E par les alias
Clippy TOML, corrigé sans admettre flags LLVM/wrappers/env/target.

> [!warning] CI globale non qualifiée verte
> L'IT Manifesto `t6_post_get_components_contract_bytes_frozen` échoue dans
> `43473245852` (3 passes/1 échec dans son binary). Cause non diagnostiquée ici.
> Aucun nouveau 90 % Sonar, aucune qualification production ou promotion ADR.

Suite de [[projects/aiforall/references/session-iac-k6-qa-20261112]], rattachée
à [[projects/aiforall/aiforall]]. La session coverage voisine n'est pas un
descendant : ses 16 unités `--lib` et ses 22 binaries `--test` restent distincts.
