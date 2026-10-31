# ADR-0506 : N’élargir le SDK autour d’OpenBao qu’après preuve d’une abstraction récurrente utile

- Statut : Proposed
- Réalité : Unimplemented
- Date : 2026-10-31
- Décideurs : à renseigner après accord humain sur cette proposition
- Jalon concerné : opportunité de mutualisation OpenBao / RustyCog (hors Apparatus P0–P6)
- SuperSède : aucune
- SuperSédée par : —

**Opportunité conditionnelle, en veille**, consignée à la demande de l’utilisateur. Ce n’est ni un nouveau statut, ni une cible d’implémentation engagée. `Unimplemented` concerne l’extension éventuellement envisagée, **pas** le KV déjà livré sous [0505](0505-startup-secrets-sdk-openbao.md), qui reste `Proposed / Implemented`. Aucun calendrier ni mécanisme automatique de réouverture.

## Contexte et faits de référence

L’[audit publié de réutilisation OpenBao](../missions/20261031-openbao-lazaret-iam-reuse-audit.md) est la base de cette proposition : AIForAll `master@2d00e5af0e3d727e48fd2b5897837ba7e4ac83ab`, SDK/gitlink `93b890ff7b821e9cb2e7d0aeb81957d098778a2f`. L’audit porte sur les sources au commit applicatif `8785cc9` ; sa publication documentaire ne constitue pas une migration. Ses sources, consommateurs et tests ont été inspectés ; les tests n’ont **pas été exécutés pendant l’audit**. La présente ADR ne refait ni exploration générale ni qualification runtime.

Trois usages sont constatés, pas trois besoins interchangeables :

- **GitHubConnect — KV au démarrage.** Le SDK fournit déjà la résolution async d’un champ KV v2, avec valeur/erreurs expurgées, transport borné et authentification explicite. GitHubConnect en fait un snapshot de boot. Le port n’interdit pas une résolution ultérieure, mais le résolveur conserve le credential acquis une fois, sans réacquisition/renouvellement ; sa TTL courte ne constitue pas une solution de credentials durables.
- **Lazaret — KV à la demande autorisée.** Port/parser métier local ; admission/session/grant avant résolution et injection dans l’opération. Une délégation au KV SDK existant est un **candidat à évaluer**, pas une migration décidée. Cycle de vie du credential, grammaire des références, valeurs vides et bornes diffèrent ; préserver aussi le mode explicitement désactivé. Garder l’adaptateur local reste valable.
- **IAM — Transit, autre sémantique.** Signature distante, lecture de clé publique à version exacte, création et rotation ; politique/scopes/bindings métier locaux. IAM est le **seul consommateur indépendant de Transit démontré dans cet audit**. Plusieurs organisations, clés, versions, instances ou tests d’IAM ne démontrent pas plusieurs consommateurs indépendants. La recherche bornée n’est pas une preuve d’absence de consommateurs ailleurs.

Transport et acquisition de credentials sont des **candidats à comparer**, pas des modules communs déjà décidés. L’audit relève notamment un raccord WIF→token OpenBao non qualifié : un test de câblage n’établit pas une authentification OpenBao réelle. Cet écart local ne doit ni être masqué par une abstraction, ni attendre sa création pour être traité dans un lot autorisé.

## Décision proposée

**Conserver la possibilité d’une mutualisation future, sans l’imposer.** Ne proposer un élargissement du SDK que si les trois conditions sont établies ensemble :

1. **Récurrence réelle suffisante** : des usages concrets, indépendants et durables ont besoin de la même capacité ; leur simple dénombrement n’est pas une décision.
2. **Besoins sémantiques proches** : opérations, erreurs, contraintes de sécurité, cycle de vie et responsabilité peuvent partager un contrat sans affaiblir les consommateurs ni accumuler des exceptions propres aux services.
3. **Abstraction significative et bénéfice net** : une responsabilité cohérente et des invariants réellement communs sont pris en charge, avec un gain explicite supérieur au coût du contrat public, des dépendances, tests, compatibilités et migrations.

Le même fournisseur OpenBao, des URL ressemblantes ou quelques lignes HTTP déplacées ne satisfont pas ces conditions. Une réduction de duplication peut aider l’argumentaire, pas le remplacer. **Conserver local est une issue normale**, y compris après réévaluation ; le nombre d’usages ne déclenche aucune promotion automatique dans le SDK.

Un futur port sémantique **fournisseur-agnostique** avec adaptateurs peut être pertinent si des besoins réels justifient cette séparation. Ce n’est pas la cible demandée aujourd’hui : ne pas inventer un second backend, implémenter un provider hypothétique ou présenter `mount/path/field` comme une référence universelle pour justifier un port. **Jamais d’API fourre-tout KV/Transit** ; lire un secret et effectuer une opération cryptographique restent des capacités distinctes.

## Quand rouvrir l’analyse

Un prochain lot peut rouvrir cette question s’il apporte au moins un **signal concret**, puis instruit les trois conditions ci-dessus :

- un autre consommateur indépendant demande effectivement une capacité aujourd’hui locale, par exemple une opération de signature distante compatible ; pas un simple nouvel usage interne d’IAM ;
- plusieurs chaînes réelles reproduisent les mêmes règles de transport ou de credentials, et leurs besoins de cible, validité, réacquisition et échec sont assez proches pour comparer une responsabilité commune ;
- un raccord concret, notamment Lazaret→KV existant, révèle une limite répétée et documentée que ni un adaptateur local ni une adaptation bornée de l’existant ne résout proprement ;
- des corrections répétées sur les mêmes invariants ou un coût de divergence observable rendent la comparaison économique/maintenance nécessaire.

Ces signaux autorisent une **analyse**, pas une extraction. Un incident local, une nouvelle clé/version/organisation, le seul partage d’OpenBao ou un backend seulement imaginé ne suffisent pas. Aucun daemon, rappel automatique, échéance de revue ou compteur de services n’est créé.

## Preuves attendues et sortie du prochain agent

Comparer les faits **du jour** à la baseline de l’audit, sans la traiter comme éternellement actuelle. Fournir un dossier proportionné, sourcé, distinguant faits observés, besoins confirmés et hypothèses :

- **Récurrence** : consommateurs indépendants nommés, opérations réellement utilisées/demandées, chemins/symboles/configurations et tests pertinents ; couverture et limites de la recherche. Distinguer tests lus, tests exécutés et protocole réellement qualifié.
- **Contrat commun possible** : ressemblances **et différences** d’opérations, références/versions, autorisation, erreurs, délais/tailles, confidentialité, credential/provenance/validité et comportement après expiration. Identifier les règles qui doivent rester chez chaque service ; ne pas décider une pseudo-API pour masquer un désaccord sémantique.
- **Gain démontré** : responsabilité et invariants supprimant une vraie complexité répétée, corrections/tests effectivement partageables, bénéfice qualitatif ou mesuré avec ses limites. Ne pas assimiler déplacement de fichiers ou comptage de lignes à un gain d’abstraction.
- **Coût et risques** : surface publique à maintenir, dépendances/features et portabilité Windows/Linux, sécurité, tests par opération/provider, compatibilité des valeurs/références, migration et durée de maintenance. Comparer aussi l’absence de migration ; aucun gain chiffré n’est présumé.

Rendre explicitement **une recommandation motivée parmi trois issues**, sans présupposer la troisième :

1. **Conserver local** : les besoins diffèrent, la récurrence est insuffisante ou le gain ne compense pas le coût. Documenter les faits et éventuels futurs signaux, sans créer de dette d’extraction obligatoire.
2. **Adapter l’usage au SDK existant** : privilégier un raccord local au KV livré ou une adaptation bornée de cette capacité si les faits le justifient ; préciser les incompatibilités à trancher. Ce n’est pas une autorisation implicite de modifier 0505 ou de migrer Lazaret.
3. **Proposer une extension** : seulement si les trois conditions sont établies ; nommer la responsabilité sémantique candidate et les alternatives rejetées. Une décision transverse ciblée et son éventuel contrat d’implémentation viennent **ensuite**, dans un lot explicitement autorisé, avant le code.

La réévaluation et son issue doivent être persistées avec leurs preuves. Elle ne transforme automatiquement ni cette ADR ni 0505 en `Accepted`, et n’autorise aucune migration à elle seule.

## Conséquences et invariants

- **Aujourd’hui : documentation seulement.** Pas d’extraction Transit, de nouveau port, d’adapter multi-backend, de module transport/credentials, de migration Lazaret/IAM ou d’extension SDK décidés. Le KV livré et son contrat restent inchangés.
- Invariants pour toute comparaison future : snapshot/fail-closed et défauts sûrs GitHubConnect ; contrôles Lazaret avant résolution, injection au moment autorisé et port métier local ; version exacte, isolation des scopes/bindings, non-exportabilité des clés et absence de retry après rotation ambiguë pour IAM. Ne pas déplacer de clé privée Transit en KV.
- Une correction locale de transport ou d’authentification n’a pas à attendre une mutualisation ; elle requiert son propre mandat. Les bornes KV ne deviennent pas automatiquement celles de Transit, dont les réponses et opérations diffèrent.
- Blast radius **actuel documentaire** ; blast radius **éventuel API/contrat cross-module**, à établir seulement lors d’une réouverture justifiée. Aucune nouvelle dépendance, route, migration DB, configuration de déploiement, compilation ou exécution de tests Rust dans ce lot.
- **Aucun contrat d’implémentation ni fiche API n’est créé** : aucune API, frontière de composant ou migration nouvelle n’est tranchée. Cette ADR fournit elle-même le cadrage de réévaluation ; pas de pseudo-API Transit/auth.

## Alternatives écartées à ce stade

- **Extraire dès qu’OpenBao apparaît dans plusieurs services** : confond fournisseur et sémantique ; ignore coût de maintenance et différences de cycle de vie.
- **Extraire Transit maintenant** : pas d’autre consommateur indépendant démontré ici ; garder local IAM reste le choix de référence actuel.
- **Décider d’avance un framework identité/transport ou un port multi-backend** : les candidats ne sont pas encore des responsabilités partagées démontrées ; aucun backend hypothétique pour les justifier.
- **Ne rien consigner** : perdrait la précision utilisateur et obligerait le prochain agent à redécouvrir les mêmes limites. L’ADR de veille garde l’option ouverte sans engagement de réalisation.

## Non décidé ici

API/provider futurs, architecture ou produit d’identité, mécanisme de credentials durables, migration d’un service, opérations Transit à partager, bornes propres à chaque opération, calendrier, financement ou promesse de réalisation. L’acceptation de toute proposition reste humaine et explicite.

## Références

- [Audit OpenBao Lazaret/IAM/RustyCog](../missions/20261031-openbao-lazaret-iam-reuse-audit.md), publié en `2d00e5a` : faits, sources code/tests, documentation primaire consultée et limites ; aucune preuve de migration ou de tests exécutés pendant cet audit.
- [ADR-0505 — secrets de démarrage SDK](0505-startup-secrets-sdk-openbao.md), `Proposed / Implemented` : KV existant distinct de l’extension future, aucun amendement/supersede par cette proposition.
- Précision utilisateur du présent lot : ne pas mutualiser à tout prix ; récurrence, sémantique et abstraction utile nécessaires ensemble. La mention « OpenAI » a été interprétée comme **OpenBao** dans ce contexte et cette interprétation a été annoncée par le parent.
- Preuve d’implémentation de l’extension envisagée : **aucune** ; `Réalité : Unimplemented` demeure intentionnel.
