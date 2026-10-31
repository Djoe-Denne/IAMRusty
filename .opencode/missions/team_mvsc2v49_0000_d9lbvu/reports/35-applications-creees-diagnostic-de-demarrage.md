# Applications créées, diagnostic de démarrage en cours

Les deux jobs de bootstrap ont réussi, puis les applications ont été créées.
Le déploiement s'est arrêté sur sa première attente applicative : le contrôleur
Apparatus n'est pas admis par la politique de sécurité du laboratoire, car il
demande un volume du système hôte. Cette politique reste active et inchangée.

Ce contrôleur n'est pas nécessaire aux preuves d'authentification prévues.
Je ne lance donc pas de refonte de stockage et n'affaiblis pas la politique pour
le démarrer. Ce défaut du profil complet restera explicitement signalé ; aucun
succès global du laboratoire ne sera déclaré.

La lecture de l'état montre aussi qu'IAM, Envoy et Manifesto quittent au démarrage.
IAM et Envoy bloquent directement les preuves restantes. Je localise leurs erreurs
sans relancer les jobs ni les applications, ni contourner le contrôle de cible.
Le support de diagnostic doit rester borné, en lecture seule et sans secrets.

Plusieurs autres services sont prêts, dont Hive, Telegraph, le contrôle des
permissions, le connecteur GitLab et le simulateur d'identité. Cela ne constitue
pas encore une preuve de bout en bout.

## Preuves et limites

- Bootstrap : fga-bootstrap et openbao-seed de la campagne terminés avec succès.
- Applications appliquées, attente interrompue sur apparatus-operator.
- Refus d'admission : politique `baseline`, volume hôte `admission-store`.
- IAM, Envoy, Manifesto : CrashLoopBackOff observé, cause non encore établie.
- Aucun test Envoy/navigateur exécuté, aucun job ou secret recréé.

## Diagnostic sans contourner la garde

L'architecte confirme qu'une lecture de journaux strictement bornée est compatible
avec les frontières existantes. J'ai ratifié ce delta opérationnel et confié à
un seul auteur la garde et ses tests, en isolation. La lecture exigera les
identités exactes du namespace et du Pod avant capture, puis une vérification
du Pod avant toute restitution ; aucune sortie brute ou donnée secrète en rapport.

Aucun accès générique, droit Kubernetes supplémentaire, nouvelle plateforme ou
changement de cible n'est autorisé. Les tests seront exécutés après intégration,
avant le diagnostic réel. Le résultat Envoy demeure explicitement non exécuté.

Le candidat final passe les 35 tests de la suite, sans appel au cluster. La revue
de sécurité du code exact est en cours avant sa première lecture réelle. Ces
tests portent sur la garde de diagnostic, pas sur le démarrage des applications.
