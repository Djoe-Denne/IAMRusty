# Apparatus P4 — ADR-0012

Canon : `docs/adr/0012-apparatus-http-knative-par-binding.md` ; contrat v1 : `docs/contracts/apparatus-knative-runtime.md`. Statut : **Accepted le 2026-10-20** (pins B5 via spike séparé) ; Réalité : **Unimplemented** (annexes A du 2026-10-20, aucune preuve runtime). Voir le fichier ADR.

Annexe §13.1 du contrat **ratifiée le 2026-10-20** (hash JCS/SHA-256, identités/rôles, lease/fence/cursor, erreurs v1) — interface gelée et consommable par C/D/T ; Statut/Réalité inchangés.

Annexe §13.2 **ratifiée le 2026-10-20** : lookup instances via GET binding borné (sélection Page/ByPodUid/ByInstanceId, fraîcheur ≤5 s, tombstones jamais allow), discovery Pod UID sans UUID client (instance_id retiré de la requête enrollment, retourné en réponse ; Downward API ; unicity par Pod UID), `target_binding_id` sans alias, DTO accept/context/begin/reservation/registration/complete précisés (completion_state distinct). HEAD interface C/D/T publication autorisée ; sans nouveau endpoint ni changement de Statut/Réalité.

- Cible : Serving/KPA HTTP par projet × binding, pré-déclaration à zéro ; autorisation Lazaret avant réveil, pas de buffer/autoscaler maison. 0011 Rejected reste historique ; supersession de 0008 partielle, effective à l’Accept du 2026-10-20.
- B1/B2 proposés : caller workload déjà vivant distinct de cible, nouvelles routes runtime/ticket et enrollment Pod UID↔Revision↔binding ; net-istio/Istio mTLS + gVisor à ratifier/qualifier, pas auth par headers. WorkloadIdentity/SessionClaims conservés.
- B3/B4 proposés : writer Manifesto et stockage/port internes, CAS/epochs/tombstones ; accept après buffer et begin-effect sérialisé avec révocation. Effet engagé avant révocation peut finir ; compute/objet résiduel possible sans nouvelle autorisation, pas exactement-once/cancellation magique.
- B5 **verrou NON LEVÉ** : valeurs toutes à ratifier ; pins exacts/digests Serving/net-istio/Istio/gVisor non fournis. Pas de GO publication B/C/D/T ni exposition hostile avant lock ratifié/compatibilité prouvée. B6 : HTTP fini, état externe, migrations additives, secrets par référence, purge hors tranche.
- Contrat §§13–15 : inventaire API/claims/DTO/traits/schémas/migrations sans nouveau event ; ownership unique (partagés/Cargo=C, Lazaret+référence=D, infra=B, tests séparés=T), gates A/Q/B/C/D/T/E et rollback fermé sans digest/local.
- Accept humain consigné le 2026-10-20 (pins via spike séparé = condition d’entrée de B) ; 0009/0010/0600 inchangées, aucune clôture pré-prod/APP/TCB. Travail documentaire seulement, pas de code/test/build/runtime/commit.
