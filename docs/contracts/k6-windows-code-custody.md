# Protection persistante du code k6 Windows — proposition abandonnée

- Statut : **Abandoned / Superseded — jamais ratifiée**.
- Disposition documentaire du 2026-10-31 ; Réalité : aucune protection persistante
  qualifiée par cette proposition. Aucun résultat réel Windows n'en est déduit.
- Remplacement : [amendement révision3 du contrat Windows](k6-windows-functional.md#amendement-révision3--simplification-windows),
  **Accepted**, ratifié par le lead le 2026-10-31. Le contrat historique Accepted et le
  [raccord browser-version Accepted](k6-windows-browser-version.md) sont conservés.

La proposition du 2026-10-30 n'a jamais été ratifiée. La décision utilisateur,
révision3 approuvée `82a5aefa6a4dcd7deb05121d85b35da21b567585b615661f568ff45d27a22bb6`,
abandonne ses obligations de protection persistante, validations ACL, leases/verrous
exécutables/DLL et leurs oracles QA. **Ne pas les implémenter ni les compléter** ;
son ancien gel des vrais runs est caduc, pas un prérequis supplémentaire.

Conserver les protections natives Chromium et permissions Windows ordinaires,
versions/provenance/SHA256, profil séparé, timeout et nettoyage raisonnable des seuls
processus créés. Aucune sonde ou attestation du sandbox n'est requise. Après correction
minimale et contrôle court, le premier cas réel reste `about:blank` / `6 * 7 === 42`,
sans nouvelle enquête ou revue préalable. Les autres conditions du contrat restent.

Provenance de la proposition remplacée : corps de 133 lignes lu dans le checkout
canonique, SHA256 `6652566a6f76ab8f9589f3055364805f557e0297ac07ff1e25ccbba3b498cab9`.
Cette empreinte identifie l'ancien candidat, pas une ratification ni une preuve
d'exécution. Disposition sourcée par le briefing
`.cursor/review-briefings/20261031-k6-windows-simplification.md`, le rapport94 et
`.cursor/handoffs/20261029-k6-01-socle.md`. Aucun ledger concurrent modifié ici.
