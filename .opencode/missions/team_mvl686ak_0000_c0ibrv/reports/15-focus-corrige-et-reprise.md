# 15 — Plus de vol de focus observé ; trois campagnes reprises

## Résultat vérifié
Correctif local **0.37.0+windows-consolefix.1** déployé sur le binaire commun.
Source, tests, preuve de provenance et rollback : rapport14-consolefix-verifie.md.
Sept profils et absence de plafond local vérifiés PASS ; managers1MCPv6.

Après relance réelle des trois enrichissements, sonde WinEvent/Toolhelp10s :
**9 appels watch --status exit0**, **43 lancements Git observés**,
**0 événement de changement de premier plan**. Les enfants Git des trois
daemons sont bien présents pendant cette mesure. Preuve persistée :
`C:/Users/djden/AppData/Local/grepai-windows-fix/live-focus-proof.json`.
Pas de garantie universelle sur tous les programmes Windows : le cas de focus
reproduit a été corrigé et contrôlé pendant l'activité concernée.

## Campagnes actives, identités et baux
- AIForAll : watcher51668, born17:22:12.5511750Z, parentmanager5380 ;
  moniteur30312, born17:22:28.8081890Z.
- ARTIST : watcher52008, born17:22:08.1735730Z, parentmanager3320 ;
  moniteur4224, born17:22:34.2636950Z.
- re-ff8 via racine commune retro-eng : watcher52064, born17:22:14.0395950Z,
  parentmanager42240 ; moniteur38056, born17:22:40.1248180Z.

Nouveaux jobs : `C:/Users/djden/AppData/Local/LiteLLM/rpg-enrichment-20261023-consolefix/`.
Les trois audits initiaux sur copies sont stables et coverageVerified:true.
Les enrichissements ne sont pas encore annoncés terminés. Aucun index/cache
effacé, aucun restart LiteLLM depuis celui explicitement autorisé auparavant.

Shells à notification automatique, ne pas poller :
- AIForAll : `sh_169c916840013YMhcLJVrwHGyf`.
- ARTIST : `sh_169c92bd0001L61ZAJIJSPH404`.
- retro-eng : `sh_169c942a9001CEAVZop2BbJeqm`.

## Localisation / impact / limites
Go : nouveau helper interne de création de commandes, quatre sites Git masqués.
Windows uniquement ; arguments, annulation, I/O et comportement hors Windows
préservés. Ni Git global, ni WindowsTerminal global, ni fichiers applicatifs
AIForAll touchés. Sources hors racine Serena, couverture symbolique non applicable,
usages exacts vérifiés dans l'archive source et tests fonctionnels natifs exécutés.

Les moniteurs continuent les baux demandés. Ils auditeront les résumés persistés
sur copies et arrêteront gracieusement leur daemon possédé sur fin/erreur, sans
redémarrage automatique. Pas de pourcentage interne ni consommation tokens
inventés. Les ressources actives sont conservées pour ces campagnes utilisateur.
Aucun Docker/WSL/Kind/Cargo, aucun commit/push, aucune suppression de source.
