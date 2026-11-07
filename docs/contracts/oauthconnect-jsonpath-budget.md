# OAuthConnect — budget préalable de projection JSONPath (S-1)

Date : 2026-11-07. Clarification technique du [contrat actif, §2.3](oauthconnect-multi-instances.md#23-jsonpath-et-normalisation), sous ADR0413 **Accepted / Partial**. Aucun changement de Statut/Réalité, de pin `jsonpath-rust = "=1.0.12"`, de grammaire admise, de mapping ou de règle IAM. Ce fichier cadre le correctif ; il ne l'implémente ni ne le qualifie.

## 1. Disposition et seuil de décision

**S-1 à traiter avant clôture du lot, pas réputé fermé.** Le finding sécurité du 2026-11-07 est MEDIUM, sans HIGH/CRITICAL : un JSON fournisseur peut amplifier une expression configurée de confiance, notamment via des tests absolus imbriqués. IAM ne choisit pas cette expression. Les résultats 38 tests config et 41 tests projection verts rapportés par le parent ne démontrent pas ce bornage.

Décision locale : **préflight conservatif AST + structure JSON avant tout appel exhaustif à `js_path_process`**, au niveau de `CompiledMapping::project`. Une admission réussie n'exempte pas des contrôles existants de cardinalité, types, null et parents localisés. Un coût dépassé ou impossible à majorer produit l'erreur RustyCog générique existante, sans données/expressions dans le diagnostic, sans résultat partiel ni fallback. Pas de délai async autour du travail synchrone, de nouveau parser/évaluateur, de réécriture d'expression, de cache ou de framework de quotas.

C'est un **détail d'implémentation du bornage déjà accepté**, non une nouvelle décision structurelle : le contrat impose un traitement borné de données non fiables, sans garantir l'évaluation de toute taille/composition syntaxiquement valide. Le budget peut refuser conservativement des données dont le résultat aurait été valide, mais ne supprime aucune famille de query et doit conserver les témoins nominaux sous budget. Aucun nouvel accord humain requis pour ce correctif borné. Modifier les règles de projection, interdire les requêtes absolues, changer le pin/AST ou rendre ce budget configurable/public rouvrirait le contrat : remonter au parent avant de le faire.

## 2. Sources et frontière réelle

Lecture du **candidat non committé canonique**, `C:/Users/djden/source/repos/AIForAll`, HEAD `9e5a03901cee6b63fc5dba8b873d4da6dfaac146` ; pas des stubs du worktree documentaire :

- `services/OAuthConnect/infra/src/lib.rs` : `CompiledMapping::project` appelle quatre/cinq sélections, puis éventuellement `ensure_same_parent` ; hash lu `41480ffc116a821812d0b53fa130c714c3a93d8eabfce02b76091db1a8775fda`.
- `infra/src/json_path.rs` : `CompiledClaim::select` matérialise tous les résultats avant `len > 1` ; `ensure_same_parent` parse les deux chemins localisés et réévalue le parent. Hash lu `648e81bc7c6fb3128fa095790363f235de25afa30d80e3f6001e44210b83ed9e`, différent du briefing sécurité ; les corps pertinents ont été relus.
- `infra/src/json_path/policy.rs` : validation AST 512 octets/16 niveaux, tests relatifs/absolus récursifs admis, descendants/fonctions refusés ; hash lu `a12371e7afbb417050aceda251a6f9e49db34ea14b8f12cd9071aef54d06ed74`.

Sources **1.0.12** lues dans le cache registry, vérifiables dans [le paquet publié](https://docs.rs/crate/jsonpath-rust/1.0.12/source/src/query/) :

- `test.rs:6–12` : `AbsQuery` repart de `state.root` à chaque invocation ; `RelQuery` part du candidat courant. `atom.rs:17–40` vérifie l'existence **après** calcul du nodelist, sans limite de cardinalité précoce.
- `filter.rs:6–57` : collection entière parcourue/collectée ; candidats internes évalués comme prédicat. `Or`/`And` utilisent any/all, mais le préflight doit compter **toutes** les branches, sans crédit de court-circuit.
- `selector.rs:7–158`, `segment.rs:16–25`, `state.rs:125–159` : noms/index ne produisent au plus qu'un résultat par entrée ; wildcard/slice/filter peuvent développer une collection. Les unions clonent l'entrée pour chaque sélecteur, puis les réductions concatènent à nouveau les préfixes ; les doublons comptent.
- `comparable.rs:31–53`, `comparison.rs:6–77` : les queries comparables sont singulières ; deux valeurs sélectionnées peuvent néanmoins être des objets/arrays comparés profondément. Les comparaisons avec un littéral scalaire n'ont pas ce coût de traversée profonde.
- `queryable.rs:177–189` matérialise aussi les membres d'un objet. `state.rs:201–244` échappe les noms et construit/copie des chemins ; `query.rs:80–89` matérialise encore le résultat final. Compter seulement les prédicats ou les matches finaux ne suffit donc pas.

Cette lecture est une preuve statique de mécanisme/coût, pas une mesure CPU, un benchmark ou une revue sécurité indépendante du correctif à venir. Analogie existante de bornage avant traitement : plafond des corps HTTP du contrat §4 ; il ne remplace pas le budget combinatoire ci-dessous.

## 3. Constantes locales et scan structurel

Constantes privées proposées pour le correctif, **non exposées en config/API** :

- `MAX_SCAN_UNITS_PER_API = 1_048_576` ; unité de scan = un nœud `Value` + octets UTF-8 de ses chaînes/clefs, comptés une seule fois. C'est un majorant de volume en mémoire logique, pas une sérialisation JSON.
- `MAX_JSON_DEPTH = 128`, racine à profondeur 0.
- `MAX_PROJECTION_WORK = 1_000_000` unités abstraites de travail majoré définies au §4.
- `MAX_PROJECTION_COPY_BYTES = 16_777_216` octets majorés de copies/chemins au §4.

Le scan parcourt **seulement les APIs référencées par les quatre/cinq claims**, une fois par API distincte, sans réseau, clone, stringify ni parcours récursif de `Value`. Pile d'itérateurs empruntés par niveau : ne pas empiler d'un coup tous les enfants d'un tableau/objet large. Vérifier depth/compteurs avant de descendre ou de poursuivre ; arrêt dès dépassement. `.len()` donne les longueurs UTF-8, sans rescanner chaque chaîne. Ne pas parcourir les réponses supplémentaires non référencées pour établir le budget.

Résumé par API : `N` nombre de valeurs, `B` somme des octets clefs/chaînes, `S=N+B`, `A` largeur maximale d'array, `O` nombre maximal de membres d'objet, `K` taille maximale d'une clef UTF-8. Poser `F=max(1,A,O)`. Zéro clone de JSON ; résumé éphémère propre à la projection, jamais conservé dans `CompiledMapping`/entre requêtes. Les réponses restent empruntées immuablement entre préflight et évaluation.

Le scan est borné séparément à au plus cinq fois `MAX_SCAN_UNITS_PER_API` et 129 cadres ; il ne remplace pas le plafond HTTP décompressé de 1 MiB par réponse du futur transport. Les `ApiResponses` construits directement en mémoire sont aussi contrôlés. Les constantes ne prétendent pas un SLO en millisecondes ou un plafond RSS exact.

## 4. Majorant AST, sans évaluer les prédicats

Parcours de **l'AST maintenu déjà validé**, pas de deuxième parser. Ne lire aucun résultat de query pour décider de l'admission. Un résumé de séquence transporte `(R,C,p)` : multiplicité maximale de références, travail majoré cumulé, longueur maximale d'un chemin produit. Entrée d'une query racine : `R=1`, `C=4`, `p=1`. Une addition/multiplication utilise `u64` vérifié ou saturation à **plafond+1**, jamais wrap ni saturation au plafond accepté. Tester `<= plafond` ; au-delà, refuser immédiatement.

### Règles de calcul

- **Name** : `R'=R`, ajouter `4*R*(O+1)` à `C`. Une recherche de clef est majorée conservativement par le nombre de membres d'objet ; ne pas appliquer la largeur des arrays à un lookup nom. **Index** : `R'=R`, ajouter `4*R`. Inclure indices négatifs admis dans la même règle de coût, sans changer leur sémantique. Ainsi un chemin noms/index reste linéaire en nombre d'étapes/entrées, **pas `N^depth`**.
- **Wildcard/slice** : `R'=R*F`, ajouter `4*R*(1+F)` à `C`, même si la slice réelle retourne moins de valeurs. Cela couvre collecte/matérialisation intermédiaire et l'énumération objet. Ne pas créditer un petit résultat final.
- **Filter sélecteur** : si `c(f)` majore un prédicat sur un candidat, `R'=R*F` et ajouter `4*R*(1+F) + R*F*c(f)` à `C`. Le prédicat lui-même ne multiplie pas à nouveau par `F` à chaque nœud logique ; seuls ses parcours de collection le font. Le mode interne booléen du moteur est couvert conservativement, sans devoir le reproduire.
- **Segment à plusieurs sélecteurs** : calculer chaque alternative depuis la **même** entrée, additionner sorties et coûts, puis ajouter `4*k*(R + somme_des_sorties)` pour les `k` alternatives. Cette surcharge couvre les clones d'entrée et copies cumulatives des réductions/concaténations. Ne pas dédupliquer les sélecteurs/références ni prendre le maximum d'une seule branche.
- **Séquence** : composer les multiplicités et additionner les coûts des étapes ; ne pas multiplier tout le JSON à chaque Name/Index. Les segments descendants/fonctions restent refusés par la policy existante ; ne pas les faire entrer dans un majorant permissif.
- **Prédicats** : groupe/négation = `4 + coût_enfant` ; `And`/`Or` = `4 + somme(coûts_enfants)`. Test existentiel = `4 + coût_de_sa_query`. Pour `AbsQuery`, **repartir de la même racine et de ses statistiques, une fois par invocation du prédicat** ; son coût est donc multiplié par les candidats de son filtre appelant. `RelQuery` commence sur un candidat, avec les mêmes bornes structurelles conservatrices de son API ; pas de visite concrète des candidats dans le préflight.
- **Comparables** : littéral scalaire coût 4 ; query singulière Current/Root coût de ses noms/index, multiplicité 1, Root réinitialise le chemin. Comparaison = `4 + coût_gauche + coût_droite + h`. Si au moins un opérande est un littéral scalaire, `h=4+2*L`, `L` étant le maximum des longueurs UTF-8 des littéraux chaînes concernés (0 pour les autres scalaires). Si les deux sont des queries singulières, `h=4+2*S` : couvrir égalité profonde, clefs/chaînes et le double traitement possible de `<=`/`>=`. Ne pas supposer leurs valeurs runtime scalaires seulement parce que leur query est singulière.

### Chemins, copies et coût total

Calculer `P`, majorant maximal de chemin de **toutes les sous-queries considérées**, en parallèle du calcul précédent. Name ajoute `4+6*len(nom_UTF8)` ; Index ajoute au plus 22 ; wildcard/slice/filter ajoute au plus `max(22,4+6*K)`. Les alternatives prennent le maximum des longueurs, sans oublier la somme des coûts. AbsQuery/Comparable Root repartent à `p=1` ; une query relative de prédicat part d'un chemin interne vide (`p=0`). Conserver les maxima rencontrés avant chaque remise à zéro. Le facteur 6 couvre l'échappement des noms ; ne pas construire les chaînes pour le mesurer.

En particulier, un chemin purement noms/index ne prend pas le `K` d'une énorme clef **non sélectionnée** ailleurs dans le JSON. Le maximum de longueur des littéraux chaînes d'AST est `Lmax` (0 s'il n'y en a pas).

Avant toute évaluation, réserver aussi la vérification existante des parents email/verified : au plus 16 noms/index, coût `4+64*(O+1)` sur leur API, chemins au plus aussi longs que les chemins email/verified déjà majorés. Cette réserve couvre les deux parsings de chemins localisés, leur comparaison structurelle et la réévaluation du parent, sans lui réappliquer arbitrairement la borne config 512 octets. Pas d'appel ultérieur à `js_path_process` hors de cette réservation.

Additionner les `C` des quatre/cinq queries et cette réserve en `C_total`. Prendre `Pmax` et `Lmax` sur ce même ensemble ; définir `U_total = 8*C_total*(Pmax+Lmax+32)`. Ce majorant volontairement large charge répétitions, références, buffers de chemins, clones de littéraux et concaténations intermédiaires/finales, pas seulement la taille de sortie. Le facteur 8 et le terme 32 sont des marges locales pour les opérations/copieurs de cette version, non une mesure d'allocateur ou une garantie mémoire globale.

Admission **unique pour toute la projection** : `C_total <= MAX_PROJECTION_WORK` **et** `U_total <= MAX_PROJECTION_COPY_BYTES`, scan de chaque API accepté. Ne pas réinitialiser un budget par claim, sous-query absolue, branche logique ou seconde évaluation du parent. Après admission seulement, laisser la bibliothèque évaluer **le même AST** et conserver toutes les règles de projection existantes. L'estimateur n'est ni un évaluateur de secours, ni un oracle sur la vérité d'un filtre.

### Argument de borne et limites

À chaque niveau, les nœuds effectivement traités sont au plus la multiplicité calculée : nom/index au plus un successeur ; expansion au plus `F` enfants par occurrence ; unions conservent les occurrences en double ; tests absolus répètent le coût racine à chaque occurrence appelante. Les `C` additionnent ces occurrences et les re-copies des unions ; `U_total` charge leur longueur maximale de chemin/littéral. Les comparaisons profondes sont couvertes séparément par `S`, sans les confondre avec les simples comparaisons à un booléen/entier/chaîne littérale. Aucun raccourci d'existence, de logique ou de cardinalité n'est supposé. La préanalyse elle-même ne développe aucun produit cartésien : elle combine des entiers bornés sur l'AST de profondeur ≤16, puis s'arrête en dépassement.

Ce raisonnement est attaché au moteur 1.0.12 et aux variantes autorisées. L'auteur production doit recouper ces règles avec les points de copie effectivement appelés et refuser de revendiquer une majoration qu'une variante rendrait fausse ; signaler un écart au parent avant de changer de mécanisme. Le budget est conservatif : une grosse structure inutilisée ou un filtre qui aurait rapidement répondu faux peut être refusé. Cela ne justifie ni une exemption GitHub, ni un élargissement implicite des plafonds, ni une annonce de fermeture S-1 avant les preuves.

## 5. Oracles petits, dans la suite existante

Préparer les tests avant le correctif ; exécution par le parent uniquement. Garder les 41 attentes existantes, les trois caractérisations lexicales séparées et les vrais comptes RED/GREEN. Aucun benchmark, stress, nouveau harness, runtime ou assertion de temps écoulé.

1. **Trois tests absolus imbriqués** : même expression compilée sous 512/16, par exemple `$.items[?($.items[?($.items[?($.items[?(@.ok == true)])])])].id`. Elle contient trois `AbsQuery` imbriquées sous le filtre extérieur : jusqu'à quatre parcours de collection. Fixture racine à deux membres `items` et `p` ; `items` à 2 puis 4 objets, tous `ok=true`, **un seul** avec `id="stable"` ; `p={e:"one@example.test",v:true,u:"name"}` fournit les autres claims par `$.p.e`, `$.p.v`, `$.p.u`, sans picture. Les petites clefs fixent ici `K=5`, `O=3`, `F=3` puis 4 ; ne pas gonfler involontairement le majorant avec d'autres champs. Les deux projections doivent réussir : pas d'interdiction du motif absolu, pas de cardinalité finale multiple qui masquerait le budget.
2. **Dépassement avant évaluation exhaustive** : même mapping, même forme, 32 éléments seulement, toujours un seul `id`. La compilation reste admise ; la projection doit refuser par le préflight, sans fuite de valeurs. Le résultat logique resterait singleton si évalué : le refus ne doit donc pas provenir de `matches.len()>1`. Avec les règles ci-dessus et les groupes explicites de cette expression, le calcul de référence à 4 éléments donne `C_total=13_720`, `Pmax=85`, `Lmax=0`, soit `U_total=12_841_920`, sous les deux plafonds ; à 32 éléments le seul repli de sous-query atteint déjà `C=1_348_760`, avant la query extérieure. Ce sont des estimations contractuelles, **pas des résultats d'exécution**. Recouper le branchement préflight avant le premier appel bibliothèque ; témoin interne local de non-appel si nécessaire, sans API publique ni moteur substitut. Pour les opérations d'addition/multiplication/admission, couvrir aussi égalité au plafond admise et plafond+1/overflow refusés par des tests unitaires locaux, sans grossir les fixtures JSON.
3. **Nominal linéaire non régressé** : petit document avec une chaîne de 8 à 16 noms/index vers un ID unique et un array non sélectionné de 128 petits éléments. Projection admise : sa largeur ne doit pas devenir `N^depth`. Ajouter/conserver le mapping GitHub primaire seul sur une petite collection (dont 32 entrées avec une seule primaire) ; les noms/index et comparaisons à `true` restent admis. Même corps avec une valeur de booléen changée ne réduit pas le coût estimé par court-circuit.
4. **Fermeture de la couverture du majorant** : petits témoins ciblés pour union avec doublons, wildcard/slice, query relative et absolue, plusieurs branches And/Or, comparaison de deux sous-objets et chemins avec noms échappés. Vérifier cumul des claims/réserve de parent et bornes du scan, y compris profondeur excessive construite en mémoire. Aucun cap de sortie ajouté qui accepterait discrètement le premier match ; succès/refus fonctionnels historiques restent les oracles.

Ces tailles sont des témoins calculables, pas un seuil public « 32 emails interdits » : le seuil dépend du coût AST/structure complet. Les auteurs vérifient les estimations et le passage effectif sous/au-dessus des constantes choisies ; s'ils ne correspondent pas, remonter la différence de calcul avant de modifier l'oracle ou les plafonds. Une erreur syntaxique/compilation/environnement n'est pas le RED de S-1.

## 6. Handoff et preuve restante

Actualisation d'exécution du 2026-11-07 : mécanisme intégré,12tests publics S-1 exécutés en RED6/6 avant correction puis12/12verts,44tests projection non régressés et20unités privées complémentaires vertes. Recoupement sécurité ciblé du fonctionnel au commit `a16d12b202a1d742d1c7f0c2c1f6a4caed3b4cec`, sans nouveau finding ; seuls le module conditionnel et les tests ont été ajoutés ensuite. Les conditions unités/qualité ont été satisfaites : fmt/check/Clippy strict verts. **S-1 clos au périmètre de ce lot**, sans qualification HTTP, RSS, SLO ou service complet. Les prescriptions de validation ci-dessous décrivent le protocole qui a été suivi, pas un manque de preuve restant.

Localisation de correctif : `services/OAuthConnect/infra/src/{lib,json_path,json_path/policy}.rs`, éventuel petit module privé de préflight ; tests dans la suite de projection existante et unités locales nécessaires. Domaine, configuration publique, SDK, root Cargo/lock et pin inchangés. Aucun changement du scanner lexical de validation déjà acquis pour satisfaire le budget.

Le parent intègre cette clarification avant les auteurs tests/production, arbitre l'unique slot Cargo et conserve la revue S-1 initiale. Attendre : RED ciblé comportemental, GREEN sur mêmes oracles, régression projection/config et trois caractérisations, Clippy strict ciblé ; puis recoupement de sécurité limité au mécanisme de mitigation et à ses preuves, sans nouvelle revue globale automatique. Les commandes existantes du lot restent les entrées, pas un nouveau runner. Zéro test/Cargo/runtime exécuté par l'architecte pour cette note ; aucun verdict de clôture de S-1 ou promotion de Réalité ADR0413.
