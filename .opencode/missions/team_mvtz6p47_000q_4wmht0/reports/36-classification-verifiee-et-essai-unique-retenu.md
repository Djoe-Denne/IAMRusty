# Classification vérifiée ; un essai pour lever l'ambiguïté restante

## Résultat intégré

Le complément distingue les buffers vides des formats fatal, assertion ou texte
non reconnu. Il ne copie aucun contenu, emplacement source ou condition native.
Les trois fichiers correspondent exactement au candidat ; le lead a réexécuté
les **140 tests Windows**, tous réussis, avec 7 tests POSIX non exécutés.

Le superviseur, les binaires épinglés, les scénarios, le workflow et le contrôle
du résultat privé sont inchangés. La comparaison de structure du code confirme
que les fonctions existantes ne changent qu'aux deux endroits ajoutant cette
observation et la validation de sa sortie. Les formats ont été recoupés avec
les sources officielles Chromium156 et glibc2.35 ; cela ne prouve pas quelle
bibliothèque émettrait un message lors du prochain essai.

## Décision et règle d'arrêt

Après examen de la proposition architecturale et du code intégré, le lead retient
**un essai Linux borné**, déclenché par publication normale. Il doit lever une
ambiguïté réelle du dernier résultat : absence de texte ou format reconnu/non
reconnu. Ce n'est pas un essai de qualification présumé réussi.

Le diagnostic direct22 restera soumis au refus k6 courant admissible du même job.
Aucun nouvel argument, lecture système, réglage de sécurité ou collecte n'est
ajouté. Le job24 reste conservé et pourra être relancé incidemment par ce push.

Si le résultat est « autre texte », « aucun texte » ou une famille insuffisante,
pas de nouvelle série de reconnaisseurs ni de répétition identique. La proposition
d'emplacement source reste **Proposed**, sans entrée pertinente ni consommateur
admis. Toute autre observation doit d'abord disposer d'un contrat et d'un canal
compatible réels ; ce run n'en donne pas l'autorisation.

## Vérification et limites

QA proportionnée : lecture exacte, recoupement des formats, tests négatifs sur le
vrai lecteur avec frontières système simulées, comparaison de structure et test
complet local. Pas de réaudit global ajouté. Les anciennes revues indépendantes
restent valides pour leurs deltas, pas présentées comme une revue de ce complément.

Le code est committé avec le vrai hook de formatage et diff-check réussis. Aucun
nouveau résultat Linux n'est acquis à la rédaction. HTTP et selftests live restent
sans cible conforme autorisée, Chromium et runtime parent non qualifiés. Aucune
ressource Docker/Kind/WSL locale créée ou redémarrée ; mission non clôturée.

## Annexe technique

- Source : `1c0036f4f2b7bb8d7f306ba3e1a1222787a686d1`, delta contre `783fa80`.
- Candidat : `result_mvuaeuzv_001k_7bjgh3`, même session auteur
  `ses_e75f211bfffe498yEpFr2CniSK` ; ROUGE11/9errors attribué à l'auteur,
  GREEN140/0.762s/exit0/skips7 réexécuté par le lead, 133 tests exercés.
- Commande : `C:/Python313/python.exe -B -m unittest discover -s ops/tests/k6/tests -p test_*.py`,
  temp hôte approuvé. `git diff --check` normal et staged : exit0.
- 17 des19 fichiers de base inchangés ; AST de toutes les fonctions existantes
  identique après retrait des seuls blocs nouveaux `direct_text`.
- Sauvegarde20fichiers source/workflow et index vérifiée :
  `.tmp/k6-direct-text-source-1c0036f-21badf775d8348ea9e8ad9006a91c17e`.
- Deux écarts de récupération documentaire ont été corrigés avant tout test ;
  pas d'erreur produit ni panne MCP. Aucun changement utilisateur/étranger intégré.
