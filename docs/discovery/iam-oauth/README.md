# Pilote — catalogue fonctionnel IAM/OAuth

**Proposed, 12 novembre 2026.** Photographie du code à
`f7b557f76920f47650997062bd23e335b6a85be9`. Ce pilote extrait des comportements
existants ; il ne décide pas des exigences futures, n'accepte aucune ADR et ne
qualifie pas le produit. L'extraction initiale était statique ; la caractérisation
Q01/Q02 ci-dessous ajoute des validations ciblées, pas un PASS global du catalogue.

## Résultat utilisable

Le [catalogue JSON](catalogue.json) propose **un domaine, quatre fonctionnalités
et treize critères**. Les IDs `IAM-OAUTH-C01` à `C13` sont des propositions
stables à valider, pas de nouveaux identifiants canoniques imposés aux ADR.
Chaque critère relie code, décision et assertions effectivement lues. Les chemins,
symboles et lignes sont dans les registres `sources` et `tests` du JSON.

[Ouvrir la vue interactive locale](http://127.0.0.1:4317/?canvas=iam-oauth-discovery-pilot-b9f7d617f6e7)
ou consulter sa [source versionnée](pilot.canvas.tsx). La publication et la
compilation TypeScript sont vérifiées ; le rendu visuel n'a pas été contrôlé
dans un navigateur, aucun outil navigateur n'étant disponible dans cette session.

Les quatre fonctionnalités proposées sont :

- **F01 — Démarrer un parcours OAuth** : redirection/state, cookie et PKCE.
- **F02 — Autoriser un retour navigateur une seule fois** : liaison, concurrence
  writer et échec de stockage sans effet aval.
- **F03 — Créer ou retrouver le compte plateforme** : inscription/complétion,
  session, email requis au LOGIN et priorité à l'identité déjà liée.
- **F04 — Lier ou relier une identité à un compte** : cible autorisée,
  absence d'email admise en LINK/RELINK et autorité différente au START/callback.

Les treize critères ont une correspondance avec des assertions unitaires/IT
examinées. **Cela ne signifie ni treize PASS actuels, ni100% de couverture.**
La sélection concerne une tranche déjà riche en tests : elle ne permet pas
d'estimer un taux global ni de conclure à l'absence d'exigences sans test.
Six ont aussi une correspondance avec des checks k6, dont deux partielle. Les
sept autres ne sont pas déclarés « sans test » : leur preuve sélectionnée est en
IT. Le bon niveau de test dépend du risque ; dupliquer toute IT en E2E n'est pas
un objectif de ce pilote.

## Ce que le pilote révèle concrètement

**Q01 — Oracle actuel caractérisé, sans changement produit.** LOGIN sans email
retourne **401 / authentication_failed** : `UserProfileError` devient
`provider_error` dans le mapper applicatif, puis une erreur publique d'authentification.
Une `RepositoryError` devient `database_error`, puis **500 / login_failed**.
Deux tests unitaires distincts traversent ces conversions et vérifient aussi la
non-divulgation des causes. L'IT sans email exige désormais exactement401 et
`authentication_failed`, sans credentials ni effets métier. Le test500 est une
preuve de mapping, pas une injection de panne DB dans le parcours HTTP.
Les transactions invalides restent des400 ; la panne writer lors de leur
consommation est un503, couvert par une autre IT : ces cas ne sont pas interchangeables.

**Q02 — Oracle actuel200.** La route et son IT retiennent200 pour la complétion.
Le check k6 `registration-completed` exige maintenant200, sans tolérance201.
Le prédicat réel a été exercé hors navigateur :200 accepté,201 refusé ; cela
ne remplace pas une nouvelle exécution Chromium/k6.

**Q03 — Les frontières des preuves comptent.** La race entre deux listeners est
vérifiée par une IT Postgres ; les refus403 d'Envoy ne remplacent pas les refus400
de liaison transactionnelle IAM. Le test k6 décode les claims JWT, tandis que
l'IT sélectionnée vérifie aussi signature/JWKS et refresh persisté. Aucun de ces
raccourcis ne doit devenir « OAuth entièrement couvert ».

**Q04 — Classification humaine restante.** Un critère décrit une attente
observable, pas chaque fonction Rust. Il faut encore valider le vocabulaire,
la granularité et les obligations manquantes avec le responsable produit.

## Méthode et limites

Essai **adapté, code-only** du skill
[DiUS codebase-discovery](https://github.com/DiUS/agent-toolkit/tree/ce859b342b9451a42d3a25e4628d0be039441d24/skills/codebase-discovery) :
préflight, exploration ciblée, extraction avec preuves, comparaison documentaire,
vérification des assertions et registre des questions. Pas d'installation globale,
d'onboarding complet, d'interview métier ni de revue indépendante déjà acquise.

Localisation inconnue : recherche hybride Rust MCP `local-qwen3-8b` ; symboles et
corps courants : Serena. Certains résultats hybrides avaient d'anciens noms/spans ;
les pointeurs du catalogue viennent des sources courantes, pas de ces résultats.
QMD a fourni les décisions liées ; les ADR et contrats canoniques ont ensuite été
lus dans le dépôt.

Ce n'est pas un inventaire exhaustif d'IAM ou d'OAuthConnect. Password/refresh,
cleanup, toutes les variantes invalides, OIDC/JWKS interne, fournisseurs réels et
cutover restent hors analyse. Les tests IT sélectionnés utilisent Postgres/HTTP
IAM réels et un Connect WireMock ; cela ne prouve pas le fournisseur externe.
La qualification k6 historique citée porte sur la slice locale à54a19ba, pas sur
une exécution nouvelle, Linux/CI, le lab nominal ou la production.

## Comment utiliser les ADR ensuite

1. Une nouvelle décision produit fournit des **critères candidats**, avec ses
   paragraphes de décision et ses exclusions. Les choix purement techniques ne
   deviennent pas artificiellement des fonctionnalités utilisateur.
2. Conserver séparément **décidé**, **observé dans le code**, **assertion présente**
   et **résultat d'exécution**. Lire les actualisations et supersessions partielles :
   `Accepted` n'est pas un PASS et `Partial` n'est pas « rien n'est livré ».
3. Rattacher les nouveaux tests aux IDs validés, puis seulement générer leurs
   labels Allure `epic`, `feature`, `story`. Ce JSON est un prototype de liaison,
   pas un format Allure natif ni une intégration existante.
4. Conserver le catalogue indépendant des résultats pour pouvoir afficher les
   exigences sans test, non exécutées ou partiellement vérifiées. Ne pas créer
   de faux tests PASS/SKIP pour peupler Allure.

La vue associée résume le catalogue ; le JSON reste la source des relations et
ce document explicite leurs limites. Les données et chemins privés sont exclus.

## Vérifications du livrable

- JSON parsable, IDs uniques et références résolues (voir les registres du JSON).
- Chemins, plages de lignes, symboles et noms des checks k6 vérifiés ; les fichiers
  de preuve sont inchangés par rapport au commit source.
- Treize critères et leurs titres/features/relations k6 concordent entre JSON
  et canvas ; compilation TypeScript stricte sans erreur.
- Deux caractérisations unitaires exécutées :2PASS,0échec,62autres tests filtrés.
- Binaire IT `oauth_browser_transactions` :17PASS,0échec/ignoré/filtré ; run
  `it-d4364c436ed149d5900782655f6a2ee4`. Les cas sans email401, complétion200,
  refus transactionnels400 et panne consume503 ont effectivement été exécutés.
  Fixture possédée :1créée/1retirée/0running ; trois conteneurs préexistants
  conservés. Preuves locales privées : `.tmp/itfixtures-it-d4364c436ed149d5900782655f6a2ee4/`,
  registre append-only `.tmp/docker-resources-20261112.log` ; aucun log brut publié.
- Aucun faux résultat Allure, changement de code métier, instrumentation LLVM
  ou lecture d'entrée privée. L'IT utilise uniquement les fixtures du lanceur natif.
