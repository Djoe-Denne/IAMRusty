# Envoy refuse le mauvais nom ; certificat correct restauré

Le test réel réussit : le bon certificat donne une réponse303 ; un certificat
signé par la même autorité mais portant un autre nom donne503 et incrémente
précisément le compteur de refus du nom. Après restauration, la route répond
à nouveau303. Ce résultat démontre le contrôle du nom par Envoy, pas seulement
la validité d'un certificat dans un client HTTP.

Le bon certificat et le service IAM sont restaurés. L'identité d'Envoy et son
compteur de redémarrage restent inchangés. Deux remplacements de Pod IAM prévus
par le test sont tracés ; aucune suppression de données ni lecture de Secret.

La prochaine étape est la matrice du navigateur réel. La vérification infra
indépendante continue sur les preuves figées ; aucun succès global ni qualification
de production n'est déduit de ce test.

## Preuves techniques

- Wrapper7da5… sur livraison3c2e7ea : exit0 en93.778s.
- Compteurs `(handshake, refusNom, autreErreurVérification)` :
  bon `(2,0,0)` ; mauvais nom `(2,1,0)` ; bon restauré `(3,1,0)`.
- UID IAM : df614… →236607… →377006… ; UID Deployment/Secret contrôlés.
- Résultat `envoy-san-31e1764abf534fbfa2d1e4e1474298c0/summary.json`,
  restauration du patch, du rollout et de la route attestée.
