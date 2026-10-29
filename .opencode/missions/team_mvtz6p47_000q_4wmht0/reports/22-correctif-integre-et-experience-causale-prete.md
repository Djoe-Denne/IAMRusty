# Correctif intégré ; expérience causale prête

## Résultat vérifié

Le dépassement des flux est désormais conservé indépendamment du diagnostic final de cleanup. La classification directe ne voit plus un préfixe tronqué comme une preuve de confidentialité. Les limites, priorités d'erreur, identités et règles d'arrêt/sandbox restent inchangées.

Le lead a comparé les cinq fichiers correctifs au candidat puis exécuté96tests Windows :89exercés avec succès,7spécifiques POSIX ignorés, exit0. Les régressions utilisent le vrai chemin Child.execute/select/read/troncature avec frontières OS simulées ; elles ne créent pas de processus survivant. Le commit et son vrai hook réussissent.

## Portée QA et suite

La revue indépendante a donné PASS sécurité au nouveau chemin direct. Son point non bloquant d'exactitude a été traité par ce correctif mécanique, inspecté et testé par le lead ; pas de nouvel audit général ni pipeline de rôles imposé. Le replay Linux du contenu conservé sera effectué par le même workflow natif.

L'expérience suivante comporte le cas k6 existant et, seulement après son échec et le succès des outils, un lancement direct explicite du même Chromium. Hypothèse issue des recherches : le parseur k6 peut perdre une erreur native de démarrage. Le diagnostic comparera les sources et réglages hôte lus, sans changer ces réglages ni transformer le diagnostic en qualification.

## Preuves et limites

- Correctif05e25e7a61c1f6183d0706d1f4b6f3051a56f34c, basee9af3b6 ; cinq fichiers sousops/tests/k6, aucun changement du workflow dans ce correctif.
- Candidat `result_mvu6bzpt_000g_i8edej`, même auteur/session ; ROUGE96/1échec3erreurs/skips7 attribué à l'auteur. GREEN96/skips7 réexécuté par le lead.
- Recherche motivant l'essai dans rapports18–19 et README ; addendum Accepted avant code. Aucun essai identique répété sans hypothèse.
- Aucun nouveau runtime local, paquet, privilège ou modification de politique hôte. Chromium/HTTP restent non qualifiés avant résultat de l'expérience.
