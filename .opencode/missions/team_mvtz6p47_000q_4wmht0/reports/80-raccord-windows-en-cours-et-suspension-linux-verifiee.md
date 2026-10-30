# Raccord Windows en cours, suspension Linux vérifiée localement

Le but est de réussir les tests minimaux k6/Chrome sousWindows, sans relancer
l'enquêteWSL ou une campagneLinux. Le contrat accepté et le brief ont été intégrés
avant la création de l'auteurWindows isolé. Sa tâche est en cours sous la révision
approuvée, avec les tests ciblés du nouveau code, pas encore un essai navigateur.

## Deux branches indépendantes

- L'auteur écrit le lanceurWindows, ses contrôles et ses tests, sans exécuter les
  vrais k6/Chrome ou le GETapplicatif. Il n'utilise pas les archivesWSL comme code
  actif et ne possède ni l'acquisition des outils ni le workflow.
- Le lead a préparé et vérifié la suspension des deuxjobsLinux, et lancé
  l'acquisition explicite des archivesWindows officielles. Les outils ne sont
  ni installés automatiquement ni exécutés par ce téléchargement.

La comparaison avec le workflow intégré confirme que seuls son préfixeSUSPENDED,
deux commentaires et les deux conditions de job toujoursfausses ont changé.
IDs, nomsdeschecks, filtres, actionsépinglées et permissions sont conservés.
Ce contrôle statique passe ; aucun job distant n'a été déclenché. La suspension
reste locale tant que la publication n'est pas faite. Les futurs jobs ignorés ne
constitueront pas une qualification, même si GitHub afficheSuccess.

L'acquisition des archives est enarrière-plan, sans relance ou lecture répétée
pour attendre. Les empreintes et l'architecturePE seront vérifiées sans lancer
les exécutables ; le résultat n'est pas encore acquis. Les consommateurs de
preuve réelle attendront code intégré, verrou de versions et contrôles utiles.

## Limites actuelles

Aucun testk6/ChromeWindows, GET/iam/health ou nouvellepreuve d'isolation réelle
n'est encore exécuté. WSL reste suspendu/inconclusif, donnéesconservées. HTTP
restera inconclusif sans cibleexistanteadmise. AucunDocker/Kind, nouveauLinux,
VMGitHubActions, tunnel, dispatch ou push implicite.

## Annexe technique

- Base intégrée avant auteur : `2cc7ae6`, huitfichiersdocumentaires, vrai hook
  cargoFMT et diffcheckcached0 ; contratWindows Accepted/Unimplemented.
- Tâcheauteur `task_mvvakq7y_000t_eau9qs`, profilhard-implementer natifpool20,
  branche `ensemble-812055e4-6a4a-4b0b-9045-880dd255d8ac`, contextefresh,
  ownership/completionlead et QAproportionate ; aucun modèle/scoreoverride.
- Contrôlesworkflow : comparaison exacte au blob2cc7ae6, deuxconditions
  `if: ${{ false }}`, aucun changementnonallowlisté ; gitdiffcheckglobalexit0.
- Acquisition nonexécutante : commandehôte `sh_18e093644001kt2gefCJ3c2PCY`,
  `.tmp/k6-acquire-windows-assets-20261030.py`, syntaxe vérifiée avant lancement.
  SourcesofficiellesAPI/checksums/CfTversionnés, limitesURL/taille/timeout,
  aucuneextraction/exécution ni logsprivés publiés ; notificationàattendre.
