# 75 — Publication des livrables durables : contrôle et limites

## Conclusion

La publication a bien inclus les corrections applicatives et l'instrumentation
intégrées, pas seulement le wiki. Mais l'affirmation « toute la documentation
durable est publiée » n'est pas encore démontrée : les ledgers de transmission et
plusieurs comptes rendus finaux sont volontairement restés locaux lors du dernier
push. Une exclusion par dossier technique ne suffit pas à classer leur contenu.

La règle utilisateur est désormais explicite : conserver dans master le code,
la documentation et l'instrumentation durables ou réutilisables, y compris produits
dans un worktree ; exclure les outils jetables de campagne, données générées,
logs et sorties temporaires. Les dossiers Git des worktrees eux-mêmes ne doivent
pas être ajoutés. Un test de régression ou une fixture source réutilisable ne se
confond pas avec les données jetables d'une exécution.

## Vérifications réalisées

- 18 worktrees natifs encore présents de la mission authentification examinés :
  statuts, fichiers modifiés non committés et chemins des changements committés.
  101 occurrences de chemins dans les deltas committés comparées à master.
- Les fichiers source non committés observés existent tous dans master ; 12
  occurrences sont identiques en normalisant CRLF. D'autres restent différentes,
  notamment des propositions historiques ou des versions antérieures : leur
  existence dans master n'est pas à elle seule preuve d'équivalence fonctionnelle.
- Deux chemins committés absents de master : proposition ADR0315 remote et sa page
  wiki. Le ledger canonique explique explicitement son rejet au profit du contrat
  retenu : limites Sign4096/public16384, pas nouvelle ADR0315. Ce n'est pas un
  livrable accepté oublié ; ne pas réintégrer cette alternative par automatisme.
- Exemples d'évolution vérifiée : ADR0314 proposée dans son worktree mais ratifiée
  dans master ; tests d'ingress améliorés dans master après leur livraison auteur ;
  écart Manifesto configuration/src/lib.rs sans diff Git textuel (fins de ligne).
- 14 arbres isolés sous `.worktrees/` sont propres. Les quatre arbres SDK ciblés
  ont leur HEAD ancêtre du pin publié02e5. Deux arbres applicatifs sont ancêtres
  de master ; huit arbres Telegraph/HTTP ont une histoire séparée après intégration
  squash : 42 occurrences de chemins changés, aucune absente de master. Une
  comparaison fonctionnelle exhaustive de ces anciennes branches n'a pas été faite.
- Aucun fichier suivi sous `.tmp/` ou `.worktrees/`, aucun fichier `.log` suivi.
  Cela ne constitue pas une recherche exhaustive de toute donnée de test possible.
- 226 fichiers déjà suivis sous `.opencode/missions/`, principalement215 rapports
  Markdown et des plans/politiques ; ce n'est pas226 logs runtime. Dans la plage
  publiée lors du dernier push, seul le rapport71 y a été ajouté.
- Les trois ledgers récents auth/k6/publication sont ignorés par Git ; aucun ledger
  sous `.cursor/handoffs/` n'est suivi. Leurs conclusions principales sont reprises
  dans les contrats/wiki, mais tous leurs détails durables ne sont pas audités.

## Suite justifiée

Réconcilier le contenu permanent des comptes rendus/ledgers encore locaux avec
les documents canoniques, puis publier uniquement les compléments utiles et
assainis. Achever au besoin les comparaisons de contenu des branches conservées,
sans remettre des versions périmées par-dessus les corrections intégrées.
Ne pas utiliser `git add -A`, ni fusionner toutes les branches indistinctement.

Pas de modification applicative, nouveau test ou opération runtime pour cet audit.
Portée : mission auth actuelle et14arbres isolés locaux ; les autres anciens
worktrees enregistrés (87 au total dans le dépôt) ne sont pas tous audités.
Master distant vérifié précédemment : bf41bd9. Aucun nouveau commit/push dans
ce contrôle. La précédente affirmation d'exclusion des « fichiers étrangers »
ne doit pas être interprétée comme une garantie d'exhaustivité des livrables.
