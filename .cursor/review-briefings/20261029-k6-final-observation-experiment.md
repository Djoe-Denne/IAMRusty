# Contrat borné — dernière observation Chromium, sans relâchement

## Décision du lead

Après les deux essais natifs non concluants, le rapport emergency-thinker `result_mvu4dt4z_0022_vgmkfd` établit une hypothèse différente et falsifiable : séparer échec natif avant scénario, exécution partielle et défaut d'observabilité du renderer. Le lead accepte son expérience de diagnostic uniquement. **Troisième et dernier essai natif autorisé après intégration et tests du paquet d'observation**, pas avant. Budget total maximum trois ; pas de nouveau pipeline/harness ni de méthode de preuve sandbox substituée.

Le lookup K6_BROWSER_EXECUTABLE_PATH n'est pas en défaut : gs.Env/os.Environ → InitEnv.LookupEnv → optionsbrowser ; include-system-env-vars=false filtre seulement __ENV. Ne pas ajouter --env pour cette variable ou changer les pins comme prétendu correctif. Le second diagnostic NATIVE_CAUSE_UNCLASSIFIED signifie exitk6 non nul, pas simplement témoin /proc absent. Cause native exacte inconnue.

## Ownership et état

HEAD publié `4e82a365077c505bb6681bbbb3018f3869f995b7`. Ancien auteur pool-4 arrêté, sources utiles publiées et sauvegardées ; messages force ne réactivent pas un membre shutdown. Un nouveau propriétaire isolé reçoit ce sous-lot distinct, sans reprendre/écraser l'ancien index et sans fork cross-directory d'un arbre sale. Périmètre exclusif `ops/tests/k6/**`, pas de workflow/ADR/contrat/service/identité/rustycog. Le lead écrit ledger/rapports et garde les exécutions CI/runtime. Un seul écrivain source.

## Paquet d'observation fermé demandé

Sur l'échec browser uniquement, projeter dans le résultat déjà existant, sans nouvel artefact d'échec :

1. catégorie du code enfant natif : absent/zéro/non-zéro/signal, pas de texte natif ;
2. état du résumé strictement courant : absent/invalide/valide ; compteurs du cas courant classés absent/0/1/autre, jamais valeurs brutes ni résumé intégral ; valider JSON, taille, confidentialité et corrélation au run courant ;
3. renderer **possédé effectivement observé**, pas absence certaine déduite d'un scan ;
4. lectures namespace de ce renderer : lisible/refusée/disparue/autre/non observée. Les valeurs de liens, errno brut, PID ou chemins ne sont jamais projetés.

Utiliser les observations déjà faites par le superviseur, sans nouvelle commande OS, scan distinct, accès privilégié ou lecture de processus étrangers. Conserver identité UID/PID/début pour cleanup. Les lectures optionnelles échouées ne doivent pas effacer les identités de base. Une nouvelle tentative de lecture ne devient pas preuve d'absence. Les témoins actuels et verdicts de sandbox ne sont **pas modifiés**, ni leur ordre de sécurité pour donner PASS.

Le classifieur lexical existant reste utile mais n'est pas suffisant. Toute projection doit être construite/validée par allowlist de clés/labels/types avant affichage ; une exception ou fixture ne peut injecter un dictionnaire/texte arbitraire dans le résultat. Les erreurs de confidentialité et cleanup continuent d'interdire publication et PASS. Ne pas convertir un résumé synthétique, absent, périmé, partiel ou une métrique manquante en succès.

## Invariants et interdits

Même k6v2.3.0, CfT156.0.8078.73, scénario about:blank, contexte neuf et règles de sandbox/confidentialité. Aucun argv/env/URL/path/texte natif/métrique brute publié. Aucun debug, wrapper, privilège, syscall de configuration, AppArmor/sysctl, installation, pin, nouvel artefact, tâche HTTP, CI dispatch ou répétition par le worker. Aucun changement d'architecture ; si une autre méthode de preuve devient nécessaire, remonter au lead/architecte avant code.

## Tests et livrable

Tests d'abord ROUGE pour les nouvelles catégories/corrélation et forged outputs ; puis tests purs Windows : erreurs de lecture simulées, résumé absent/tronqué/périmé/types invalides, sentinelles, garanties non-PASS et publication non appelée. Réutiliser fixtures Python existantes ; pas de serveur HTTP ni runtime ajouté. Le lead fera exercer les tests POSIX par le même workflow Linux lors de l'unique essai.

Rendre diff exact contre4e82a36, commandes/résultats réels, sources officielles utiles et limites. Pas de Cargo par le worker ; hook fmt Windows206 connu sur chemins worktree longs, ne pas répéter ni contourner le hook. Aucune écriture ledger partagé. Rapports13–15 et briefing `20261029-k6-browser-two-attempts.md` donnent les preuves antérieures sans les requalifier.
