# Vérification du vérificateur de service Manifesto

La mission doit préserver le parcours de service légitime sans permettre à son
identité d'être utilisée comme compte humain. Le correctif ratifié est maintenant
intégré au code canonique, mais n'est pas encore validé à l'exécution.

La nouvelle déclaration d'autorité est obligatoire et propre à la seule route
qui fournit l'instantané des droits. Le vérificateur garde sa clé, le destinataire
attendu du jeton et son algorithme de signature ; aucune
compatibilité historique ou politique IAM globale n'est activée pour contourner
le refus. Les tests utilisent la vraie cryptographie et le vrai middleware.

Je lance les vérifications natives Windows sur l'arbre gelé. Une revue sécurité
indépendante examine en parallèle le diff intégré et ses scénarios adversariaux.
Les tests de base de données seront sélectionnés après ce premier résultat,
avec les fixtures existantes, pas dans Kind.

IAM reste prêt. Le correctif équivalent des six routes Envoy est encore chez son
auteur. Aucune nouvelle image Manifesto ni preuve Envoy/navigateur n'est annoncée.

## Périmètre technique

- Base ef4bc1f + sept fichiers Manifesto du contrat ; SDK02e5 inchangé.
- Vérificateur dédié : HS256, audience manifesto-bindings, politique historique refusée.
- Formatage global, unitaires configuration, quatre tests setup ciblés : lancés,
  résultats en attente. Un seul Cargo natif, 24 jobs, cible E:.
- Revue fresh en lecture seule, sans Cargo, runtime ou lecture de Secret.
- Récupération du candidat WIP via patches ; trois fichiers exacts en octets.
  La configuration Rust et les trois TOML ne diffèrent que par CRLF/LF ;
  les sept contenus normalisés sont identiques. Aucun résultat de test d'auteur
  n'a été réutilisé puisqu'il n'en avait exécuté aucun.

## Premiers résultats

Le formatage passe. Le test de configuration et les quatre tests du vérificateur
passent réellement, sans échec, test ignoré ou filtré. Les témoins positifs et les
refus d'autorités étrangères utilisent la vraie cryptographie et les vraies routes.

Ces résultats ne prouvent pas la base de données ni le démarrage de la future image.
La revue indépendante continue ; le prochain contrôle compile aussi le consommateur
monolithique, sans relancer les services inchangés.
