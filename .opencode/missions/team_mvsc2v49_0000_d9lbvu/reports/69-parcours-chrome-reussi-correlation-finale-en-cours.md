# Parcours Chrome réussi ; corrélation finale en cours

Le navigateur réel réussit maintenant la connexion puis la réassociation du
compte par les routes exactes d'Envoy. Les trois appels sans authentification
reçoivent403. Le contexte neuf, Chrome et le tunnel temporaires sont fermés.

Les observations du même run concordent : deux autorisations, échanges de code,
profils et validations PKCE côté fournisseur ; quatre requêtes IAM pour les
quatre étapes positives et quatre contrôles d'autorisation. Aucun accroissement
des erreurs de vérification TLS ou des erreurs IAM5xx.

Ce n'est pas encore une clôture : une revue bornée examine les inférences tirées
des compteurs agrégés et du lien entre la configuration statique et Envoy actif.
Le contrat demande aussi deux suites d'intégration supplémentaires sous le même
assemblage ; leur sélection ciblée dans le lanceur existant est en préparation.
Il n'y a ni nouvelle plateforme de tests, ni nouvelle demande de GO de principe.

## Preuves et limites

- Source7b8c1f95/SDK02e5, IAMimageb6f140… ; contextefull1+2, entrée18080.
- Chrome149/contexteinitialementvide ; scénario existant inchangé, aucune
  extraction ou archive de JWT/cookie/URL de callback.
- Dossier `browser-proof-2b6c3a80b3b6498aaedcd8393987b180` : résultat outil transcrit,
  observationsavant/après, runtime et corrélation. Envoy mêmeincarnation/epoch0.
- Limites explicites : compteurs négatifs non isolés par sous-phase ; aucune
  mesure directe DB. Configuration reliée par Pod/mount/ConfigMap/effets réels,
  pas capture directe de config_dump. Compteurs absents conservés MISSING.
