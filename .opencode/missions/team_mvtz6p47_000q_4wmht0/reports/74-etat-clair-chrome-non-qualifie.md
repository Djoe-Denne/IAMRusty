# État clair : Chrome n'a pas passé les tests

L'objectif reste de faire démarrer Chrome avec son sandbox et de réussir le cas
k6 minimal. **Cet objectif n'est pas atteint.** Le dernier essai réel échouait
avant le début du cas ; le lancement direct sans k6 avait aussi produit un arrêt
fatal. Aucun nouvel essai Chrome n'a été lancé pendant l'enquête récente.

## Pourquoi unshare et ldd

Ces deux outils examinent les pistes apportées par l'utilisateur ; ce ne sont
pas des tests Chrome ni des tests applicatifs.

- `unshare` essaie de créer une isolation Linux utilisée par le sandbox. Il a
  réussi dans notre contexte supervisé. Le blocage spécifique de création
  userns décrit dans le ticket Claude Code ne s'est pas reproduit. Les autres
  exigences du sandbox Chrome ne sont pas toutes prouvées par ce succès.
- `ldd` examine les bibliothèques partagées nécessaires au binaire. Le premier
  appel a retourné0, mais notre lecteur ne reconnaissait pas correctement le
  chargeur parmi les dépendances directes ; le compte reste donc inconnu pour
  ce run. Un défaut du lecteur est démontré et corrigé sur fixtures, sans
  requalifier une sortie réelle effacée ni affirmer que toutes les bibliothèques
  sont présentes.

Les27tests verts concernent **l'outillage de diagnostic**, notamment ses refus et
sa confidentialité. Ils ne signifient pas que Chrome démarre ou que le cas
navigateur est passé. La cause réelle de son arrêt reste inconnue.

## Suite et limites

L'arbitrage architecte permet de préparer une seule seconde mesure ldd après
ratification ciblée et tests du sélecteur ; le contrat vient d'être livré Proposed,
pas encore consommé. Aucun second ldd, nouvel unshare ou Chrome exécuté à ce stade.
Si la vérification des dépendances n'apporte pas de cause, le repli Linux natif
proposé par l'utilisateur restera la prochaine voie à instruire, pas une boucle
de modifications du diagnostic. k6Windows est une alternative évoquée, non lancée.

AppArmor, sysctls, sandbox, pins et droits de stockage n'ont pas été modifiés.
Dernier inventaire : AIForAll arrêtée, baux libérés/copies retirées, ressources
étrangères inchangées. HTTP et Chromium non qualifiés, mission non clôturée.

Références : [mesures réelles](71-userns-reussit-dependances-encore-inconnues.md),
[défaut de lecture](72-chargeur-direct-et-defaut-du-lecteur-demontre.md),
[tests de sa correction](73-lecteur-corrige-et-27-tests-valides.md).
