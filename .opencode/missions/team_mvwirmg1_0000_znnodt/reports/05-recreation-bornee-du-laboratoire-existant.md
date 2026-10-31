# 5. Ce qu'il faut recréer pour la campagne Windows

## Objectif et changement autorisé en chat

Vous avez autorisé la recréation du laboratoire disparu et demandé ce qu'elle implique. La proposition consiste à réutiliser le profil de déploiement existant, pas inventer un laboratoire réduit, un nouveau framework ou une autre architecture. Le code des scénarios et du parent continue sous la révision1 ; la création réelle attend la révision native de politique et les préconditions finales.

## Ressources concrètes

1. **Un cluster Kubernetes local Kind**, c'est-à-dire Kubernetes dans des conteneurs Docker : trois nœuds, un control-plane et deux workers, nom aiforall-local-full, contexte kind-aiforall-local-full. Configuration existante, image de nœud épinglée, réseau Calico déjà prévu par le bootstrap. API seulement sur127.0.0.1, port attribué et observé, pas exposition publique.
2. **Le profil local-full existant avec oauth-proof** : IAM, passerelle Envoy et contrôle d'authentification, vrai GitLabConnect et fournisseur synthétique existant ; PostgreSQL et dépendances du profil. Le déployeur partagé inclut aussi les composants de plateforme déjà définis (Hive, Telegraph, Manifesto/Lazaret, synchronisation, OpenFGA, LocalStack, Redis, OpenBao, registre OCI et composants Apparatus). Ils ne deviennent pas une campagne HA/Manifesto/Apparatus ; leur présence vient du profil existant. Ne pas créer une nouvelle variante de déploiement pour les supprimer dans cette mission.
3. **Les données locales neuves nécessaires** : bases/migrations/bootstrap du profil, stockage local prévu, certificats de campagne et configuration privée OAuth/HMAC. Les secrets ne sont ni récupérés par GET sur un ancien cluster ni publiés ; le test mauvais nom réutilise une même autorité de certification locale puis restaure le bon certificat.
4. **Deux accès locaux Windows** :127.0.0.1:18080 vers Envoy et localhost:18081 vers le fournisseur synthétique, avec le mécanisme local existant. L'autorité IAM8080/iam et les callbacks18080 ne changent pas. k6, Chrome et le parent restent natifs Windows, hors cluster.
5. **Un bail et un inventaire neufs** : kubeconfig privé, IDs des conteneurs et UID de ressources observés, images immuables liées aux sources. Réutiliser les images présentes si vérifiables ; les images manquantes nécessaires pourront être construites pour Linux via les Dockerfiles existants, pas un build Rust de développement dans Docker.

## Limites et ordre

Pas de cluster tiers, nouvel IdP, frontend, tunnel distant, exposition publique, contournement TLS/sandbox, purge ou reset. Pas de qualification du runner Linux. La préparation du laboratoire est documentaire/statique tant que les auteurs n'ont pas fini et que les gates IT requis ne sont pas satisfaits ou valablement réutilisés. Ensuite : création avec bail neuf, déploiement gardé, vérification infra indépendante ciblée, campagne auth, restauration et stop gracieux des seules ressources possédées. Docker/WSL étrangers restent préservés.

Le profil existant est plus large que les seuls composants auth ; cela a un coût de ressources, mais aucune mesure actuelle ni estimation fiable de durée n'est disponible. Je recommande sa réutilisation telle quelle pour éviter une nouvelle architecture hors mission.

## Politique

La révision1 interdisait la création : le draft est amendé uniquement pour cette recréation bornée, le contrôle infra requis et ses baux/cleanup. OpenAI uniquement, QA proportionate, pas long_run ni classifieur ajoutés, pas tests de nos tests. Faire afficher `/ensemble-mission config` puis approuver la révision avec `/ensemble-mission start` avant l'admission de nouvelles tâches de provisionnement. Les tâches de code déjà admises conservent leur révision et continuent ; aucun nouveau runtime démarré ici.

## Sources vérifiées

ops/deploy/kind/cluster-local-full.yaml:1–22 ; ops/deploy/apps/overlays/local-full/README.md:11–128 ; namespaces.yaml:1–32 ; oauth-proof/kustomization.yaml:1–53 ; métadonnées/images de resources.yaml ; bootstrap-local-full.ps1:1–7 ; deploy-local-full.py options798–839 ; local-full-images.py:9–16. Le profil comprend trois namespaces : plateforme, Apparatus, plugins ; aucun besoin d'activer les scénarios plugin isolés.
