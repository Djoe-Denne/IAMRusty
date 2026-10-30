# Relecture ciblée des correctifs Windows intégrés

Reprendre reviewer21, même session/profil/modèle ; première revue indépendante
fresh préservée. Aucun nouvel auteur ou pipeline. ContratsWindowsAccepted, QA
proportionate et révisionnative2 inchangés. Source initiale68631c0, nouvelle source
intégrée au HEAD transmis après commit de ce brief ; fichiers/hashes ci-dessous.

## Surface du delta à revoir

- NativeOwnedDirectory/PathLease : création relative FILE_CREATE avec DACL à
  création et handleatomique, binding puis autoritécleanup ; DeleteEmpty par même
  handle ; EnsureCurrent/ACL/noreparse. SHA C#c653935f274a213559a27fb0c0f7e532c0736f254155d9d32f3a02be1ee801e5.
- Open-K6BundlePins/Assert-K6PinnedTool/Close-K6PinnedTool et callsites : dérivation
  de l'ensemble fermé depuisZIP authentifié, contenu et chaque fichier/dossier
  extrait/identités/ACL/readsharing ; rien extrait au run, pas nouveau lock.
  SHA PS5ba673483e8cf0d9281ebdc69d31f1f79cfc1a8838d736b354179589afa759b9.
- EntréeWindows e27a49320125a6353945d655352e54810f4bb48d129ff231222213b93289a455,
  preuveversion/checkJS dans cas existant, nouveauxtests et docs de disposition.
  Le code Child/Job/RendererToken est annoncé inchangé, à vérifier dans le diff.

## Réserves et frontières

S-1 et R-1 initiales du rapportpersisté à recouper, sans conclusionauteur reprise
commeoracle. Ne pas demander de preuve fonctionnelle d'un navigateur réel ici.
R-1reste explicitement ouvert sursurvivant : Cleanupfalse rend demain la main au
finally, qui libèrelesleases ; finhost auraitmêmeeffet. L'architecte examine cette
duréevie enparallèle, pas encore une décisionacceptée ni une correction livrée.
La relecture peut fermer/délimiter les parties indépendantes et signaler quel
fragment final devra être revu ; ne pas requalifier ce gap comme déjàfermé.

R-2 : avisarchitecte+dispositionlead intégrés6133563, document
`20261030-k6-windows-effective-sandbox-result.md`. Pasnouveautémoin/instrumentation.
Parent doit conserver ChromiumINCONCLUSIVE et exitnonzéro mêmeprimarywitness/JS/
version réussis. Ce n'est ni sandboxdésactivé ni preuveévasion. Évaluer que ce
garde empêche le fauxPASS, pas prétendre prouverl'effectif. CasréelresteGELÉ jusqu'à
S1/R1fermés et dispositionrevue ; aucuneactivationdéléguée àcemessage.

## Faits de vérification disponibles

Dixfichiers tailles+SHAidentiques àcandidate nativecomplète, `git diff --check`0.
Lead22testsciblés/23.986s/exit0/0skip, quinze nouveaux+septméthodesimpactées ;
deuxidentitésNode/Pythonobservéesfinies, dossiersprivésfixturesretirés. Pas k6/
Chrome/HTTP/WSL/Docker/Kind ou Cargo de campagne. Les21testsinitiaux restent à
leurportée, ne prouvent pas lesnouveaux guards. Sources et fixtures négatives à
examiner pour leurcouvertureeffective, pas desaffirmationsde qualification.

Readonly, deuxpasses défensive/adversariale surdelta : pascorrection/test/runtime/
permissionnouvelle. Retourcomplet borné findingsouverts/fermés/gaps/suggestions,
preuves/codepaths et hasheslus, LocalisationImpactLimites. Lead persisteavant
nouvellecorrection et ne libère pas un consommateur sur une fin d'auteur seule.
