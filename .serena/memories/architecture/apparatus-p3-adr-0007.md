# Apparatus P3 — ADR-0007

- Jalon P3. Canon : `docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md`. Statut : **Accepted**. Réalité : **Implemented** (A-DEC 2026-09-20). SuperSède : aucune.
- Décision : BC **Lazaret** (`Lazaret` / `lazaret-service`) = frontière capacités/données, distinct de Manifesto. 0006 G et E restent (pas d'`invoke` sur `ApparatusRuntime` Manifesto ; zéro `gateway` sous `Manifesto/*/src` ; `/components` gelé à 5).
- Preuve P3 : T1–T14b. Hors-jalon (ne bloquent pas Implemented) : APP-05 ouvert ; G/E en vigueur ; pas K8s ; pas de second protocole.
- APP-05 : options A/B non tranchées — voir 0004 Non décidé. V1 privé déjà testé (T7).
- Voir le fichier ADR. Ce digest n’est pas le canon.
