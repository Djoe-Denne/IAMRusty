---
title: >-
  OAuth Provider Linking — LOGIN, LINK et RELINK avec OAuthConnect v2
category: concepts
tags: [iam, oauth, authentication, security, visibility/internal]
sources:
  - docs/adr/0412-oauth-transaction-persistante-liee-navigateur.md
  - docs/adr/0413-oauthconnect-multi-instances-profils.md
  - docs/contracts/iam-oauthconnect-v2.md
  - docs/missions/20261108-iam-oauthconnect-v2-results.md
  - services/IAMRusty/domain/src/service/provider_link_service.rs
summary: >-
  IAM conserve comptes et liens ; v2 fournit tokens et identité ensemble. LOGIN exige un email, LINK ajoute sans vérifier, RELINK ignore les emails étrangers.
provenance:
  extracted: 0.95
  inferred: 0.05
  ambiguous: 0.0
created: 2026-04-14T17:46:37.6929647Z
updated: 2026-11-08
---

# OAuth Provider Linking

## Frontière de responsabilité actuelle

[[projects/iamrusty/iamrusty|IAM]] possède les UUID, comptes humains, emails, liens, tokens persistés et décisions de rapprochement. [[projects/oauthconnect/oauthconnect|OAuthConnect]] authentifie une identité externe configurée et retourne un résultat **groupé** `{tokens, profile}`. Il ne décide ni quel compte IAM modifier ni s'il faut relier deux personnes.

Depuis le raccord local du8novembre, LOGIN, LINK et RELINK appellent seulement `authorize_authentication` et `complete_authorization`. Aucun retour au couple token/profile v1, aucune identité reconstruite du seul access token. Les méthodes v1 restent pour leurs consommateurs historiques, pas comme fallback des nouveaux parcours.

Une identité externe est **(instance fournisseur, identifiant opaque)**. L'identifiant n'est pas converti en nombre : `00012345` garde ses zéros. Deux instances peuvent retourner le même texte sans désigner le même lien. Changer d'issuer sous le même slug n'est pas une simple rotation de clés.

## Les trois opérations ne sont pas interchangeables

### LOGIN : identifier le compte

Après une complétion valide, un email est requis **avant tout lookup métier**, même si l'identité fournisseur était déjà liée. Ensuite, le lien provider+ID est prioritaire ; un email fournisseur changé ne doit pas réaffecter le lien ou changer l'UUID. Le rapprochement par email n'est possible que si le fournisseur le déclare vérifié, selon les règles existantes de création/compte incomplet. Un email absent n'est pas compensé par un appel de profil v1.

### LINK : ajouter un fournisseur au compte authentifié

L'autorisation provient du principal plateforme vérifié au START. Le callback utilise la cible persistée, pas un `user_id` fourni dans une query. Le service refuse l'identité déjà liée au même compte ou à un autre selon les règles existantes.

Un email absent est accepté. Un email neuf peut être ajouté au compte cible, mais reste **non vérifié dans IAM**, même si `email_verified=true` dans le profil externe. Le résultat expose `new_email_added=true` quand cet ajout a effectivement lieu. Un email déjà possédé par une autre personne ne doit pas être transféré par ce parcours.

### RELINK : renouveler un lien existant

RELINK remplace les tokens d'un fournisseur déjà lié ; il conserve sa politique propre, plus permissive que LINK pour les emails. L'email d'un autre compte est ignoré sans réaffectation ; email absent est également admis. Ne pas copier arbitrairement les refus de LINK dans RELINK.

La collision avec une identité externe déjà liée ailleurs est caractérisée localement : avec `email=None`, la frontière existante d'unicité `(provider, provider_user_id)` refuse et préserve les deux liens et tous leurs tokens. Aucune nouvelle garde métier n'a été ajoutée pour rendre ce témoin passant. Le statut HTTP précis de ce refus n'est pas redéfini par cette note.

## Précédence commune et non-rejeu

1. IAM crée la transaction durable au START, puis demande l'URL au connecteur.
2. Au callback, les bindings navigateur/state/provider/intention/cible/redirect/expiration sont contrôlés.
3. Le writer verrouille, consomme, efface nonce et verifier, puis **commit** avant de rendre une capacité privée.
4. Un unique complete v2 utilise les secrets originaux de cette capacité.
5. Seulement après réussite, les règles métier ci-dessus s'appliquent.

Une erreur fournisseur ou un DTO contenant des tokens sans identité ne doit créer ni compte/email/lien/session/JWT. La transaction, elle, demeure consommée : nouvelle tentative = nouveau START. Voir [[projects/iamrusty/concepts/oauthconnect-v2-durable-context]] et [[projects/iamrusty/concepts/oauth-state-and-csrf-protection]].

## Preuves, statut et limites

Le lot IAM qualifie325unités et80IT sur11binaires, pas tout IAM ni un fournisseur live. Les témoins navigateur couvrent les trois intentions après refus/DTO partiel, les emails ci-dessus, les collisions, deux onglets, deux replicas, le replay et l'absence d'effets. Les revues statiques ne sont pas confondues avec ces exécutions.

ADR0413 reste **Accepted / Partial** : E2E GitHub/GitLab, équivalence réelle du `sub` GitLab, bascule et retrait legacy restent externes. Les anciennes instructions «nouvel IdP = nouveau service Connect» sont partiellement supersédées : instance compatible dans OAuthConnect, voir [[projects/iamrusty/skills/extending-iamrusty-with-oauth-providers]] et [[projects/iamrusty/decisions/0413-oauthconnect-multi-instances]].

## Sources et navigation

- [Contrat IAM v2](../../../../../docs/contracts/iam-oauthconnect-v2.md), particulièrement les précédences et politiques LOGIN/LINK/RELINK.
- [Résultats locaux exécutés](../../../../../docs/missions/20261108-iam-oauthconnect-v2-results.md).
- [[projects/aiforall/references/session-oauthconnect-iam-20261108]] — décisions, essais et contributions des sessions.
- [[projects/iamrusty/references/iamrusty-api-and-auth-flows]] — référence historique à croiser avec les routes du code courant.
