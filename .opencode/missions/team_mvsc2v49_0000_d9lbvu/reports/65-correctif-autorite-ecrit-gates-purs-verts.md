# Correctif d'autorité écrit, vérifications ciblées vertes

Le contrat de l'architecte est ratifié et le correctif est écrit : l'émetteur
et l'audience configurés sont transmis explicitement au service de jetons dans
ses deux assemblages de production. Aucun contrôle de signature, de registre ou
de révocation n'est relâché ; aucun ancien émetteur n'est accepté en secours.

Les vérifications locales passent : **19 tests du domaine, trois tests du vrai
codec RS256**, compilation du service IAM et formatage. La nouvelle régression
de connexion après inscription est écrite mais pas encore exécutée. La revue
sécurité indépendante du delta est en cours.

L'étape suivante utilise les fixtures d'intégration existantes. Le lanceur ne
sait actuellement sélectionner que deux binaires Manifesto : un ajustement fermé
pour sélectionner la suite OAuth IAM est en préparation, afin d'éviter de relancer
tout IAM ou de créer un autre dispositif de tests. Ses règles d'inventaire et
de nettoyage resteront inchangées.

Le laboratoire n'utilise pas encore ce correctif. Après intégration et tests
finaux, son image IAM sera livrée avant la reprise du vrai navigateur. Aucun
succès global n'est revendiqué.

## Preuves et limites

- Contrat `docs/contracts/oauth-domain-token-authority.md`, proposition86lignes
  lue et ratifiée avant code ; cinq fichiers IAM autorisés.
- Tests :19PASS/0ignoré/34filtrés ;3PASS/0ignoré/0filtré.
- `cargo check --locked -p iam-service` et `cargo fmt --all -- --check` :exit0.
- Windows natif,24jobs, un Cargo à la fois, cibleE: ; aucun conteneur créé par
  ces vérifications. Logs `oauth-authority-pure-gates-b32a2d267d104b209b787fb33687e637`.
