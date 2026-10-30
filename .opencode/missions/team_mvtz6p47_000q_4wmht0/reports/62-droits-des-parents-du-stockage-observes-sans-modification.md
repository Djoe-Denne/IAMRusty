# Droits des parents du stockage observés, sans modification

## Fait nouveau

Le défaut du conducteur ne doit pas seulement être corrigé sur des fixtures.
Une lecture Windows en lecture seule montre que le parent partagé `I:\WSL`
accorde au groupe « utilisateurs authentifiés » des droits DELETE sur ce dossier,
ainsi que de création de fichiers et de sous-dossiers. Ce dossier hérite de ses
parents et n'a pas de protection d'héritage propre.

En revanche, le dossier de l'instance et son disque possèdent bien les permissions
privées vérifiées lors du provisionnement : propriétaire courant, SYSTEM et
administrateurs seulement, héritage protégé. Aucun reparse point n'est observé
sur les quatre objets examinés. **Aucune substitution ou compromission n'est
constatée** ; ces lectures ne constituent pas un calcul complet des accès effectifs.

Les droits de l'ancêtre demandent donc une disposition explicite avant de lancer
le conducteur. Le bon résultat d'un garde corrigé pourra être un refus de cet
environnement, pas une raison de relâcher le garde.

## Décision et suite

Le lead ne modifie pas les permissions du parent partagé ou du volume et ne
déplace pas l'instance maintenant. L'auteur poursuit les deux corrections du
conducteur avec les oracles convenus, indépendamment de cette question d'hôte.

L'architecte examinera en parallèle la solution minimale dans le mandat de
l'instance dédiée sur I:, en préservant son nom, ses données et les autres
installations. Un éventuel déplacement natif de la seule instance possédée vers
une nouvelle chaîne privée peut être étudié ; il n'est ni exécuté ni tenu pour
autorisé par cette simple hypothèse. Toute opération doit d'abord avoir ses
préconditions, sauvegarde, permissions et effets exacts établis.

Le préflight réel et la collecte restent suspendus. La recherche documentaire,
les lectures nécessaires et les tests purs restent autorisés ; il n'y a pas de
nouveau GO général à demander pendant cette investigation.

## Preuves techniques et limites

- Deux diagnostics Windows ciblés exit0, Get-Item/Get-Acl et métadonnées seulement,
  sans WSL ou code du conducteur exécuté. Rôles/flags conservés, pas d'identifiants
  de compte personnels ou de descripteurs bruts publiés.
- Sur `I:\` : Allow AuthenticatedUsers applicable à l'objet, DELETE et création
  fichier/dossier ; pas FILE_DELETE_CHILD observé sur cette ACE. Propriétaire SID
  non account-SID, résolu, hors token courant ; son rôle fiable reste à caractériser.
- Sur `I:\WSL` : propriétaire courant, Allow AuthenticatedUsers hérité applicable
  à l'objet avec les mêmes flags ; pas FILE_DELETE_CHILD sur cette ACE.
- Feuilles `I:\WSL\AIForAll` et `ext4.vhdx` privées/protégées ; aucune modification.
- Documentation Microsoft File Security/Access Rights et File Access Rights
  Constants HTTP200 recoupée : DELETE et FILE_DELETE_CHILD sont distincts ;
  WRITE_DAC/WRITE_OWNER influencent aussi la sûreté d'une chaîne.
- Aucun exploit, processus adverse, nouvel utilisateur, mutation ACL, runtime,
  déplacement ou lecture de site inconnu. Les tests189 restent des preuves séparées.
