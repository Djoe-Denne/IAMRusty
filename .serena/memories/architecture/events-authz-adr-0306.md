Jalon : architecture actuelle / AuthN signature org.
Chemin : docs/adr/0306-hive-iam-configuration-signature.md
Statut : Accepted · Réalité : Unimplemented

- Config signer = commande synchrone RPC/HTTP s2s Hive→IAM après AuthZ org admin. Pas de secret dans les events.
- Pas Telegraph (correction « Telegraf » → Telegraph = notifications 0403). Aucun bus de commandes Hive→IAM n’existe.
- Commandes : Configure/Test/Rotate/DisableOrganizationSigner. Hive = métadonnées UX ; secrets dans OpenBao via IAM.
- Runtime : pas de client Hive→IAM ; events MemberJoined / OrganizationCreated sans KMS.

Voir le fichier ADR.