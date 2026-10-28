# 24 — Diagnostic borné de la latence Serena

## Objectif et stade
Répondre à la question utilisateur : les ~20s signalent-ils un défaut d'index ou
de cache ? Diagnostic lecture seule ; aucune réinitialisation, activation de
projet, purge, compilation ou réparation. Racine Serena AIForAll vérifiée,
LSP déclaré ready. Aucun worker supplémentaire.

## Faits déjà établis par le journal du backend AIForAll
Source privée : `%USERPROFILE%/.serena/logs/2026-10-23/mcp_20261023-094830_30200.txt`.
Ce backend est bien activé sur la racine AIForAll. Les autres journaux trouvés
concernent d'autres projets et ne servent pas de preuve pour ce diagnostic.

- Task26, `classify_transport`, a tourné **27,494s** dans Serena, alors que sa
  sortie était absente de l'événement composite où un script Context Mode avait
  échoué (rapport22). Le travail serveur a néanmoins continué.
- Task27, `classify_publisher`, planifiée à15:38:02,350, ne commence qu'à
  15:38:17,623 : **15,273s en file**, puis **4,258s d'exécution**. Cela explique
  presque exactement les19,569s vus par le client. Ce n'était donc pas une
  recherche de références prenant20s de calcul à elle seule.
- Task33, `ComponentStatus`, exécution réelle **20,796s** ; réponse riche
  originelle21695caractères, rendue compacte car le budget était8500.
- Polling fichiers environ0,19–0,22s ; sauvegardes de caches Rust observées,
  fichiers document_symbols.pkl/raw_document_symbols.pkl présents. Ces faits
  ne prouvent pas l'intégrité absolue ni la chaleur de tous les caches.

Logs : lignes1320–1347 (Task26/27),1390–1412 (Task33). Pas de LSP tracing activé,
donc décomposition précise recherche HIR/LSP/enrichissement des symboles inconnue.

## Contrôle supplémentaire borné
Comparer maintenant une lecture de corps connue et deux mêmes recherches de
références consécutives, séquentielles, sans changement de sources. But : observer
latence actuelle/stabilité et effet éventuel d'échauffement ; pas benchmark
statistique ni preuve qu'un cache complet devrait exister.

Résultats à consigner après les appels. Aucun accord supplémentaire nécessaire
pour ces lectures ; rien ne sera nettoyé ou redémarré ensuite, 8B conservé.

## Résultats actuels — diagnostic terminé
Appels séquentiels mesurés le2026-10-24 à14:15UTC, backend inchangé :
- Corps `classify_publisher` via `find_symbol` : **135ms côté client**, un seul
  symbole et708caractères retournés ; **16ms d'exécution serveur** (Task36).
- Références `classify_publisher` : **513ms**, puis **356ms** côté client ;
  **411ms**, puis **256ms** d'exécution serveur (Tasks37/38). Résultats strictement
  identiques,3236caractères,5fichiers, aucune troncature.
- Pour vérifier aussi le second appel réellement lent, une lecture additionnelle
  ciblée des références `ComponentStatus`, même budget8500 qu'historiquement :
  **1412ms client**, **1315ms serveur** (Task39), contre20,796s auparavant.
  Même notice21695caractères originels, retour compact5968caractères : la baisse
  n'est pas obtenue en supprimant arbitrairement la majorité du résultat.

Les phases serveur actuelles sont consignées aux lignes1587–1642 du même log.
Recherche bornée d'avertissements/erreurs cache/corruption/pickle/index dans ce
log : aucun trouvé. Les avertissements de rattachement de certaines références
au symbole de fichier et le cas Windows `nul` sont distincts d'une corruption de
cache. Les observations n'établissent pas la complétude absolue de l'analyse.

## Conclusion et correction de l'interprétation précédente
**Pas de panne ou corruption d'index/cache démontrée.** Les lectures de corps
et références fonctionnent actuellement entre0,135et1,412s sur ces cibles.
Les20s historiques ne décrivent pas la latence normale de toutes les opérations
Serena ; en particulier, l'appel publisher était surtout de l'attente en file.

Le premier appel `classify_transport` a continué côté serveur malgré l'échec
de l'appel composite côté client. Le suivant était donc en attente : c'est une
cause directement établie par les horodatages, pas une hypothèse d'index cassé.
Pour les27,494s de la première recherche et les20,796s de ComponentStatus, le
coût initial est compatible avec une analyse LSP et des données encore froides,
mais sans traces LSP détaillées on ne peut isoler analyse lazy, enrichissement
des symboles et effets des caches. Les différences de cibles/contenu interdisent
d'en déduire un ratio statistique de cache ou une garantie permanente de latence.

Serena repose ici sur LSP/rust-analyzer et ses caches de symboles ; ce n'est pas
l'index vectoriel8B ni le graphe persistant de rust-code-mcp. Leur réindexation
ou libération ne découle pas de ce diagnostic et n'aiderait pas la file Serena.

## Suite et limites
Recommandation : **ne rien vider ni réindexer**. Distinguer temps client,
attente en file et exécution serveur ; préserver les résultats d'opérations
parallèles quand un autre appel échoue, sans lancer des retries aveugles.
Une décomposition exacte d'un futur ralentissement nécessiterait des traces LSP
ou un profilage plus fin, à cadrer séparément sur cette connexion partagée.

Aucun défaut bloquant établi, aucune décision utilisateur nécessaire pour ce
constat. Aucun changement de configuration, restart, purge ou reconstruction ;
les caches internes peuvent naturellement être mis à jour par les lectures
Serena. Aucun Cargo/Docker/Kind/WSL ni modèle supplémentaire démarré. Le8B Rust
n'a pas été interrogé, libéré ou redémarré. Ce diagnostic ne valide pas tous les
symboles, versions/features ni la couverture globale du workspace.
