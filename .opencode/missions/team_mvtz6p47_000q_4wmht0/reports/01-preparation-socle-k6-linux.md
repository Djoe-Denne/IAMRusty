# Préparer un smoke k6 fiable sous Linux

## Objectif et étape

Livrer le socle k6 et un petit smoke HTTP réel, avec qualification navigateur séparée. Nous sommes en préparation : aucun auteur, test, installation ou runtime lancé. Le plan ne porte ni OAuth ni charge.

## Évolution et faits vérifiés

L'utilisateur a désigné Linux natif comme cible, autorisé WSL/Docker en local et GitHub Actions pour la qualification. Le brouillon conserve le catalogue présenté, sans classification, score manuel ou délégation long run ajoutée. La QA proportionate est précisée avec dix commandes proposées et quatorze critères ; aucun résultat de ces commandes n'est prétendu acquis.

Le code fournit GET /iam/health avec un corps stable OK. Les gardes ont des branches POSIX mais leur fonctionnement réel depuis Linux n'est pas démontré. Le workflow GitHub existant fait des builds/tests Linux et du rendu hors ligne ; il ne fournit pas de laboratoire k6. La dernière campagne auth est archivée et ses 23 worktrees retirés ; les autres anciens worktrees ne sont pas à supprimer pour cette mission.

## Proposition

Un auteur pour le socle et la CI, puis une revue indépendante ciblée sur les frontières cible/baux/confidentialité/permissions. Vérifications statiques, négatifs discriminants et Chromium séparé. Le smoke atteint uniquement un vrai service autorisé, avec exactement un cas, HTTP200 et corps OK. Les preuves HTTP, Chromium et cleanup sont distinctes.

Le contrat V1 doit être ratifié avant code ; architecte si modification structurelle. La première étape fixe le raccordement Linux sans adopter un ancien bail ni créer de déployeur.

## Limites et choix

Le runner local peut être WSL ; Docker non privilégié reste un repli permis. Le runner GitHub ne rejoint pas le loopback du laboratoire Windows. Le plan ne suppose donc pas une cible disponible dans la CI : elle peut qualifier l'outillage/Chromium indépendamment, mais aucun PASS HTTP/global sans cible Linux réelle ancrée. Si elle manque, le smoke reste INCONCLUSIVE et la vérification sur Linux démarré peut fournir la preuve finale. Un nouveau laboratoire CI est une extension à arbitrer, pas un détail caché.

Nous recommandons de conserver proportionate : light ne dispense pas des négatifs/cleanup obligatoires ; reinforced ajouterait un plan et une revue plus explicites sans besoin actuellement établi. Aucun coût ou délai d'exécution CI estimé ici.

## Décision attendue

Contrôle documentaire réellement exécuté : quatre fichiers décodés UTF-8 strict, aucun caractère de remplacement ni espace final ; policy JSON valide, 14 entrées catalogue, dix commandes et quatorze critères ; git diff --check exit0. Ce dernier contrôle ne couvre pas les nouveaux fichiers non suivis, vérifiés séparément par lecture UTF-8/espaces. Aucun test fonctionnel.

Corriger le périmètre ou la QA dans le chat si nécessaire ; sinon afficher le brouillon corrigé par /ensemble-mission config, puis seul l'utilisateur peut /ensemble-mission start pour libérer l'exécution de la révision présentée. Cette préparation ne ratifie pas automatiquement ADR0504 et ne lance ni GitHub Actions ni déploiement.

## Références techniques

- Plan complet : ../prepared-plan.md.
- Policy réelle : ../model-policy.json, verification.level=proportionate, dix commandes/quatorze critères ; version2, startingIntelligence50, catalogue/providers inchangés.
- Code : services/IAMRusty/http/src/lib.rs:64, public_routes.rs:25 ; rustycog/rustycog-http/src/lib.rs:39 ; ops/deploy/local_full_target.py::private_file, approved_root, LocalClusterTargetAnchor.check/_uid_check.
- Historique : docs/contracts/20261029-auth-security-qualified-state.md ; docs/reviews/20261029-worktree-consolidation.md:77–121.
- CI : .github/workflows/ci.yml, lecture et recherche ciblée, aucun run.
- HEAD observé : c3c65965c2ffb2c4b6b01d5b2efce5a5edb9605d ; SDK02e5 ; WIP étrangers préservés.
