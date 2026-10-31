# Préparation des tests réels, sans changer les 24 jobs

La mission reste à l'étape de qualification : les corrections et leurs revues
source sont terminées, mais les preuves réelles de base de données, de contrôle
des droits, d'Envoy et du navigateur ne sont pas encore acquises.

Pour éviter le pic mémoire du premier gros lot, les54 cibles de tests Hive/IAM
sont précompilées une par une. Les24 jobs internes restent inchangés, avec un
seul Cargo actif. Cette opération n'exécute aucun test ni conteneur ; son résultat
final reste attendu.

Le préflight confirme que les images PostgreSQL, LocalStack et OpenFGA exactes
des fixtures sont déjà présentes. Aucun téléchargement ni démarrage supplémentaire
n'a été nécessaire. Le runner existant exécutera ensuite les suites Hive/IAM,
en conservant l'inventaire et le nettoyage des seuls conteneurs qu'elles créent.

Les tests de routage SQS des deux services activent réellement leur file dans
leur propre processus. Deux autres anciens tests IAM restent explicitement
ignorés, dont Kafka : ils ne seront ni forcés ni présentés comme réussis.

## Limites et prochaine étape

Les20 conteneurs préexistants restent étrangers à la mission. Trois anciens
registres ont un encodage ambigu et ne donnent aucun droit de nettoyage.
Aucune ressource Docker, Kind ou navigateur n'a été créée par cette préparation.

À réception du résumé de compilation, j'examinerai les résultats puis lancerai
la campagne réelle autorisée, sans nouvelle demande de GO de principe. Kind et
le navigateur restent après les IT finales.

## Références techniques

- Sources gelées : application ba21a3c, SDK publié74e8a89.
- Précompilation : `.tmp/native-it-precompile-f2b93d6ed7f644e58f0a00b69dd7db32/`.
- Inventaire avant IT : `.tmp/auth-native-it-before-984d69e2-6359-4c4b-b4f1-c5138806fda3.json`.
- Rechecks source et limites : [registre des revues](../../../../.cursor/review-briefings/INDEX.md).

## Précompilation terminée

Les54 cibles ont maintenant toutes compilé avec succès. Le résumé et chaque log
ont été recoupés : aucun test exécuté, aucune erreur mémoire, sources inchangées.
La préparation a donc réussi sans réduire les24 jobs.

Je passe à la campagne réelle Hive/IAM autorisée. À sa fin, même en cas d'échec,
le runner arrêtera puis retirera uniquement ses fixtures jetables identifiées ;
leur état temporaire sera perdu. Les ressources préexistantes restent étrangères
et ne seront pas nettoyées. Les résultats fonctionnels restent à observer.
