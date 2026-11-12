---
title: >-
  Tester avec k6 — IaC et fixtures d'abord
category: skills
tags: [testing, ops, security]
sources:
  - .agents/skills/aiforall-k6/SKILL.md
  - .agents/skills/aiforall-k6/references/native-campaign.md
  - ops/tests/k6/native/README.md
  - docs/missions/20261112-k6-three-native-results.md
  - docs/history/iac-k6-qa-20261112/README.md
summary: >-
  Déclarer la slice de test, préparer des inputs privés par campagne, créer les données via les vraies API et corréler k6 au parent avant cleanup UID/OID.
provenance:
  extracted: 0.98
  inferred: 0.02
  ambiguous: 0.0
created: 2026-11-12
updated: 2026-11-12
---

# k6 : IaC et fixtures d'abord

La voie locale à trois scénarios est `ops/tests/k6/native/campaign.ps1`, pas
`just k6-* iam` (12 cas stateless). Le skill agent `aiforall-k6` est la procédure
exécutable à relire ; cette note conserve le modèle mental et ses limites.
Voir [[projects/aiforall/concepts/k6-functional-linux]] et
[[projects/aiforall/concepts/runtime-validation-separation]].

## Répartir les responsabilités

- **Chart** : workloads, Services, montages, probes, ressources et selectors.
  Helm rend les objets ; pas de deuxième déployeur de specs en Python/PowerShell.
- **Parent natif** : sources privées, autorité de cible, reçus UID/OID,
  préimages/restauration SAN, données d'observation, processus et nettoyage.
- **JS k6** : stimuli et assertions produit ; ni Kubernetes privilégié,
  provisioning ou secrets opérateur. Une VU/itération par cas fonctionnel.
- **IdP Python** : protocole fournisseur derrière le vrai OAuthConnect v2 ;
  pas un faux IAM, un JWT fabriqué ou un connecteur simulé à la place du réel.

## Alimentation de bout en bout

Le JSON privé hors Git fournit dossier opérateur, kubeconfig, outils absolus
épinglés et images explicites. Les inputs complets IAM/Envoy/PKI sont réutilisés
sans modifier les originaux. `prepare` observe la cible et dérive run ID,
configuration DB dédiée, HMAC/client propres au run, certificats courts,
rendu Helm, contexte public fermé `target.json` et empreintes.

`prepare` ne crée ni workload, DB ou confiance CA, mais ce n'est **pas** une
commande sans accès au lab. `run` revalide les empreintes et crée la DB
`k6_<run>`, puis 8 Secrets et 10 objets du chart. IAM initialise son schéma.
Le navigateur crée ensuite le compte par inscription OAuth/complétion, refait
un login puis un relink authentifié. L'identité est liée au run ; pas de seed SQL
qui court-circuite le parcours qu'on prétend tester.

La slice a ses IAM/OAuthConnect/Envoy/IdP ; Postgres, ext-authz/OpenFGA et autres
backings restent partagés. Une DB éphémère **n'isole pas les queues/mails**.
Avant password/refresh ou un nouveau parcours, définir effets aval et inverse
sûr : supprimer user/outbox n'annule pas une notification déjà émise.
Voir [[projects/aiforall/concepts/qa-proportionate-proof]].

## Exécuter sans transformer les prérequis en scénario

- Windows x64 non élevé, lab autorisé déjà disponible ; images et outils déjà
  présents. La campagne ne démarre, ne répare et n'arrête pas le lab.
- CA publique prétrustée durablement dans CurrentUser\Root, hors timer des
  forwards et hors cleanup ; Chromium complet avec manifeste/fichiers compagnons.
- IAM `https://127.0.0.1:18443`, IdP navigateur `https://localhost:18445` :
  deux hosts isolent le cookie racine, deux ports seuls non.
- `prepare` dans un dossier neuf privé → `run` sous accord DDL exclusif →
  `clean` pour reprendre une fermeture interrompue, jamais rejouer le même run.

Les JS reçoivent un environnement fermé et le contexte public validé du parent.
Un input natif absent ne justifie pas de tomber sur le fallback legacy HTTP.
Conserver le tag `group` pour les checks nommés, et vérifier les cookies du
contexte avec domaine/Path/attributs explicites. Le cookie HTTPS réel est
`__Host-iam-oauth-browser`, Path=/, Secure, HttpOnly et SameSite=Lax.

## Prouver puis fermer

Un PASS exige santé2 checks, trois phases SAN2 chacune et OAuth26 : **34**.
Le parent corrèle SAN+1, même incarnation Envoy/restauration, IAM+7,
ext-authz+4, fournisseur/PKCE+3, users+1/outbox+0 et zéro erreur transport.
Exit nul, mocks ou `k6 inspect` ne suffisent pas.

Restaurer le bon SAN et vérifier le retour avant suppression ; arrêter les
writers puis DROP DB avec reçu OID/zéro connexion sous exclusivité DDL.
Contrôle OID puis DROP par nom restent non atomiques. DELETE Kubernetes porte
une précondition UID ; joindre forwards et processus. Un échec de restauration
reste explicite, pas effacé par une suppression des preuves.

Ne publier que projections allowlistées et empreintes des sources exécutées.
Jamais de zip récursif du dossier ayant contenu configurations, cookies,
queries OAuth ou clés. Preuve 54a19ba ; le retrait de constante fc5e29b et les
skills/profils suivants n'ont pas rejoué l'E2E. CI/legacy reportés. Provenance :
[[projects/aiforall/references/session-iac-k6-qa-20261112]].
