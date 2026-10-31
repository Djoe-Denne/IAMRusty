# 14. Laboratoire créé, déploiement en cours

Le laboratoire autorisé a été créé avec trois nœuds et un ancrage neuf. Le contrôle de cible existant passe ; le kubeconfig privé et l'identité du namespace système sont présents. Les trois IDs complets sont enregistrés dans le registre de la campagne.

Les dix images applicatives sont épinglées et leurs identifiants vérifiés. IAM utilise l'image historiquement qualifiée, pas le tag courant antérieur au correctif. Aucun build ou pull applicatif n'a été lancé et aucun tag partagé `latest` déplacé.

Le déploiement du profil existant avec OAuth est maintenant en cours. Les commandes et journaux restent privés. En cas d'échec, le parent tente l'arrêt gracieux des trois nœuds possédés sans supprimer les ressources. Les volumes apparus pendant la création restent prudemment classés à examiner, pas autorisés à supprimer.

## Ce qui reste à prouver

Le déploiement n'est pas encore terminé. Readiness, identité des services et trajet Windows → Envoy → IAM/connecteur/fournisseur seront vérifiés indépendamment avant tout appel auth. La campagne fonctionnelle, le mauvais certificat, la restauration réelle et le nettoyage final ne sont pas encore qualifiés.

## Annexe technique

Bootstrap exit0 ; trois IDs dans le bail neuf, anchor check exit0/stderr0. Index privé `.tmp/k6-auth-runtime-20261031-d00ab08-index.json`. Registre `.tmp/docker-resources-20261031.log` : événements de création opaque puis trois IDs observés et trois deltas de volumes classés ask.

Épinglage exit0, dix images vérifiées, image IAM b6f140d… conservée. Déploiement via `.tmp/20261031-deploy-auth-local-full.ps1`, handle `sh_192f94c110018CBaCt5zgUc6ey`. Aucun verdict auth déduit de ces opérations.
