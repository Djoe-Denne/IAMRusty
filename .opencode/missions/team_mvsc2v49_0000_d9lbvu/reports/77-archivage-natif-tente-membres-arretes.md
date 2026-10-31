# 77 — Archivage natif tenté : membres arrêtés, worktrees encore protégés

## Demande et résultat concret

L'utilisateur indique que l'archivage devrait désormais être possible. La procédure
native a donc été réévaluée avec la version chargée du skill, sans se limiter à
l'ancienne conclusion du rapport72. **La mission n'est pas encore archivée.**

Premier `team_cleanup({acknowledge_uncommitted:true})` : refus avant nettoyage,
12 membres encore actifs. Les12 membres ont ensuite été arrêtés avec
`team_shutdown`, sans force. Chaque appel confirme leur arrêt ; aucun nouveau
worker ni travail métier n'a été lancé.

Second appel, `team_cleanup({})`, après leur arrêt : avertissement pour11 worktrees
contenant des modifications non committées. Le nettoyage n'a pas été poursuivi.
Cette seconde tentative testait le nouveau prérequis satisfait (arrêt des membres)
avec l'avertissement de contenu activé, pas une répétition forcée du premier appel.

## Limite actuelle observée

Le catalogue expose `team_cleanup`, pas un outil d'archivage distinct. La version
du skill chargée est `20261029-worktree-retirement-cc8c9a9ad5b1`. La documentation
et l'implémentation livrées ont été lues uniquement pour comprendre les effets :
cleanup peut fusionner les branches oubliées, retirer les worktrees puis archiver
en absence de problèmes. Le booléen acknowledge ne promet pas une conservation
en place ; il désactive un avertissement. Les chemins de retrait sont vérifiés,
mais une nouvelle capacité de retrait n'est pas un archivage préservant les arbres.

Certains arbres contiennent des propositions remplacées ou rejetées et des
copies anciennes, dont la réintégration serait contraire à l'audit déjà publié.
Leur présence n'autorise donc ni fusion automatique globale, ni commit étranger,
ni retrait forcé pour satisfaire l'outil. Aucun réglage de mergeOnCleanup, état
interne, fichier du plugin ou base native n'a été modifié.

## Condition de finalisation

Une opération native explicitement exposée qui archive en conservant les branches
et arbres non committés, ou un traitement distinct et sûr de ces11arbres avant
cleanup. Ce rapport ne demande pas une nouvelle approbation du travail publié ;
il identifie le comportement encore nécessaire pour l'archivage demandé.

Master publié reste052817d ; les12 arrêts de membres sont le seul changement
d'état opérationnel de cette intervention. Aucun test, runtime, commit, push,
suppression ou purge effectué.
