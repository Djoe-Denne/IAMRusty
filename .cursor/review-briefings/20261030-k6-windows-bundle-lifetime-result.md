# Avis architecte : protection persistante et limite des handles

Retour natif result_mvvemxzx_002l_awca0h, lecture de bd7a2bf ; aucun code, contrat,
test, commande, permission ou runtime modifié par cet avis. R-1 n'y est pas clôturé.

## Constats

Open-K6BundlePins dérive le ZIPfermé et compare tous les fichiers extraits ; racine,
sous-répertoires et fichiers ont des leases et contrôles privés. PathLease.Acl
refuse les droits dangereux aux SID non fiables jusque sur les fichiers/parents.
Il exempte cependant opérateurcourant, SYSTEM, Administrateurs et TrustedInstaller :
ces rôles peuvent conserver écriture ou modification de permissions. Cela décrit
la frontière du code, pas une nouvelle approbation de confiance par cet avis.

Les ACL peuvent protéger les acteurs exclus après fermeture des handles. Elles
ne prouvent pas l'absence de mutation par les quatre rôles admis.

Close-K6PinnedTool est appelé indépendamment du cleanup. Les pins ne sont pas
héritables : CreateFileW reçoit IntPtr.Zero en paramètres de sécurité et la liste
du Child ne transmet que stdin/stdout/stderr. Éviter Dispose ne survit pas à la fin
du host. Une image mappée ne protège pas toute DLL qui serait ouverte plus tard.

## Deux assurances à distinguer

1. Protection contre les acteurs non fiables exclus par les ACL : voie potentiellement
   compatible, sans superviseur supplémentaire, par validation individuelle et
   persistante, conservation du bundle lors d'un survivant, aucun nettoyage/
   réextraction/remplacement/réutilisation mutatrice tant que sa disposition manque.
2. Code inchangé même face aux rôles opérateur/admis : non garanti après release.
   Une mise en réserve et un verdict nonPASS ne remplacent pas un verrou contre eux.

Le lead peut retenir la première seulement si elle correspond à la frontière R-1
déjà admise. Il doit l'expliciter, ne pas annoncer une immutabilité générale jusqu'à
extinction ou exclure implicitement un acteur auparavant couvert. Si la seconde
assurance est exigée, Chrome doit rester inaccessible dans les contraintes actuelles :
attente finie, host pouvant terminer, aucun autre détenteur et aucune extinction
garantie ne résolvent pas le cycle de vie. L'avis ne change aucune contrainte.

## Disposition et limites

Cleanupfalse/survivant : conserver bundle et données, disposition en attente,
pas clôturepropre ni mutation ultérieure supposée sûre. Une attente supplémentaire
bornée peut aider, pas masquer le même cas à son échéance. R-2 demeure INCONCLUSIVE.
Les ACL réellement validées du bundle cible restent nécessaires, pas déduites
des22tests. Aucune preuve Chrome/extinction/duréeprotection supplémentaire ; les
investigations indépendantes sous leurs propres préconditions restent possibles.

## Recoupement du lead avant décision

Les sources initiales68631c0, donc antérieures à la revueR-1, contiennent déjà :

- WINDOWS.md:65–68 : contrôle privé user/SYSTEM/Administrators et ancêtres
  TrustedInstaller ; modèle ne défendant pas contre opérateur/administrateur compromis.
- windows-native.cs:115 : opérateur, SYSTEM et Administrateurs fiables, pas antiadminDRM.
- même fichier:127 : listeTrusted à quatre rôles, inchangée dans le correctif.
- FindingR-1 initial : attaquant moins privilégié avec droit d'écriture sur une DLL
  préexistante, pas un opérateur compromis nouvellement exclu.

Ces faits justifient d'examiner une clarification fidèle de la première assurance,
pas une autorisation de relâcher le préflight ou de lever le gel. Un contrat
architecteProposed et une vérification indépendante de cette portée sont demandés
avant ratification et éventuelle correction. Aucune garantieuniverselle ajoutée.
