# Proposition — préparations natives et suite k6 orientée produit

- Statut : **Proposed** ; Réalité : **inventaire documentaire, aucune migration/réécriture implémentée ni exécutée**.
- Date : 2026-11-11 ; base de continuité fournie par le parent : `8da70f7`.
- Mandat courant : comparer les options avant de coder. L'exigence initiale « garder les scénarios JS » est **désormais discutée**, pas un invariant. k6 reste l'outil ; les tests Rust restent intacts.
- Les contrats [Windows](k6-windows-functional.md), [auth Windows](k6-windows-auth-functional.md), [Linux](k6-functional-linux.md) et [ADR-0504 Accepted](../adr/0504-validation-fonctionnelle-linux-k6.md) conservent leur statut et leur portée historique. Cette proposition ne les abroge pas, ne ratifie pas leur remplacement et n'autorise aucun lancement.

## 1. Inventaire borné : comportements à considérer, pas code à porter obligatoirement

Les quatre scénarios du périmètre fourni par le parent ont été relus dans `ops/tests/k6/scenarios/` :

- **`smoke/iam-health.js`** : un GET `/iam/health`, origine strictement HTTP `127.0.0.1:18080`, statut200 et corps exact `OK`, un cas démarré/achevé. Aucun seed SQL, nouvelle clé ou navigateur requis par ce JS. Candidat simple pour le trajet **HTTPS** natif, pas preuve de fonctionnalités/authentification/readiness générale.
- **`smoke/browser-prerequisite.js`** : `about:blank`, évaluation navigateur et fermeture du contexte ; contrôle de version Chromium attendu côté Windows. Aucun lab, DB ou fournisseur requis. Distinguer ce prérequis des diagnostics historiques Linux/WSL, non obligatoires pour une suite Windows.
- **`auth/envoy-san.js`** : stimulus HTTP GitLab historique, trois phases good303/bad503/restored303. Le JS ne mute rien ; son parent devait discriminer un rejet SAN, constater la même CA et la même incarnation Envoy, puis restaurer. Ce sont des comportements/preuves candidats, **pas une obligation de reprendre exactement ces phases ou compteurs** dans une nouvelle suite. Un503 seul ne prouve toujours pas un rejet TLS/SAN.
- **`auth/oauth-browser.js`** : inscription via API, nouveau login, réassociation et négatifs ; 26 checks historiques. Origines HTTP18080/fournisseur loopback18081, issuer HTTP8080 et cookie non Secure sont codés en dur. Le nouvel utilisateur corrélé au run est inscrit par `complete-registration`, **pas préinséré en SQL**. Inventaire utile pour discuter fonctionnalités et erreurs, pas catalogue obligatoire de26 oracles.

`ops/tests/k6/lib/summary.js` projette checks et compteurs du run sans exporter les données brutes. Sa **fonction** reste utile : résultats frais, cas réellement exécutés et confidentialité ; conserver ce fichier ou produire un petit équivalent dépend de l'option retenue.

Le parent historique `ops/tests/k6/lib/windows-auth-ops.py` porte admission/snapshots/fournisseur/mutation/restauration/forward via `ops/deploy/local_full_target.py`. Il vérifie notamment UID, configuration et incarnation ; le patch Secret utilise un **fichier privé `--patch-file` supprimé en finally** (lignes248–268), pas un passage stdin démontré par cette source. Ces protections ne deviennent pas inutiles parce que Python est retiré.

## 2. Deux options ouvertes

**A — Réutilisation sélective.** Garder les JS réellement adaptés au produit actuel et déplacer leur préparation vers les outils natifs. Avantage : réutilisation des comportements et preuves pertinents. Coût : désolidariser origines HTTP, routes GitLab, ancien Secret partagé, contrôle ops et assumptions de profil aujourd'hui retiré. Ne pas conserver un scénario uniquement pour amortir son ancien coût.

**B — Petite suite JS réécrite, orientée produit.** Garder k6, choisir les comportements utiles du service actuel et écrire seulement leurs scénarios nécessaires, avec setup natif externe. L'ancien code sert d'inventaire, pas d'autorité à reproduire. Risques : oublier des négatifs importants ou supprimer trop tôt une preuve encore utile ; les rendre visibles dans la sélection des cas plutôt que promettre une couverture équivalente automatique.

**Recommandation : privilégier B si la comparaison des coûts le confirme**, avec réemploi ponctuel possible, par exemple du smoke ou de la projection de résultats. Comparer sur le même petit périmètre : utilité produit actuelle, adaptation des fixtures/identités/TLS, assertions positives et négatives nécessaires, droits de préparation, inverse de cleanup et maintenance. Ni le nombre de checks ni les lignes Python ne mesurent seuls ce coût. **Aucun GO de suppression ou de recodage n'est acquis.**

Candidat de première tranche à confirmer : **IAM**, avant Hive/Manifesto ou d'autres services. Préparer une petite sélection de comportements **fonctionnels / erreurs / authn-authz et isolation multi-tenant / replay / TTL**, en distinguant ce qui est réellement exposé et testable. Ce sont des axes de discussion, pas des scénarios déjà spécifiés ou une promesse de sécurité complète. Un smoke HTTPS peut servir de premier raccord technique sans se substituer à cette sélection.

## 3. Frontière cible commune aux deux options — non livrée

Interfaces **candidates**, pas commandes actuellement disponibles ni catalogue de profils ratifié :

```text
just k6-prepare <profil>
just k6-run <profil>
just k6-clean <profil>
```

- **Préparer** : scripts PowerShell Windows bornés par profil ; outils natifs `kubectl`, Helm, OpenSSL et, seulement si le cas le nécessite, CLI PostgreSQL/SQL explicite, OpenBao ou FGA existantes. Préparation DB/profil/clé strictement justifiée par les cas choisis. **Aucune fixture SQL à inventer ici**, pas de Terraform pour seed, plugin générique, serveur de contrôle ou machine d'état maison. Un petit relevé privé des effets possédés ne constitue pas une autorité auto-attestée.
- **Exécuter** : k6 produit stimuli/assertions ; pas de création Kind, seed, rotation, réparation d'infrastructure ou administration privilégiée depuis les JS. Le superviseur natif garde environnement privé, délais, collecte bornée et fermeture des descendants. Si un cas retenu exige un changement temporaire pendant son déroulement, il faut des étapes natives **fermées et spécifiques** : ne pas prétendre qu'un setup avant run suffit à porter l'ancien SAN mutateur.
- **Nettoyer** : rétablir seulement les changements autorisés de la campagne et fermer ses propres processus/forwards sur succès, échec ou interruption. Une restauration échouée est un échec distinct ; jamais PASS global fondé seulement sur k6 exit0.

Entrées explicites communes : profil fermé choisi, run courant, répertoire privé et chemins de configuration/CA, cible autorisée et ressources concernées. Résultats séparés : préparation, assertions réellement exécutées, cleanup. Aucun résumé ancien, cas absent, compteur vide ou test empêché n'est transformé en succès.

## 4. Prérequis et limites à résoudre avant implémentation

1. **Cible actuelle, pas ancien profil supposé.** Le launcher natif expose le trajet HTTPS18443, sépare deux CA et les Secrets par service ; les connecteurs sont désactivés. L'ancien chart/profil `oauth-proof` a été retiré. Les origines, cookie et issuer des JS auth ne deviennent pas compatibles par simple remplacement de chaînes. Pour une option auth retenue, définir d'abord un **profil test dédié**, ses routes/fournisseur et les destinataires de chaque CA ; ne pas réintroduire une exposition HTTP ou affaiblir TLS du lab courant pour sauver le scénario.
2. **Lab non admis pour cette campagne.** La dernière section du ledger opérateur rapporte trois nœuds running sous keepalive mais API Kubernetes refusée, sans port public6443. C'est un prérequis bloqué, pas une disponibilité fraîche prouvée. Aucune réparation, exécution k6 ou mutation SAN autorisée ici. La mise à disposition de Kind reste une étape opérateur native distincte et explicitement autorisée ; `k6-run` ne réveille pas le lab.
3. **Identités et données minimales.** Réutiliser les configurations produit valides via leurs chemins privés, sans rotation implicite de clés/tokens ni copie de CA privées dans les consommateurs. Le smoke n'exige pas de nouvelle DB/clé ; l'ancien OAuth crée son identité par API. Pour un futur cas nécessitant des données, identifier le mécanisme produit ou le SQL précis et son inverse **avant** de coder la préparation. Pas d'insertion destinée à contourner l'inscription ou l'authentification à tester.
4. **Effets métier et cleanup.** Inscription, événements ou identités persistantes ne disparaissent pas parce que les processus s'arrêtent. Choisir une API d'inverse existante et bornée, ou une rétention locale explicitement acceptée ; sinon signaler le gap et différer le cas. Aucune suppression SQL globale, aucun drop/reset de lab.
5. **Outils.** Les sources historiques épinglent k6 `2.3.0` et Chromium `156.0.8078.73` pour Windows. Ne pas en déduire une qualification actuelle ou une obligation de relancer les diagnostics Linux/WSL. Aucun bump, téléchargement ou nouvelle qualification demandé par cette rédaction.

## 5. Protections à préserver, indépendantes de l'option

- Cible locale fermée et explicitement autorisée ; contrôles contemporains de l'Engine Linux, labels/topologie/IDs puis contexte/namespace/UID/ressources nécessaires avant effet. Noms, URL ou fichier déclaratif seuls ne donnent pas permission. Les checks natifs du [cycle local](kind-local-native-launcher.md#11-cycle-natif-des-nœuds-existants--go-utilisateur-du-11-novembre-2026) ne prouvent pas à eux seuls l'origine HTTP, la propriété d'un Secret ou une autorisation de mutation.
- Ne retirer aucune garde des chemins encore utilisés avant remplacement ciblé et autorisé ; ne recopier ni tout `LocalClusterTargetAnchor` ni un nouveau protocole de baux générique. Cartographier pour chaque effet conservé sa vérification et son refus observable.
- TLS avec CA/SAN, protections navigateur et séparation des secrets maintenus ; aucun insecure/no-sandbox. Chemins privés explicites, pas de recherche latest, credentials dans argv/logs/résumés, GET global de Secrets ou export de headers/tokens bruts. Erreur d'auth/TLS/transport ou timeout = échec, jamais « absent » autorisant création.
- Si un test mutateur est finalement choisi : autorisation spécifique, préimage privée bornée, UID et valeur attendue vérifiés avant mutation/restauration ; refus en cas de changement concurrent. Prévoir réparation même après setup interrompu, sans exiger qu'un Pod cassé soit Ready pour pouvoir le réparer. Ne pas adopter un Secret partagé historique dans le profil natif.
- Cleanup par ressources/descendants réellement possédés et vérifiés, pas par nom ou PID seul ; aucun prune, suppression de volumes, arrêt du lab keepalive ou d'un workload étranger. Un effet non annulable reste explicitement à arbitrer, pas masqué par un label « cleanup PASS ».

## 6. Retrait conditionnel et petites étapes suivantes

**Inventaire terminé pour cette proposition**, pas migration terminée. Prochaines étapes :

1. Décision humaine A/B et confirmation du premier périmètre IAM ; petite sélection produit avec cas d'erreur/sécurité utiles, effets et prérequis. Aucun nouveau test ou scénario écrit dans le mandat courant.
2. Si B est retenue, ne pas imposer les26 checks OAuth ou les compteurs historiques comme critères de parité. Si A est retenue, déterminer explicitement ce qui reste pertinent, adapté ou abandonné. Dans les deux cas, conserver les comportements de sécurité choisis et les protections opérateur ; ne pas promettre une couverture exhaustive.
3. Implémentation ultérieure d'une première préparation/exécution/cleanup native sans SQL/clé supplémentaire pour un éventuel smoke IAM HTTPS, puis auth seulement après définition du profil/fixture et de ses oracles utiles. Toute exécution future exige une cible disponible et un accord distinct ; **pas de nouvelle campagne ou test de harness imposé ici**.
4. Supprimer au fil des consommateurs effectivement remplacés ou explicitement abandonnés : wrappers Python de préparation/supervision et diagnostics optionnels devenus sans usage. `windows-auth-ops.py` reste tant que ses effets conservés n'ont pas de remplacement sûr ; l'abandon explicite d'un cas peut aussi supprimer son besoin, sans inventer une équivalence.
5. `ops/deploy/local_full_target.py` et `bootstrap_local_kind.py` ont aussi des consommateurs common/k6 hors nouveau launcher : aucune suppression aveugle. Le simulateur OAuth autonome peut rester une **fixture** si un cas choisi le nécessite ; « zéro Python » n'est pas le critère. Scénarios abandonnés et tests exclusivement liés à un wrapper exigent une disposition explicite, pas une suppression dans ce lot documentaire. Les tests Rust demeurent intacts.

Critères bornés pour la suite : périmètre/option approuvés ; chaque cas retenu relié à un comportement produit et à ses prérequis ; aucune garde utile perdue ; effets/cleanup attribuables ; erreurs, TLS et résultats incomplets non présentés comme succès. Validation de code future proportionnée aux changements, sans suite globale ni nouveau test prescrit par ce document. **Aucun verdict runtime aujourd'hui.**

## Sources et portée de vérification

- Sources relues : les quatre JS ci-dessus ; [`summary.js`](../../ops/tests/k6/lib/summary.js) ; [`windows-auth-ops.py`](../../ops/tests/k6/lib/windows-auth-ops.py), notamment185–229 et231–320 ; les trois contrats Accepted liés en tête et ADR-0504.
- Analogies/limites : [séparation runtime/validation](ops-kind-runtime-validation-separation.md), **Proposed et historiquement liée au déployeur retiré**, pas autorité supérieure au launcher natif ; [contrat du launcher](kind-local-native-launcher.md) ; dernière section du [ledger opérateur](../../.cursor/handoffs/20261110-kind-local-operator-cycle.md), base publiée `8da70f7`.
- Le volume du harness et ses autres consommateurs sont issus de l'inventaire parent, **pas d'un nouvel audit exhaustif** ici. Aucune exploration Rust, exécution k6 ou interrogation de serveur/infrastructure. Rapport/ledger de cette étape restent au parent.
- **Aucune nouvelle ADR pour l'instant** : l'arbitrage réutilisation/réécriture et le périmètre produit restent ouverts ; les interfaces ci-dessus sont candidates. Leur ratification éventuelle devra expliciter les écarts aux contrats Accepted, notamment le parent Python/route HTTP de0504, sans les remplacer silencieusement.
