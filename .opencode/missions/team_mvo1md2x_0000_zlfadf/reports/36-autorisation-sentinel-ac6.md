# Rapport 36 — extension Sentinel autorisée

## Décision humaine

À la question du rapport35 : « Autorises-tu ce sous-lot ciblé Sentinel — classifier idempotent et tests positifs/négatifs — en conservant AC1–AC7 et l'oracle de rejeu réel ? », l'utilisateur a sélectionné **Autoriser (Recommended)**.

Autorisation exacte : correctif minimal du classifier et tests positifs/négatifs, amendement ciblé du contrat et vérification indépendante bornée, aucune refonte. Ce n'est pas une ratification ADR0313 Accepted ni une autorisation de modification générale du worker/framework.

## Mise en œuvre bornée

Faire actualiser l'exception §6 du contrat par architecte, avant consommation par l'implémenteur. Continuer avec le même t3-runtime ; pas de nouveau propriétaire code ni de modèle forcé. Réutiliser la preuve rouge réelle du run2ebb73f602484bdf ; compléter les témoins ciblés du classifier avant correction quand possible. Préserver les formats historiques et ne pas convertir une erreur400 générale, authn/authz, indisponibilité ou une ambiguïté en succès.

L'oracle replay même événement/bytes avec ledger neuf reste inchangé ; aucune déduplication mémoire substituée. Relecture indépendante bornée après diff réellement livré, pas de chaîne QA globale. HTTP/résilience déjà autorisées restent indépendantes ; unique slotCargo Windows24jobs, fixtures/baux/cleanup inchangés. Tout besoin hors de cette extension est à remonter.

## État et limites

Autorisation persistée, aucun correctif Sentinel ni résultat vert nouveau annoncé à cet instant. Les messages en attente décrivant un blocage sur autorisation deviennent historiques pour ce seul sous-lot ; la barrière de remise du contrat amendé reste à franchir avant production.
