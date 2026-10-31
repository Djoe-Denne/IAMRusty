# Préflight intégré : tests simulés réussis, usage réel encore suspendu

La vérification fermée des outils est intégrée. La suite complète du candidat
donne44tests réussis ; les deux nouveaux scénarios passent aussi dans le dépôt
canonique. Le contenu intégré est identique au candidat après normalisation des
fins de ligne. Les opérations existantes restent inchangées.

Cette tranche vérifie seulement quatre commandes de version ou présence de
modules, sur les deux nœuds dont l'identité est attestée. Elle ne lit pas encore
les connexions ni les règles de rejet. Les sorties sont bornées pendant leur
capture ; les refus et erreurs ne révèlent pas leur contenu privé.

La revue sécurité indépendante porte sur cette dernière photo avant livraison et
usage réel. La présence des outils et le respect du budget de30secondes sur les
vrais nœuds ne sont pas encore établis. Un succès des tests simulés ne donne donc
aucune preuve du trajet ni autorité pour modifier la règle réseau.

Aucun accès distant nouveau, module, conteneur, image ou outil installé par ces
tests ; aucun certificat ni droit réseau changé.

## Preuves

-44PASS, aucun ignoré, suite du candidat ;2PASS ciblés après intégration.
- AST et contrôle de diff réussis ; traces sous .tmp/auth-envoy-tuple-preflight-*.
- Contrat ratifié : local-full-envoy-iptables-evidence.md, première tranche seule.
- Contrôle certificat et parcours navigateur toujours non exécutés.
