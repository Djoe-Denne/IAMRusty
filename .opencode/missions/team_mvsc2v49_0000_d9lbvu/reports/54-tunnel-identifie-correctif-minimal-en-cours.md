# Tunnel identifié : correctif minimal en cours

Les métadonnées confirment que l'adresse mesurée appartient au tunnel interne
IPIP du nœud d'entrée, et à lui seul dans ce laboratoire. Ce n'est ni son adresse
physique ni une adresse VXLAN. La liaison au nœud possédé est vérifiée ; aucune
lecture refusée n'a été contournée pour l'obtenir.

Après lecture du contrat de l'architecte et des preuves, j'ai ratifié le correctif
borné : autoriser cette seule adresse, sur le seul port d'Envoy nécessaire. Le
refus par défaut, le client interne légitime, les autres règles et tous les
contrôles d'authentification et TLS restent inchangés.

L'implémentation est confiée à un écrivain isolé. Elle devra refuser une preuve
ou une adresse incohérente, protéger la modification contre les changements
concurrents, et permettre de retirer seulement sa propre règle si la validation
échoue. Aucun changement réseau n'est encore appliqué.

La qualification du rejet reste une preuve équivalente bornée au laboratoire,
pas une trace par paquet ou une garantie production. Le contrôle après correction
sera effectué sur la même entrée et la même route, sans substituer un autre trajet.

## Périmètre et preuves

- Modification prévue : une règle de source unique/TCP10000 dans allow-envoy.
- Deux fichiers de code/tests seulement ; pas de refonte ou nouvelle infrastructure.
- Contrat ratifié : local-full-envoy-nodeport-ingress.md ; avis causal motivé
  distinct des artefacts de capture qui conservaient leur verdict en attente.
- Tests ciblés et revue du code précèdent la mutation ; contrôle infra ensuite.
