# ADR-0306 digest

- Jalon : events-authz / config signature Hive→IAM
- Chemin : `docs/adr/0306-hive-iam-configuration-signature.md`
- Statut : Accepted (amendement transport 2026-09-27)
- Réalité : Partial

- Sync ACL configure/test/rotate/disable ; pas de secret en event ; Hive metadata only ; IAM SoT crypto ; Telegraph hors canal.
- Transport inchangé : InProcess si même process ; HTTP sinon. InProcess = capability ; HTTP + `/iam/internal/...` = token fail-closed (0307 = credential HTTP seulement).
- InProcess monolithe : pont IAM + sucre Hive `with_iam_organization_signer_client` (champ du sac 0104 Partial).
- Motif plateforme sac `*OutboundOverrides` = 0104 (Proposed / Partial) ; Related 0104. InProcess IAM via setter sucre. Gaps Partial : BYOKMS, events SigningProfile*.
- Voir le fichier ADR.
