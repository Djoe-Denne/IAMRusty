# 11 — Trois enrichissements démarrés et suivis

## Résultat observé
Admissions 1MCP réussies, trois daemons distincts en phase `warming` :
- AIForAll : PID **47328**, génération `2026-10-23T16:54:45.2282180Z`, manager21224.
- ARTIST : PID **45868**, génération `2026-10-23T16:54:40.8573510Z`, manager46308.
- re-ff8 via retro-eng : PID **46452**, génération `2026-10-23T16:54:46.6448250Z`,
  manager8044. Racine commune inchangée.

Les trois scans initiaux sont terminés. Zéro échec de résumé observé lors du
contrôle de démarrage, ce qui ne préjuge pas du résultat final.
Proxy /model/info : aucun max_parallel_requests, rpm ou tpm ajouté, Luna xhigh.
Sept profils vérifiés PASS. Cinq tests de suivi/couverture PASS ; contrôles
lifecycle précédents réutilisés, sans nouveau pipeline QA.

## Correction du contrôle final
GrepAI v0.37 `rpg/store_gob.go:Close` appelle Persist même après une consultation.
Les premières lectures CLI ont donc réécrit leur snapshot canonique ; elles
avaient lieu avant le lancement des watchers, sans changement sémantique demandé.
Le contrôle de stabilité les marquait non certifiables, correctement par prudence.

L'audit final travaille désormais sur des copies isolées rpg.gob/index.gob dans
un répertoire mkdtemp possédé, avec configuration minimale **sans credential**.
Le répertoire exact est retiré après audit ; aucune suppression de source, index
canonique ou cache. Contrôle réel sur copie ARTIST : couverture388/388 fichiers,
coverageVerified:true ; résumés encore incomplets dans l'ancien snapshot.

Les seuls moniteurs ont été remplacés pour charger cette correction, après
vérification PID/date/commande et archivage de leurs propres locks. Les trois
daemons et les requêtes LLM sont restés actifs ; compteurs et offsets conservés.
Les notifications exit1 des anciens moniteurs sont donc un remplacement contrôlé,
pas un échec des reconstructions. Aucun restart LiteLLM supplémentaire.

## Suivi actif / continuité
Shells actuels, notification automatique à la fin, ne pas poller :
- AIForAll : `sh_169b31bfc0014r3C01SOCiDaWq`.
- ARTIST : `sh_169b32c5b001p6KW4DgUgR6iIb`.
- retro-eng : `sh_169b33e26001ce5T4wucZ3PqF1`.

Index des suivis :
`C:/Users/djden/AppData/Local/LiteLLM/rpg-enrichment-20261023-3/README.md`.
Chaque dossier contient job.json, progress.md/json et le futur cleanup.json.
Les ressources restent nécessaires aux campagnes explicitement demandées ; pas
de teardown prématuré. Arrêt gracieux automatique par identité sur fin/erreur.

## Limites
Pas de pourcentage interne fiable par projet tant que le graphe en mémoire n'est
pas persisté. Audit final des résumés et erreurs requis ; `ready` seul n'est pas
une preuve d'enrichissement intégral. Pas de mesure réelle des tokens, pas de
provenance Luna de toutes les features. Pas de retry automatique d'une campagne
dégradée. Aucun Docker/Kind/WSL/Cargo, aucun commit/push, aucun index supprimé.
