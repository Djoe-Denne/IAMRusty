# Correctif intégré : cinq tests verts, revue sécurité en cours

Le correctif d'entrée réseau est intégré, sans changer l'authentification ni
les autres règles. Les cinq tests ciblés réussissent, dont celui qui accepte
la redirection HTTP303 attendue et refuse les autres réponses en retirant seulement
la règle ajoutée.

La revue sécurité indépendante porte sur le code intégré et sur l'opération
préparée pour l'appliquer. Aucune règle réseau n'a encore été modifiée.

La préparation des preuves a également été vérifiée : les originaux restent
inchangés ; une copie exacte des sept documents utiles a été rendue privée et
non modifiable. Le contrôle de provenance refuse correctement le format de
fichier différent du checkout principal. La validation finale utilisera le
checkout de livraison, dont le format correspond aux preuves, sans assouplir
ce contrôle ni modifier les documents d'origine.

Après revue et livraison qualifiée, je poursuivrai le contrôle réel de l'entrée,
puis les preuves Envoy et navigateur. Aucun nouveau GO de routine n'est requis.

Mise à jour : la livraison locale est maintenant qualifiée. Le contrôle des
sept documents réussit depuis le checkout de livraison réellement utilisé ;
leurs contenus et empreintes restent identiques aux originaux. Le hook normal
de formatage Rust réussit également depuis le dépôt principal, sans contourner
l'échec de chemin long rencontré dans le worktree de l'auteur. La revue sécurité
reste le préalable à toute application réseau.

## Preuves techniques

- Tests intégrés : `python -B -m unittest discover -s ops/tests -p test_local_full_envoy_ingress.py -v`,
  exit0, 5PASS, aucun skip, 2.250s.
- Deux fichiers strictement identiques au candidat livré ; seize définitions
  préexistantes inchangées ; contrôle de diff réussi.
- Hook auteur : échec Windows OS206, sans contournement. Intégration par patch
  contrôlé après constat natif d'absence de commit à fusionner ; WIP conservé.
- Le refus local de provenance porte sur les fins de ligne du helper importé,
  pas sur un changement du JSON de preuve ou un résultat réseau.
