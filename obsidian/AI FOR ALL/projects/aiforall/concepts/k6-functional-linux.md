---
title: >-
  k6 — trois scénarios Windows qualifiés, Linux différé
category: concepts
tags: [architecture, security, visibility/internal]
sources:
  - docs/missions/20261112-k6-three-native-results.md
  - docs/missions/20261112-python-dead-code-cleanup.md
  - docs/history/iac-k6-qa-20261112/README.md
  - .agents/skills/aiforall-k6/SKILL.md
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
  - docs/missions/20261031-k6-auth-handover.md
summary: >-
  Santé HTTPS, OAuth Chromium et SAN qualifiés sur une slice native isolée au commit54a19ba. Fixtures IaC et preuves parent ; Linux/CI et retrait legacy différés.
provenance:
  extracted: 0.96
  inferred: 0.04
  ambiguous: 0.0
created: 2026-10-29
updated: 2026-11-12
---

# k6 : trois scénarios Windows, pas une preuve globale Linux

## État courant — 12 novembre 2026

La campagne `ops/tests/k6/native/campaign.ps1` a réellement passé au commit
**54a19ba**, run `e3337dff24eb470881233fdfad6f6022` : santé HTTPS, SAN
303→503→303 et OAuth Chromium26, **34 checks réussis**. Le parent corrèle
IAM+7/ext-authz+4/fournisseur+3/PKCE+3, users0→1/outbox0→0, même Envoy et
restauration vérifiée. Cleanup18/18, DB dédiée et enfants/forwards fermés ;
lab13/13 disponible laissé en marche sur accord utilisateur.

La voie est **IaC/fixture-first** : slice IAM/OAuthConnect v2/Envoy/IdP déclarée,
inputs privés hors Git, données créées par les vraies API. Postgres/ext-authz et
autres backings restent partagés, donc pas une pile hermétique. Procédure :
[[projects/aiforall/skills/testing-k6-fixture-first]].

Le prérequis autonome `about:blank` et ses modes browser ont été retirés le
11 novembre ; le contrôle de version reste dans le vrai parcours OAuth. La CA
publique durable CurrentUser\Root est installée séparément, pas retirée au cleanup.
La tranche IAM stateless à12checks est un autre chemin, pas la parité OAuth/SAN.

Le nettoyage **fc5e29b** ne retire qu'une constante du simulateur : régressions
195 k6 PASS/8 POSIX skips +9 IdP PASS, **pas de nouvel E2E**. Les skills/profils
suivants n'étendent pas les empreintes de la preuve54a19ba. Linux, CI GitHub,
password/refresh positif et retrait du legacy restent différés ; contrat de
parité native **Proposed**. Provenance :
[[projects/aiforall/references/session-iac-k6-qa-20261112]].

> [!note] Historique conservé
> Les sections datées d'octobre ci-dessous décrivent les résultats et blocages
> d'alors, pas des commandes actuelles ni le verdict de la campagne du12novembre.

## Historique — état vérifié au 31 octobre 2026

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

## Historique au 31 octobre — auth livrée, campagne non exécutée

La mission suivante est **arrêtée à la demande de l'utilisateur**, sans PASS auth
réel. Les scénarios `oauth-browser.js` et `envoy-san.js`, ainsi que les modes
`auth-inspect` et `auth` du parent Windows, sont intégrés. L'inspection native
passe ; les revues statiques sont achevées et le défaut de restauration avant
readiness est corrigé. Ni cette inspection ni ces revues ne prouvent l'exécution
du parcours navigateur, le rejet du mauvais certificat ou sa restauration réelle.

Le bootstrap du profil local-full a réussi, mais la commande de déploiement a
échoué. **La cause initiale reste inconnue** : le wrapper opérateur PowerShell a
perdu le détail de l'erreur Python en interrompant sa capture sur stderr. Après
reprise en lecture seule, les deux Jobs de bootstrap sont réussis et IAM/Lazaret
sont observés Ready ; le déploiement complet et le trajet Windows→18080 ne sont
pas qualifiés. Ce constat n'établit pas que le pipeline existant soit cassé.

Les 17 IT historiques restent réutilisées sur sources pertinentes inchangées,
sans réexécution ni réécriture en k6. Leur preuve propre dans
[[projects/aiforall/references/auth-security-closure-20261029]] est conservée.

Tous les agents et les trois nœuds possédés sont arrêtés, sans processus
k6/Chrome restant. Les volumes, diagnostics et arbres auteurs sont conservés ;
l'archivage natif de cette mission est différé. Il ne faut pas confondre cet
arrêt avec le nettoyage complet de la mission1 décrit plus bas. Le handover est
publié avec les livrables sur master (`21c61c1`), sans nouveau verdict fonctionnel.

L'attente de reprise est de consommer le pipeline existant, vérifier son entrée
18080, puis lancer les scénarios intégrés, pas reconstruire une plateforme de
tests. Le fournisseur synthétique reste sur18081 ; mutation/restauration du
certificat et corrélations restent chez le parent, pas dans un k6 privilégié.
Source : `docs/missions/20261031-k6-auth-handover.md` ; projet
[[projects/aiforall/aiforall]].

## Frontières à préserver

`k6/http` porte les scénarios API ; `k6/browser` utilise Chromium réel pour cookies et redirections OAuth. Une succession de requêtes HTTP n'est pas une preuve navigateur. La campagne réutilise les oracles de [[projects/aiforall/references/auth-security-closure-20261029]] et les routes exactes de [[projects/iamrusty/decisions/0412-oauth-transaction-browser-binding]], sans changer l'autorité pour faciliter le runner.

Le parent de la campagne native contrôle ses fixtures, perturbations, observations
et nettoyage, mais ne démarre/répare/arrête pas le lab. Pas d'API administrative
générique ajoutée pour k6. Setup/teardown ne garantissent pas la fermeture après
kill du parent ; résumé frais, complétude, non-vacuité et fin des processus restent
exigés. Les checks seuls ne suffisent pas.

## Trajectoire Linux et authentification

Les trois ordres initiaux restent distincts : socle/smoke, portage des preuves
d'authentification, propagation/reprise des droits Linux. Le portage natif Windows
est qualifié par la campagne du12novembre, pas les campagnes de droits Linux.
Les tests Rust restent natifs Windows. OHA, performance, UI générique, HA
exhaustive et Grafana Cloud restent exclus.

Les 24 sources de l'enquête WSL sont des archives inactives, dont cinq sources
`ldd-only` tardives non exécutées par le lead. Leur préservation ne relance ni sonde
ni seconde mesure. Le nombre de dépendances manquantes du run historique reste
inconnu ; la qualification Linux sera une mission distincte demandée par l'utilisateur.

La dette matérielle HSM reste acceptée et non bloquante pour ces campagnes, mais n'est pas qualifiée ; ADR 0309 reste Partial. Les résultats attendus ne deviennent pas des résultats observés par leur inscription dans le wiki.

## Clôture et conservation de la mission1 uniquement

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
