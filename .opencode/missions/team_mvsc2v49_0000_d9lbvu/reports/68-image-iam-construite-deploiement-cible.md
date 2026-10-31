# Image IAM corrigée construite ; déploiement ciblé

Les deux étapes de construction Linux réussissent : artefacts IAM et migration,
puis image d'exécution IAM. Aucun autre service n'est reconstruit. Les images
sont identifiées et enregistrées ; aucune ancienne étiquette n'est écrasée.

Le déploiement qui suit ne modifie que deux références d'image du même service :
le conteneur IAM et son conteneur d'initialisation de confiance. Configuration,
certificats, routes, règles réseau et autres services restent inchangés. Le Pod
IAM sera remplacé normalement ; ses données persistantes ne sont pas supprimées.
Une identité ou un état inattendu arrête l'opération, sans nouvelle tentative ou
annulation incertaine.

Après vérification de ce remplacement, le scénario Chrome existant sera repris
dans un profil neuf. Ni la construction ni la disponibilité ne constituent à
elles seules la preuve du parcours navigateur.

## Références techniques

Mise à jour : déploiement réussi en32.721s. Le nouveau Pod IAM est prêt ;25 règles
réseau,17Services et17autresdéploiements sont inchangés. La relecture infra des
pièces figées continue pendant la reprise du navigateur, sans accès concurrent
en mutation. Seules les deux références d'image prévues ont changé.

- Source propre7b8c1f95/SDK02e5, filtre de contexte versionné ecaf7773… conservé.
- Artefacts :a0318ec9… (78.422s, exit0) ; runtime :b6f140d1… (6.050s, exit0).
  IDs complets dans `.tmp/auth-iam-linux-8d8c3ae33c24436c8c4c017e21f3f879/results.json`
  et registreDocker29. Cache opaque UNKNOWN/ask conservé, pas de prune.
- Chargement par le garde Kind existant et alias contenu sha256 ; patch conditionné
  par UID/RV/template exact. UID avant du Pod377006f4… capturé.
- Patch sans sortie ACK JSON spécialisée : succès commande puis GET conditionné
  et comparaison du template ; ne pas présenter cette relecture comme l'ACK
  faisant autorité du contrat ingress, qui reste distinct.

Précision ultérieure : le descripteur d'image de Docker relie directement
le manifeste b6f140… à sa configuration33e39…, cette dernière étant l'identifiant
rapporté par le Pod. La terminologie initiale du ledger inversait ces deux rôles,
pas leurs valeurs. Le lien est maintenant archivé dans engine-image-link.json ;
aucune égalité d'identifiants différents n'est revendiquée.
