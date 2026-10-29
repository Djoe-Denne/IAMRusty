# AuthZ — propositions conservées lors de la consolidation

- Date : 2026-10-29.
- Statut : **Proposed — conservation documentaire non normative**.
- Auteur structurel : architecte.
- Portée : deux précisions du contrat proposé conservé dans le commit `fedca587fc278177c396064d83b6ab5a46281305`.
- **Ratification explicite du lead : conservation documentaire uniquement**, après lecture intégrale du texte architecte ; aucune acceptation des propositions techniques ni modification du contrat ratifié.

## Autorité et limites

Le canon reste [ADR-0314](../adr/0314-principal-authority-through-openfga.md) et son [contrat ratifié](../contracts/authz-trust-domain-boundary.md). Ce document ne les amende pas, ne change aucun Statut/Réalité et ne constitue ni contrat d’implémentation ni plan QA.

Les deux précisions ci-dessous ne figurent pas textuellement dans le contrat ratifié. Leur conservation évite une perte documentaire ; leur absence ne démontre ni régression applicative ni défaut de sécurité. Aucun nouveau travail de production, migration, implémentation ou qualification n’est demandé ici.

## P1 — Liaison durable entre store FGA et autorité plateforme

### Proposition historique

Source : `fedca587fc278177c396064d83b6ab5a46281305:docs/contracts/authz-trust-domain-boundary.md`, ligne 43 :

> La liaison entre store FGA et autorite plateforme est epinglee pour toute sa duree : changer platform_issuer ou les aliases exige migration coordonnee et reattribution de provenance, jamais reutilisation des anciens droits par simple configuration.

### Qualification par rapport au canon

L’intention de ne pas réattribuer implicitement des droits historiques est cohérente avec ADR-0314, décisions 2, 3 et 6, et avec le contrat ratifié §§3 et 6 : provenance des droits plateforme, issuer exact, absence de fallback et réconciliation coordonnée.

En revanche, l’épinglage explicite « pour toute sa durée » à un store FGA est une précision supplémentaire non ratifiée. Le canon configure une autorité plateforme exacte et une compatibilité historique bornée ; cette proposition n’introduit aucune nouvelle liste d’alias autorisés.

Aucune contradiction nécessaire avec Accepted n’est identifiée dans l’intention. Une lecture interdisant toute évolution même après migration coordonnée ne serait pas retenue : le canon prévoit précisément des bascules coordonnées.

### Questions conservées, non décidées

- Quel périmètre désigner par « store FGA » : store logique, restauration, environnement ou autre identité durable ?
- Faut-il matérialiser cette liaison et, si oui, où, avec quel contrôle et quelle procédure d’évolution ?

Piste d’oracle, non prescrite : vérifier qu’un changement de configuration ne permet pas à une autre autorité de récupérer les anciens droits `user:<uuid>`, tout en conservant le positif de migration avec provenance attestée.

Aucun mécanisme de stockage, contrôle au démarrage, format persistant ou nouvel oracle obligatoire n’est décidé ou qualifié par cette note.

## P2 — Invitations liées au seul UUID du destinataire

### Proposition et oracle historiques

Source : même fichier au même commit, ligne 64 :

> Les invitations dont aggregate_id ne lie que le UUID destinataire restent account-only : refuser une identite non plateforme avant consommation/mutation, meme sub. Source : services/Hive/domain/src/service/invitation_service.rs::accept_invitation. Le positif plateforme reste requis. Leur generalisation multi-autorite est hors lot ; ne pas laisser AuthPrincipal contourner ce binding legacy.

Oracle proposé à la ligne 101 :

> uuid_bound_invitation_rejects_foreign_authority_before_consumption : meme UUID, autorite distincte, invitation non consommee et aucune creation membre apres refus ; temoin plateforme positif.

### Qualification par rapport aux frontières actuelles

Cette proposition est une application ciblée compatible avec ADR-0314, décision 5, et le contrat ratifié, invariant 5 et §5 : une frontière UUID-only ne transforme pas une identité étrangère en identité plateforme, même après autorisation FGA.

La revue indépendante de consolidation rapporte un handler actuel utilisant `AuthUser` et une méthode domaine comparant `aggregate_id` au UUID. Ce constat est une source statique réutilisée, pas une nouvelle vérification du code ni une qualification de tous les appelants. L’écart documentaire seul ne démontre donc pas un bug d’acceptation d’invitation.

L’annexe ratifiée « Application de la frontière UUID-only à l’auto-invitation interne » traite un autre chemin : la création d’invitation depuis la synchronisation. Elle ne constitue pas, à elle seule, une preuve sur la consommation d’une invitation.

### Questions et oracle conservés comme propositions

L’oracle nommé reste une proposition historique ; cette note ne le déclare ni implémenté, ni exécuté, ni manquant au regard d’une campagne existante. Elle ne l’ajoute pas au plan QA.

La couverture de tous les appelants, les garanties de consommation et l’absence de mutation ne sont pas qualifiées ici. Aucune conversion du schéma d’invitation, nouveau principal destinataire ou interdiction générale des identités organisation dans Hive n’est décidé. Une telle interdiction générale contredirait ADR-0314 et n’est pas retenue.

## Disposition documentaire

Conserver ces deux propositions et leurs questions sans les promouvoir en exigences. Aucune nouvelle ADR, modification du contrat Accepted ou correction applicative n’est nécessaire pour cette seule conservation. Une décision structurelle séparée ne serait à examiner que si un futur lot devait effectivement fixer une liaison persistante ou étendre le modèle d’invitation.

## Sources et vérifications

- Snapshot Proposed : `fedca587fc278177c396064d83b6ab5a46281305`, contrat AuthZ, lignes 43, 64 et 101.
- Canon consulté : ADR-0314 et contrat AuthZ ratifié, notamment §§3, 5, 6 et annexe auto-invitation interne.
- Revue indépendante : `.cursor/review-briefings/20261029T1920Z-correctness-worktree-consolidation-052817d.md`, §§« Deux précisions Proposed » et « Vérifications et limites » ; source examinée `052817d61885e9585e1ecc491974915bf85b7199`.
- Continuité : `.cursor/handoffs/20261029-worktree-consolidation.md`.

Comparaison documentaire uniquement, depuis le dépôt canonique et les références Git. Aucun worktree externe lu, aucune exploration Rust nouvelle, aucune écriture par l’auteur, aucun Cargo/fmt/test, runtime, publication ou ressource Docker par l’architecte. Aucun PASS comportemental ou global revendiqué.
