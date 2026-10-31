# 6. Recréation approuvée, préflight sans démarrage

## Objectif et étape

La révision2 autorise maintenant la recréation du seul laboratoire local-full existant avec OAuth. Le portage des scénarios et du parent continue. Aucun nœud ne démarre avant leur intégration et les contrôles pertinents ; aucune qualification Linux du lanceur n'est ajoutée.

## Résultats nouveaux

Les dix images applicatives du profil sont présentes en Linux AMD64. Le tag courant IAM pointe toutefois vers une source antérieure au correctif OAuth qualifié. L'image IAM du run qualifié est encore disponible par identifiant immuable, avec la révision attendue. Je retiens cette candidate pour vérification de provenance et déploiement ; aucun tag partagé n'est déplacé et aucun build n'a été lancé. Les autres images restent à rapprocher des sources effectivement retenues, pas à qualifier par leur seul nom.

Les outils natifs requis sont localisés : Python, PowerShell, kubectl, Kind et Git Bash ; PyYAML est disponible. OpenSSL est présent dans Git, sans installation supplémentaire ni modification globale de PATH. Cela vérifie leur présence, pas encore toute la campagne.

## Correction minimale indispensable au mandat

La commande de bootstrap existante ne passait pas `--retain` à Kind. L'aide du binaire présent indique que cette option conserve les nœuds si la création échoue ; elle est nécessaire ici pour ne pas laisser Kind nettoyer automatiquement une création échouée alors que la mission exclut les suppressions.

Modification limitée à deux lignes de bootstrap : option appliquée au seul cluster aiforall-local-full, comportement legacy inchangé. Une assertion est ajoutée dans le test de bootstrap déjà existant, sans nouvelle fixture, serveur ou harness. Le test unique est d'abord **ROUGE** (option absente), puis **VERT** après correction :1test, zéro échec. Aucune CLI Docker/Kind réelle n'est appelée par ce test à commandes simulées. Ce n'est ni un défaut produit d'authentification ni une campagne de robustesse du framework.

`git diff --check` est sans erreur. Aucun test des17IT ni suite générale supplémentaire. Aucun appel applicatif, navigateur, k6, création de ressource Docker, build, mutation du cluster ou lecture Secret.

## Suite

Rapprocher les images immuables et préparer la fiche de commandes/bail neufs. Attendre les livrables intégrés, réaliser la revue bornée des garanties modifiées, puis bootstrap et déploiement existants. Vérification infra indépendante ciblée avant les scénarios auth. Les changements de sécurité ne servent pas à ajouter une nouvelle architecture ou des tests de nos tests.

## Annexe technique

Révision2/hash481e04fb97be0a61b0424712f95cf373b95f544091dc8805491507ae47b2b00c. Images courantes étiquetées sourcecf02c4a6c5f7501ffa3eb99764817d8221a8befd ; différences pertinentes IAM avec HEAD : oauth_service.rs, token_service.rs, setup/app.rs et deux tests. Image IAM qualifiée trouvée : IDsha256:b6f140d16558ae9b9dae53a4944e897226952ab91b8b09d02f9dcff69fc6d960, tag auth-7b8c1f9-20261029 et tag sha256-b6f140..., label révision7b8c1f95faa7c5db8d7aa9bcfbfb37fd8cb56ccd. Le digest config historique33e39... n'est pas présent comme image inspectable ; ne pas le confondre avec l'ID Engine/manifest vérifié.

Delta source : ops/deploy/local_full_target.py bootstrap condition local-full et --retain ; test existant ops/tests/test_local_full_target.py:AnchorTests.test_pending_creation_attests_endpoint_then_seals_uid. Commande exacte native depuis ops/tests : `C:/Python313/python.exe -B -m unittest test_local_full_target.AnchorTests.test_pending_creation_attests_endpoint_then_seals_uid -v`. Avant : exit1,1FAIL/0.027s ; après : exit0,1PASS/0.020s. Deux erreurs de syntaxe du script Context Mode ont précédé le premier run réel ; corrigées sans panne MCP et sans test exécuté dans ces essais invalides. Aucun autre contrôle du framework introduit.
