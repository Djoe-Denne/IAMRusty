# Cause identifiée : autorité incohérente pour un jeton intermédiaire

Le diagnostic établit un défaut dans le parcours de connexion d'un compte déjà
inscrit : il essaie d'abord de signer un jeton intermédiaire avec l'ancien nom
d'autorité, alors que le signataire exige l'autorité configurée. Ces deux noms
diffèrent ; le contrôle de sécurité refuse donc correctement la signature.
L'erreur est ensuite présentée comme401.

L'inscription ne prend pas cette branche, ce qui explique pourquoi elle peut
réussir avant l'échec de la connexion suivante. Le défaut est antérieur au
correctif réseau. Les tests existants vérifiaient l'inscription ou construisaient
directement un jeton avec la bonne autorité ; ils ne couvraient pas cette succession.

La contradiction est démontrée dans le code et concorde avec les observations,
sans lecture directe de l'erreur interne du service. La suite est une reproduction
avec le vrai codec de signature et une correction bornée, **sans accepter l'ancien
nom ni relâcher les contrôles**. L'architecte fixe le raccordement minimal avant
tout changement de contrat. Le navigateur sera repris après validation du correctif.

La reproduction avec le vrai codec réussit maintenant : l'ancien chemin est
refusé, tandis que le chemin configuré est accepté avec une clé équivalente.
Cela confirme le défaut de composition ; le code de production n'est pas encore
corrigé et le navigateur n'est pas validé.

## Preuves et limites

- Diagnostic natif31 : msg_1882ff2220014CJkK3x1qraT1K ; briefing persistant dans
  `.cursor/review-briefings/20261029-oauth-complete-user-issuer-conflict-3c2e7ea.md`.
- Huit fichiers causaux inchangés depuis la source de l'image déployée.
- Aucun test ou runtime exécuté par le diagnosticien ; reproduction ensuite par
  le lead :1test PASS,0ignoré,2autres filtrés, Windows natif24jobs, sans PostgreSQL
  ni conteneur créé. Formatage final réussi.
- Les transactions OAuth et données fournisseur peuvent être écrites avant ce401 ;
  aucun refus sans effet revendiqué pour ce parcours incomplet.
