# Le trajet HTTP étudié n'est pas public — outillage toujours en cours

## Objectif et problème concret

Mission1 demande un GET public IAM retournant200 et OK, sans credentials. Le port18080 proposé traverse Envoy, la passerelle d'entrée. Le handler IAM est public, mais cela ne suffit pas : la configuration source de la passerelle garde le contrôle d'autorisation externe sur le préfixe /iam/. Sans jeton Bearer, ce contrôle refuse la requête.

Ce constat explique un deuxième prérequis à la preuve HTTP : il faut non seulement une cible Linux contemporaine, mais aussi une exposition réellement publique compatible avec le smoke. Aucun403 live n'a été mesuré et aucune campagne n'a été lancée. Il n'est pas question de présenter un refus attendu de la passerelle comme un bug produit reproduit.

## Résultat vérifié et disposition

Le retour du second architecte a été recoupé dans la configuration Envoy, le renderer local-full et le handler ext-authz. Son seul delta nouveau est conservé comme addendum architecte ratifié. Les anciens statuts Proposed de sa proposition ne sont pas réintroduits ; ADR0504/contrat restent Accepted / Unimplemented.

L'auteur outillage a reçu le nouveau commit et l'obligation de lire le contrat intégré, plutôt que supposer que son worktree se synchronise. Il continue les scénarios, le lanceur, les tests locaux et Chromium, sans activer HTTP par défaut sur cette origine ni ajouter un booléen d'autorisation auto-attesté.

## Suite et limites

Rechercher uniquement une exposition publique existante autorisée et vérifier son lien à la cible ; tout changement d'origine de la V1 exige un amendement architecte ratifié avant activation. Aucun token, exemption, route, tunnel, port-forward ou statut403 substitué au200/OK n'est ajouté pour rendre le smoke vert. L'absence de preuve HTTP ne bloque pas les travaux indépendants utiles, mais interdit le PASS global.

La proposition alternative de l'architecte est sauvegardée et vérifiée, son arbre conservé. Les autres éléments recouvrent le contrat déjà accepté ou sont des options de provenance non qualifiées ; pas de fusion globale ni de downgrade documentaire. Aucun nouveau GO de jalon requis.

## Annexe de preuves

- Commit addendum3f2e8fbe625b47fbc4093e673dc2804bcf9a3023, parentc9b537c ; deux documents seulement ; diff-check et vrai hook cargo fmt réussis.
- ops/deploy/apps/overlays/kind-mesh/envoy-mesh.yaml:167–170,201–205 : route /iam/ sans exemption et filtre fail-closed.
- ops/deploy/render-local-full.py:139–152 : transport downstream adapté et callbacks filtrés, pas d'exemption health ajoutée.
- workers/ext-authz/src/check.rs:82–84,128–130 : absence de Bearer refusée,403 ; test denies_envoy_client_path_without_bearer:327–339 lu, non exécuté, chemins génériques de refus, pas test live spécifique /iam/health.
- Proposition pool-3/session ses_e7697c371ffe5Dl4RX4kPhuJDt ; backup .tmp/k6-architecture-alternative-aec55bb7810945c38da3adb672e5843f avec SHA256 vérifiés. Membre arrêté normalement, aucune suppression de son arbre ni fusion fictive annoncée.
- Mémoire Serena de0504 signalée stale/Proposed ; ce digest non canonique n'est pas utilisé comme autorité. Son delta local préexistant à la ratification n'est pas écrasé aveuglément ; les documents Git acceptés priment.
