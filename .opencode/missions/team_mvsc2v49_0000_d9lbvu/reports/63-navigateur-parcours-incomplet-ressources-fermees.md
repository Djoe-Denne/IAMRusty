# Navigateur : parcours incomplet, ressources temporaires fermées

Chrome réel a exécuté le scénario dans un contexte dédié, mais une assertion
du parcours a échoué. La matrice n'est donc pas validée. Les observations montrent
deux échanges OAuth réussis avec le fournisseur simulé et cinq appels IAM ; elles
ne suffisent pas à identifier l'assertion ni à valider les routes négatives.

Le contexte, Chrome et le tunnel du fournisseur sont fermés. Les observations
avant/après sont conservées sans cookies, jetons, corps ou URL de callback. Je
localise l'échec par ces preuves et des diagnostics bornés, sans rejouer le
scénario à l'aveugle ni modifier ses assertions pour le faire passer.

Les preuves réseau et certificat précédentes restent valides ; aucun résultat
partiel du navigateur n'est présenté comme réussite globale.

Mise à jour : un diagnostic HTTP distinct localise un refus401 sur le callback
de connexion du compte de preuve. Il ne remplace pas le navigateur et n'identifie
pas encore la cause interne. L'analyse ciblée porte maintenant sur la création
de session et l'émission du jeton IAM ; aucune modification applicative engagée.

## Preuves techniques

- Chrome149, profil temporaire neuf, PID32952 possédé puis arrêté.
- Première invocation : erreur de syntaxe avant exécution, corrigée en retirant
  uniquement le point-virgule final de l'enveloppe ; fonction de scénario inchangée.
- Une seule exécution fonctionnelle ensuite : échec expurgé, contexte fermé.
- Fournisseur : authorize/token/profile/pkce_validated chacun0→2 ; IAM requests+5.
- Envoy même identité/epoch0 ; manager terminé, forward_closed et chrome_stopped
  confirmés, aucune installation ou configuration globale modifiée.
- Dossier `browser-proof-6c0f94b08ab4455cb1d6fcb5d369dd60`.
