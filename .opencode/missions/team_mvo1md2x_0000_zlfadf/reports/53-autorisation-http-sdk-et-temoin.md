# Rapport 53 — extension HTTP SDK autorisée, témoin avant correction

## Autorisation humaine

À la question du rapport52 « Autorises-tu ce sous-lot minimal rustycog-http — ownership HTTP additif, tests ciblés, publication/pin exact et raccordement standalone/monolithe — en conservant AC1–AC7 ? », l'utilisateur choisit **Autoriser HTTP SDK**.

Portée autorisée : témoin rouge d'abord, correction sur pile existante, contrat/ADRProposed et revue indépendante bornée ; aucun nouveau backend ou harness. Cela ne ratifie aucune ADR Accepted, ne modifie ni l'oracle6s ni les exclusions exactes§5.2, et n'autorise pas de refonteglobale/APIgénériquepluslarge/désactivationprotocole silencieuse.

## Découpage et parallélisme utile

1. Architecte, même session : rédiger le contrat/API additive, limites de surface et ADRforwardProposed après contrôle numéro libre. Pas de codeapp/Cargo/runtime. Artefacts intégrés avant auteur SDK.
2. Test-engineer isolé, contexte fresh et mandat limité : reproduire C-1 sur **code actuel** aa400f3 avec vraie socketloopback et handlerasyncpending/RAII, chemin HTTP réel et vrai helper shutdown ; rouge avant fix, nettoyage même en échec. Aucun Docker, nouveau backend/dependance/harness ou correctionproduction. Ce témoin dépend du contratAC7 existant et findingpersisté, pas de la nouvelleAPI encore à définir : il peut avancer en parallèle de la rédactionarchitecte.
3. Après intégration des entrées et du rouge, hard-implementer pour SDK+adoptionTelegraph/monolithe selon contrat, tests complémentaires et pin exact publié/revu. Reprendre les contextes utiles quand le routage natif permet, sinon fresh explicitement documenté. Un seul CargoWindows24jobs, pas modèle forcé.

Les consommateurs de l'API future ne sont pas publiés avant intégration du contrat/témoin. Pas de duplication des tâchesT3/T4/T5 déjà livrées : nouveau sous-lot correctifC-1 autorisé. T5docsstaged préservées en attendant réconciliation factuelle, aucune nouvelle tentativehooklongue.

## QA bornée

Contrat de preuve `.cursor/qa-plans/20261026-http-ac7-c1.md` persisté avant création du témoin. QA mission demeure proportionnée, pas scores/classification/pipeline obligatoires nouveaux. Témoins positifs/négatifs utiles au risque partagé, pas recampagneglobale. Le rouge courantHTTP n'est pas encore exécuté : statut statiqueHIGH de T4 conservé jusqu'au résultat réel.
