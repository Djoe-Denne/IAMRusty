# ADR-0104 digest

- Jalon : architecture actuelle / hexagone (hors P-Apparatus)
- Chemin : `docs/adr/0104-outbound-overrides-composition-root.md`
- Statut : Proposed
- Réalité : Partial

- Sac typé `*OutboundOverrides` local à chaque `*-setup` consommateur ; `Default` = HTTP ; `with_outbound` ; setters nommés = sucre.
- 0102 clarifiée, non SuperSédée : binaire service ne câble pas ; hôte 0404 peut fournir des adapters InProcess dans `runtime/monolith/`.
- Livré : Hive + Lazaret sacs ; getter Manifesto `binding_grant_snapshots()` ; ponts `in_process_iam_signer` et `in_process_binding_grant` ; fail-closed monolithe.
- InProcess = capability ; HTTP (+ routes nestées) = token fail-closed (0306/0307). Hors scope : 0308, 0309.
- Voir le fichier ADR.
