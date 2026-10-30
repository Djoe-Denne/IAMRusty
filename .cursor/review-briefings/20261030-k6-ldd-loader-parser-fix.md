# Corriger la couverture du chargeur dans le parseur ponctuel ldd

## État et preuves

Même auteur13/session/profil/modèle, deux fichiers.tmp seulement. Bases : script
SHA2567f17cdc551a62cc3b73c513da961677ed7316ff34332d1777b3c795961f64554,
fixtures09da2edea06721396d7df89e4b72d2b0accfed2b31b87d6f31456502b2a2074d.
OPSf279, Child/env/pins/scénarios/contrats/ledger inchangés.

Une seule vraie commande ldd : exit0 mais countnull, capture effacée conformément
au contrat ; unshare0. Nettoyage/confidentialité/retrait LinuxPASS et ressources
parent réconciliées/arrêtées, aucune qualification Chromium/HTTP. AnalyseRO
`result_mvv8a2cg_000h_ijnump` conclut à un défaut de couverture statique, sans
prouver la branche exacte rencontrée par le run réel.

Lecture **fichier ELF seule**, sans nouvelle commande :32DT_NEEDED et basename du
chargeur directement nécessaire=TRUE, ELF/archives/chargeur/source revérifiés.
Cette condition rend le défaut pertinent, pas rétrospectivement causal certain.
Aucun nom de bibliothèque ou flux ldd inconnu n'a été exporté. Sources glibc2.39
rtld.c/ldd.bash.in versionnées déjà relues ; pas version locale affirmée par ces
sources. `/usr/bin/ldd` sourcehash4f1d37e25f27535e3f02a5b7da63e1ce18d4982445db2c25fc8f985a3d395cc3.

## Correctif autorisé, avant toute nouvelle expérience

Seulement la branche qui a déjà reconnu un **chemin de chargeur autorisé** :
compter aussi son basename dans les noms effectivement représentés, afin que
needed<=names puisse couvrir un chargeur directement nécessaire. Ne jamais
ajouter un nom simplement attendu dans l'ELF sans une ligne reconnue. Conserver
chemin whitelisté, cardinalité exacte1 et tous les refus doublons/ambiguïtés.
Pas de large grammaire flottante, nouveau collecteur ou nouvelle sortie brute.
Les autres formes non reconnues peuvent rester INCONCLUSIVE ; pas de permissivité
pour fabriquer un zéro. Tout élargissement supplémentaire doit être justifié.

Fixtures pures : GOOD+needed{liba,chargeur} ROUGE(None) puis GREEN0 ; même cas
avec liba notfound =>1 malgré exit0 ; GOODsanschargeurdansneeded =>0 inchangé.
Chargeur absent/chemin inconnu/doublon/autre besoin non couvert =>None. Conserver
signaux, non-zéro, timeout, interruption, confidentialité, overflow et retrait.
Suite ponctuelle seulement, pas194redondants, pas WSL/Linux/native commands.

Livraison native DELTA exact sur ces bases, nouveaux SHA et tests réels ; pas
relecture du worktree externe par le lead, ni écriture canonique par l'auteur.
Ne pas modifier les copies de référence de la preuve réelle passée.

## Limite d'exécution inchangée

**Aucun nouvel appel ldd/unshare/Chrome/k6 n'est autorisé par ce correctif.** Le
contrat Accepted borne encore les commandes à une occurrence par voie. Une
éventuelle expérience supplémentaire devra être arbitrée explicitement avant
code consommateur/runtime ; ne pas relancer tout le script qui contient unshare.
Pas de mode ldd-only, instrumentation supplémentaire ou changement contrat ici.
La sortie du run02f... reste countnull, même après correction du lecteur.
