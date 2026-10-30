# Protections à corriger avant le premier navigateur Windows

L'objectif reste de vérifier k6 et Chrome sous Windows sans affaiblir leurs
protections. La revue indépendante a trouvé une course qui pourrait faire
nettoyer le dossier d'une autre opération locale. Elle a aussi identifié deux
conditions à résoudre avant d'autoriser le navigateur : vérifier ses bibliothèques
chargées, et ne pas présenter un contrôle partiel de son isolation comme une preuve
complète.

Aucun k6 ou Chrome Windows réel n'a encore été exécuté. Le lancement réel reste
gelé ; les corrections et la recherche documentaire autorisées continuent. Les
21 tests réussis restent une preuve du code initial, pas de ces futurs correctifs.

## Suite retenue

L'auteur corrige la création exclusive du dossier et la vérification du code
Chrome extrait, avec des tests négatifs sur des données factices jamais exécutées.
L'architecte examine en parallèle si une preuve d'isolation suffisante est possible
avec les droits ordinaires et les seules observations déjà admises. Tant qu'elle
manque, Chromium reste INCONCLUSIVE même si son JavaScript fonctionne.

Pas d'élévation, nouvelle instrumentation, suppression forcée, relâchement des
gardes ou reprise WSL. Cette revue n'autorise pas un nouveau périmètre. Aucun
arbitrage utilisateur n'est nécessaire pour ces corrections compatibles ; une
vraie limite de permissions ou de mandat sera signalée si elle devient indispensable.

## État et preuves

- [Rapport indépendant complet](../../../../.cursor/review-briefings/20261030T1913Z-security-k6-windows-primitives-cbcc1bb.md),
  lecture figée du code intégré68631c0, retour natif reçu puis persisté avant correction.
- Défaut actuel : création non atomique donnant à tort une autorité de nettoyage.
- Risque conditionnel bloquant avant navigateur : archive et exe intacts ne prouvent
  pas l'intégrité des DLL, les bibliothèques de code que Chrome charge ensuite.
- Limite de preuve : le jeton principal du renderer, le processus qui exécute la
  page, peut déjà être restreint alors qu'un thread conserve encore davantage de
  droits pendant son démarrage. Le booléen actuel ne suffit donc pas à qualifier
  l'isolation effective du cas.
- Revue statique seulement ; aucun test ou runtime par le reviewer. Contrôles
  diffcheck annoncés exit0, pas de PASS global. HTTP toujours sans cible admise,
  Linux suspendu, publication et clôture toujours inachevées.
