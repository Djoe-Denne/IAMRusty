# Archives Windows vérifiées, aucun outil encore exécuté

Le but est de tester les versionsWindows épinglées, sans confondre un navigateur
installé différent avec la cible retenue. Les deux archives officiellesWindows
sont désormais acquises. Le lead a revérifié leurs empreintes sur disque et la
forme des exécutablesWindows64bits, sans les lancer.

Le k6 déjà installé correspond exactement aux octets de l'exécutable issu de
l'archive officielle2.3.0. Cela établit cette correspondance de contenu, pas encore
la sortie réelle de la commandeversion. Chrome156Windows est acquis séparément ;
le Chrome149 installé n'est ni utilisé ni modifié.

## Provenance et limites

- k6 : URL de version officielle, métadonnéesrelease et checksums éditeur recoupés,
  archive et exécutable Windows64bits contrôlés.
- Chrome forTesting156.0.8078.73 : métadonnéeGoogle versionnée/win64/révision1706539
  corrélée à l'archive HTTPSofficielle, empreintes locales figées et exécutables
  PE64 contrôlés. AucunSHA256 éditeur publié ouMD5 éditeur n'a été observé : les
  empreintescalculées ne sont pas présentées comme une signatureGoogle.
- Les archives et leur provenance sont dans le dossier temporaire ignoré deGit.
  Aucun binaire ou cache n'est ajouté aux sources à publier, et aucuneextraction
  ni installation n'a encore été effectuée. Ce dossier dansGit n'est pas un
  chemin admis à l'exécution future : export privé et recontrôle nécessaires.

Le lanceurWindows et ses tests sont en cours dans l'arbre de l'auteur. Le verrou
de versions sera aligné sur le format fermé effectivement livré, avant toute
exécution réelle. Les contrôles de chemin, identité, confidentialité, isolation
et nettoyage ne sont pas remplacés par un checksum correct.

Actualisation de préparation : une copie portable neuve horsGit est maintenant
extraite dans le répertoiretemporaire approuvé,306fichiers contrôlés et les
empreintes des huitexécutables revérifiées. Aucun outil n'a été lancé. Les gardes
Windows du futur run ne sont **pas** déclarées validées par cette copie ; elles
seront exécutées avec des observations fraîches après intégration du lanceur.

## Verdicts actuels

Acquisition/intégrité des archives : vérifiées. Versions exécutées, inspections,
casChromium, isolation et HTTPWindows : **non exécutés**. WSL : **inconclusif et
suspendu**. Les deuxjobsLinux sont désactivés dans le changementlocal vérifié,
sans effet distant ou qualification revendiqué avantpublication.

Aucune nouvelleautorisation n'est requise pour les étapes ordinaires restantes
du mandatapprouvé. AucunDocker/Kind/WSL, nouveauLinux, VM, tunnel ou dispatchGitHub
n'est lancé. Les consommateurs de preuve réelle attendront les entrées intégrées.

## Annexe technique

- Commande d'acquisition `sh_18e093644001kt2gefCJ3c2PCY` terminée, état fermé
  `WINDOWS_ASSETS_ACQUIRED_NOT_EXECUTED`, pas d'exécution de binaire acquis.
- Preuve locale : `.tmp/k6-windows-assets-20261030-fa532dade42a4645bbd0ddf00750b289/provenance.json`,
  schéma/état et deux archives recoupés indépendamment après notification.
- k6 archive31706650octets :
  `112276d495e5741c968e2bc09ea6196099c1275bd6db9ee0875d173c7148ce43` ;
  exécutable67116752octets :
  `efec04c36452cfb9c0ef313082c021e5656caf5004f922c2d8b807fdf4a0e02f`.
- Chrome archive208195825octets :
  `5db0ad4c070907a791ec895b9de5ed15334b27f79af2556a131c3c11b3913a2b` ;
  chrome.exe4466176octets :
  `1f6d96eff29d6d5f410056bdcab4e22b93386b69a95cbd03501cb813b0471afa`.
- SourcechecksumsSHA2568ebbfcd5…, métadonnéeGoogleversionnéeSHA2564ed247fc… ;
  septimages.exe PE64 dansCfT, une dansk6. Ces faits ne prouvent pas leur lancement.
- Suspension du workflow et rapports intégrés localement `abe170a`, hooknormal
  cargoFMT/cached-diffcheck0, aucunpush. Pas de doubleédition des fichiersauteur.
- Métadonnéeshôte lues séparément sans k6/Chrome :OS/process64bits, build26300/
  OSVersion10.0.26300.0, WindowsPowerShell5.1.26100.9711/CLR4.0.30319.42000,
  principal effectivement nonadministrateur. Les valeurs du registre ProductName
  legacy et DisplayVersion26H2 sont conservées comme telles, sans inférence d'un
  autre labelOS. Ce relevé n'est pas un bail ou un contrôle d'identité fraîche du run.
- Export local `.tmp/k6-windows-portable-export-20261030.json`, état
  `VERIFIED_PORTABLE_WINDOWS_EXPORT_NOT_EXECUTED`, commandeexit0 après syntaxe0,
  huitexécutablescontrôlés et originauxconservés. Emplacementprivé effectif
  réservé dans ce relevé, pas une valeur de configuration publiée.
