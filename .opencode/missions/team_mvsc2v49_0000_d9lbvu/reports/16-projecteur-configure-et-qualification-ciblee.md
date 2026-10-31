# Projecteur configuré, reprise des contrôles ciblés

La mission vise toujours à fermer les frontières d'authentification puis à les
qualifier réellement. Le défaut de configuration du projecteur de droits est
corrigé : le profil par défaut déclare désormais explicitement l'autorité attendue,
sans valeur implicite permissive. Les deux profils rendus et le parseur Rust réel
ont été testés. La revue de sécurité relit ce correctif dans la même session.

Le complément SDK de deux lignes de fixture a été publié sur la branche de
mission et sélectionné, après ses deux revues et les63 tests HTTP déjà valides.
La configuration utilisateur étrangère du sous-module reste inchangée.

## Vérifications acquises

- Le nouveau test de configuration échoue sur le profil initial sans policy ; après correction, les11 tests d'assemblage passent.
- Le parseur Rust accepte le fichier réellement monté, valide son autorité et refuse toujours l'absence de policy : un test réussi, aucun ignoré,49 hors filtre. Ce test ne prouve pas encore la projection ou la révocation en fonctionnement.
- Ce binaire ciblé a compilé avec24 jobs sans l'erreur mémoire du gros lot précédent. Aucun réglage mémoire, nombre de jobs, cache ou processus étranger n'a été modifié.
- Formatage global du dépôt et contrôle du diff réussis ; commit normal avec hook de formatage exécuté.

## Travaux restants

La correction du lien de synchronisation et les quatre renforcements de tests
sont en cours dans deux worktrees séparés. Leurs plans ont été examinés et
approuvés ; aucun runtime ou nouveau Cargo ne leur est accordé pour l'instant.
La qualification reste bloquée jusqu'à intégration et exécution des oracles
nécessaires. IT réelles avant Kind, puis vrai Envoy et navigateur : aucun de ces
résultats n'est anticipé par les succès statiques.

## Traçabilité

- Commit intégré :73e50d30619fde716f4053bc46fc643e19c3d979, cinq fichiers explicitement sélectionnés.
- SDK publié :74e8a89cfd15384d8b20725c96d0cf72e295b1ee, avance rapide depuis77672b8 ; SHA distant exact vérifié. Un fichier de tests, deux champs ajoutés, aucune production/assertion modifiée.
- Log du test ciblé : `.tmp/auth-s2-sentinel-3f5b713fc4ae4615b8b7349fca0ef70e.log`, exit0,24.65s compilation. Source8fa+correctifS2 et SDK74e8, puis commit73e50.
- Recheck S-2 demandé au reviewer initial ; S-1 toujours ouvert. Findings finaux sécurité/tests persistés et indexés avant correction.
- Recheck indépendant reçu et persisté : **S-2 clos statiquement**, aucun nouveau finding ciblé. S-1 et qualification live toujours ouverts. Référence : [relecture de la configuration du projecteur](../../../../.cursor/review-briefings/20261028T2238Z-security-sentinel-policy-recheck-73e50d3.md).
- Ressources de mission créées : aucune Docker/Kind/WSL. Un processus WSL préexistant de propriété inconnue a seulement été observé et reste intact.
