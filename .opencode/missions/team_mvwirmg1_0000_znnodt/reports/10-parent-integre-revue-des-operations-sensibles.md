# 10. Parent intégré, revue des opérations sensibles

## Objectif et étape

Le raccord Windows complet est maintenant intégré : inspection des quatre cas, admission du nouveau bail, témoin bon certificat, mauvais nom puis restauration, et parcours navigateur corrélé. Il est encore non qualifié en exécution réelle.

## Ce qui a changé et été vérifié

Cinq fichiers récupérés sont identiques byte-à-byte au candidat. La précision documentaire --patch-file était déjà intégrée et n'a pas été écrasée. L'auteur rapporte parsing PowerShell, ASTPython et compilationC# sans appel natif réussis ; le lead a vérifié les identités et le hook fmt du commit. Pas de relecture ni test systématique du framework.

Le code utilise le même identifiant de campagne que les inputs du fournisseur, ferme le forward avant les snapshots Admin, conserve un schéma fermé de26checks navigateur et2parphaseSAN, et refuse les résultats incomplets. Ces propriétés présentes statiquement ne prouvent pas le fonctionnement réel des acquittements, compteurs ou du cleanup.

## Vérification indépendante justifiée

Le nouveau delta manipule des certificats privés et introduit l'attente asynchrone du forward dans le mécanisme de processus existant. Une revue sécurité fresh est donc ciblée sur ces opérations : propriété/admission, confidentialité, mutation/restauration/acquittements, processus/jonction et conditions de PASS. Les anciens mécanismes ne sont examinés qu'aux points d'appel utiles, sans audit global ni nouvelles fixtures.

La création du laboratoire reste derrière cette revue et les contrôles requis. Les scénarios JS ont déjà leur PASS de parité statique borné, sans extension au parent ou au runtime. Aucun lancement auth, build, nœud ou ressource Docker nouveau dans cette étape.

## Annexe

Commit local51e74e1, diff0c1e21d..51e74e1 :5fichiers876insertions/9suppressions ; candidat result_mvwm2qtr_0002_v70vxq, même auteur/session parent-auth-windows. Pas de commit/push auteur ; SDK absent dans son arbre, d'où récupération canonique et commit au hook normal sans contournement. Source JS/produit/summary inchangée. Briefing .cursor/review-briefings/20261031-k6-auth-parent-security.md. Publication distante et nettoyage final encore à faire.
