# Échec localisé au démarrage navigateur ; recherches avant chaque nouvel essai

## Résultat du dernier essai

Le push interrompu avait abouti : `origin/master` vérifié à f8df62f93f5262354339297dd4f9fa5354ded0c3, aucun processus git encore actif. Aucun push répété. Le run natif40968668360 est terminé failure : 70 tests Linux réussis, acquisition/versions/inspect réussis ; navigateur INCONCLUSIVE, cleanup des enfants PASS, publication skipped.

Le résumé du run courant est structurellement valide : un cas attendu, zéro démarré/achevé, une itération, zéro requête ; exit natif k6 non nul, aucun renderer possédé observé, lectures namespace non observées. Cela localise l'échec avant le corps du cas navigateur, sans établir sa cause. Aucun refus de namespace effectivement mesuré dans ce run.

## Nouvelle instruction utilisateur

L'utilisateur autorise autant d'essais utiles que nécessaire, avec réflexion et recherches Internet entre chaque essai : que se passe-t-il, pourquoi, cause connue et solutions documentées. Cette instruction remplace la limite locale de trois essais du plan de diagnostic. Pas de boucle aveugle ou remise à zéro du registre des tentatives ; après stagnation, hypothèse différente et recours spécialisé restent applicables. Aucun changement de modèles, niveau QA, objectif ou autorisation de contourner la sandbox/installations/déploiements n'en découle. Le workflow reste déclenché par publication approuvée, pas dispatch implicite.

## Ce qui est connu sur Internet

1. La documentation Chromium **156.0.8078.73**, `docs/security/apparmor-userns-restrictions.md`, décrit le refus possible des user namespaces pour les builds téléchargés sur Ubuntu23.10+. Un profil AppArmor attaché à un chemin installé peut autoriser Chrome stable sans autoriser Chrome for Testing extrait dans un répertoire privé. C'est une explication connue compatible avec notre contexte, **pas encore notre cause mesurée**.
2. Le retour Puppeteer13595 rapporte précisément « No usable sandbox » après migration22.04→24.04. La solution rapportée emploie un helper de sandbox SUID existant. Ce retour n'est pas une preuve k6 ni une autorisation d'installation privilégiée ici.
3. L'incident runner-images10443 confirme des refus `unshare` dans un autre outil sur Ubuntu24.04. Les contournements proposés incluent désactivation globale, conteneur privilégié et changement de distribution : ils ne sont pas adoptés automatiquement et certains contredisent nos gardes.
4. Le source k6v2.3.0 `browser/registry.go:303–331` construit le navigateur sur IterStart, avant le corps JS. `common/browser_process.go:208–278` peut perdre les lignes Chromium FATAL et retourner le premier ERROR, éventuellement secondaire. Cela explique pourquoi notre diagnostic public peut ne pas révéler la cause racine.
5. Les codes natifs ont une signification documentée dans `errext/exitcodes/codes.go` : thresholds99, configuration104, script107, abort108, etc. La catégorie actuelle NONZERO conserve le refus mais perd cette distinction ; un prochain diagnostic peut l'exploiter sans publier de texte natif.

## Décision de suite

Ne pas appliquer no-sandbox, sysctl global, profil/AppArmor privilégié, helper SUID ou changement de pin pour rendre le run vert. Préparer une expérience causalement discriminante : observer les prérequis de l'hôte sans les modifier et/ou la défaillance du binaire Chromium épinglé sans le filtre k6, sous le superviseur existant et avec projection fermée. La seconde voie doit être arbitrée avant exécution pour rester un diagnostic parent, pas un nouveau harness de qualification. Aucun quatrième essai n'est encore lancé.

## Sources

- https://github.com/Djoe-Denne/IAMRusty/actions/runs/40968668360
- https://chromium.googlesource.com/chromium/src/+/156.0.8078.73/docs/security/apparmor-userns-restrictions.md
- https://github.com/puppeteer/puppeteer/issues/13595
- https://github.com/actions/runner-images/issues/10443
- https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/browser/registry.go
- https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/common/browser_process.go
- https://github.com/grafana/k6/blob/v2.3.0/errext/exitcodes/codes.go

## État de livraison et ressources

Livrables source/revues publiés, source k6 canonique propre ; toutes les refs de mission inventoriées sont ancêtres de master. Le contenu utile des arbres modifiés a été récupéré et sauvegardé, pas fusionné fictivement par native merge. Worktrees pool3/pool4/pool7 et refs préservées encore présents, aucune suppression forcée/archivage acquis. Gitlink SDK02e5db23 inchangé, WIP étrangers préservés.

ARTIST-Ubuntu observé Stopped ; Docker Desktop étranger Running, son seul conteneur préexistant40201bda… inchangé Running/aiforall-local. Aucun conteneur créé/redémarré par la mission, aucun prune/stop étranger. Mission non clôturée : navigateur et HTTP non qualifiés, cleanup/archivage des arbres encore ouverts.
