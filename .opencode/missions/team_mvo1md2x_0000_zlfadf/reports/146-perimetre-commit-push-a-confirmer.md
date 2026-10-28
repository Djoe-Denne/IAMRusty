# 146. Confirmer les travaux à publier sans mélanger les missions

## Objectif et état observé

Demande : « commit all + push ». Aucun stage, commit ou push encore effectué.
Le worktree courant Telegraph est propre, sur sa branche de livraison déjà
publiée. La bibliothèque et le wiki de cette session sont dans l'arbre principal,
sur master, avec de nombreux autres changements de configuration et de code Rust
(Lazaret, Manifesto, contrats et opérateur Apparatus), des artefacts d'autres
missions, le sous-module dirty et le dossier .worktrees non suivi.

Preuve : git branch/status des deux arbres au début de cette demande. La QA de
cette session couvre la bibliothèque (49 PASS et smoke Context Mode), pas tous
les changements actuellement présents dans l'arbre principal.

## Décision nécessaire

- Bibliothèque et documentation de cette session : sélection explicite des
  fichiers liés, autres travaux inchangés. Recommandé pour ne pas publier des
  travaux concurrents non validés ici.
- Tout le code/configuration de l'arbre principal : inventaire et contrôle des
  fichiers à publier avant commit/push ; les validations absentes restent absentes.
  Ne signifie jamais ajouter aveuglément worktrees imbriqués, secrets ou données
  temporaires. Le sous-module doit être examiné séparément si ce périmètre est choisi.

La branche de destination doit aussi être explicite : branche dédiée recommandée
ou master sur demande. Aucun changement utilisateur déplacé, reset ou stash.
Le détail du choix sera ajouté au ledger après réponse.
