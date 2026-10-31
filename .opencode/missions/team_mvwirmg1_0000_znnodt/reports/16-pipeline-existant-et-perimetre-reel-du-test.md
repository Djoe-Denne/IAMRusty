# 16. Pipeline existant et périmètre réel du test

## Réponse au problème soulevé

L'attente de l'utilisateur est correcte : consommer le pipeline local-full déjà livré, obtenir la cible réelle, puis exécuter les scénarios k6 Windows contre son entrée18080. Cette mission ne devait pas devenir un nouveau chantier de plateforme ou une requalification globale du pipeline. La différence entre pipeline existant et raccord k6 auth manquant n'excuse pas une orchestration trop lourde.

Un échec de l'invocation de campagne n'établit pas que le pipeline est inutilisable. Ici, la cause précise de l'erreur Python du déploiement n'a pas été conservée : le wrapper PowerShell du lead a interrompu la capture sur stderr. C'est un défaut de conduite et de diagnostic à reconnaître, pas une preuve de défaut du produit ou de l'ensemble du pipeline. Aucun correctif applicatif n'est justifié sur cette base.

## Faits nouveaux depuis le bilan15

- La commande de déploiement a échoué, après création enregistrée de122ressources ; son succès final n'est pas acquis.
- Les trois nœuds de campagne ont été arrêtés gracieusement par le traitement d'échec, états exited vérifiés. Aucun retrait ou prune.
- Ils ont été redémarrés sous le même bail pour un diagnostic strictement en lecture seule, IDs complets enregistrés comme restarted. Aucun nouveau cluster ou déploiement relancé.
- Le premier contrôle après démarrage a échoué sur l'appel API ; un contrôle ultérieur passe avec le même ancrage. Ce refus transitoire n'était pas un changement d'identité démontré.
- Lecture gardée :18Deployments présents et deux Jobs de bootstrap réussis. Lors de cette lecture, plusieurs disponibilités étaient encore en cours de remontée. Observation ultérieure ciblée : IAM et Lazaret Ready. Cela ne prouve pas encore la disponibilité complète ni le trajet18080.
- Aucun scénario auth, navigateur ou stimulus mauvais certificat exécuté. Cause initiale du déploiement toujours inconnue ; pas de deuxième déploiement à l'aveugle.

## Ce qui constitue le test

Le scénario navigateur utilise l'entrée IAM fixe18080 et suit le fournisseur synthétique déjà prévu sur18081. L'inscription, la nouvelle connexion, la réassociation et les refus sont bien dans le test k6, pas remplacés par une qualification de Kubernetes.

Les seules opérations externes spécifiques au contrat sont le changement/restauration du certificat, les observations des compteurs et la durée de vie du forward. Ces opérations ne doivent pas être confiées à k6 avec des privilèges système ; elles doivent réutiliser les gardes existantes du parent. Leur nécessité ne justifie pas une reconstruction générale du pipeline ni une nouvelle suite de tests de son framework.

## Suite bornée

Ne pas ajouter de refonte, nouvelle fixture ou revue générale. Vérifier l'état effectif et le trajet du laboratoire par les mécanismes existants, isoler tout blocage concret sans modifier le produit, puis lancer les scénarios intégrés. Garder distincts : commande de déploiement échouée, disponibilité observée après reprise, et verdict auth encore absent. Si une réparation dépasse cette borne, présenter le problème précis plutôt qu'étendre silencieusement la mission.

## Preuves

Handle de déploiement sh_192f94c110018CBaCt5zgUc6ey exit1. Logs initiaux vides ; défaut de capture documenté. Registre .tmp/docker-resources-20261031.log : stops puis redémarrages des mêmes trois IDs. Script diagnostic .tmp/20261031-inspect-failed-deploy.py : arrêt au target.check initial, pas reprise du déploiement. Contrôle CLI ultérieur exit0/stderr0. Diagnostic gardé .tmp/20261031-diagnose-iam-readiness.py exit0 : IAMReadytrue, LazaretReadytrue, un redémarrage chacun. Les métadonnées de cette reprise ne constituent pas une cause du premier échec.
