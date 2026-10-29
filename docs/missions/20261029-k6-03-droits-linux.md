# Ordre de mission3 — Qualifier propagation et reprise des droits avec k6, entièrement sous Linux

**Préparation seulement. Dépend des missions1 et2 intégrées.** Objectif fonctionnel :
les bonnes personnes obtiennent les bons accès, les autres restent refusées,
même après retard, redémarrage, duplication ou rejeu des changements de droits.
Ce n'est ni une campagne de charge ni un audit exhaustif de tous les services.

## Contrats et prérequis

Lire [0504](../adr/0504-validation-fonctionnelle-linux-k6.md),
[contrat k6](../contracts/k6-functional-linux.md),0314 et
[frontière de confiance](../contracts/authz-trust-domain-boundary.md), événements/
outbox/queues0200–0202 et les contrats de projection effectivement concernés.
Statuts existants conservés ; pas promotion0314 par la seule présence de tests.
Tant que le graphe réel des événements et ses garanties d'ordre/rejeu ne sont pas
confirmés, architecte tranche les oracles AVANT stimuli/code de scénario.

Tout le dispositif de cette campagne tourne sous Linux : k6, contrôleur de phases
et services déployés. Cible observée et possédée, parentops utilisable sousLinux,
réseau runner vérifié ; pas appel PowerShell Windows caché pour contrôler les
pannes. Une adaptation Linux mince des gardes est livrée/vérifiée avant ses
consommateurs ; absence de garde bloque l'action, pas contournement par kubectl nu.
La règle des UT/IT Rust de développement Windows n'est pas remplacée par ce lot.

Chaîne réelle représentative : APIHive → événement/outbox → transport explicitement
activé → sentinel-sync → OpenFGA réel → accès métier via passerelle. Aucun mock
de projection ou insertion directe de grants à la place de cette chaîne. Les
deux autorités doivent être reconnues par l'authentification, pas rejetées avant
le contrôle des permissions. Préparer les credentials via mécanismes approuvés,
pas nouvelle API de mint org ni suppression de garde pour fabriquer un témoin.

## Scénarios et oracles obligatoires

1. **Accorder** : propriétaire autorisé donne le droit àA ; A finit par accéder à
   l'objet précis, B reste refusé ; preuve que l'événement/projection a traversé
   la chaîne et pas un ancien grant déjà présent.
2. **Retirer** : retirer le droit deA le rend refusé dans la borne de propagation/
   cache prévue. Une ressource témoin reste autorisée pour distinguer panneglobale
   et révocation effective. B ne gagne jamais de droit indirect.
3. **Séparer les autorités** : même sub dans deux issuers valides ; droit deA
   ne s'applique pas àB, et chaque autorité possède un témoin légitime positif.
   Vérifier avant effets/RPC non autorisés ; pas comparaison de deux jetons dont
   l'un serait simplement invalide à l'entrée.
4. **Retard** : pause bornée du consommateur possédé, changement enregistré,
   reprise puis convergence attendue. Ne pas affirmer révocation immédiate pendant
   la pause si le modèle autorise l'ancienne projection jusqu'à la borne ; cette
   politique doit être fixée au contrat, pas inventée par le test.
5. **Redémarrage** : changement en attente puis arrêt/redémarrage gracieux du
   consommateur exact, traitement repris sans perte ni octroi au mauvais principal.
6. **Duplication** : livraison répétée du même événement ne crée pas de droit
   supplémentaire ; résultat final et cible restent stables.
7. **Rejeu après retrait** : un ancien événement d'attribution ne doit pas
   ressusciter le droit retiré, selon les garanties contractuelles de version/ordre.
   Définir avec architecte un stimulus réalisable avec les mécanismes existants.
   Si la propriété n'est pas satisfaite, conserver ROUGE et traiter le défaut ;
   ne pas « prouver » le cas en le remplaçant par le rejeu inoffensif d'un retrait.

Le parent contrôle pause/restart/replay et observations sous baux ; k6 ne détient
ni kubeconfig ni pouvoir générique d'administration. Polling borné sur état
attendu, pas sommeil fixe utilisé comme preuve. Les délais sont contractuels :
pas de transposition des60sJWT en délaiOpenFGA. Aucun retry d'une mutation non
idempotente ou transactionOAuth déjà consommée pour masquer un échec.

## Livrables et périmètre

Scénarios `ops/tests/k6/scenarios/authz/`, jeux de données possédés parrun, helpers
de corrélation réutilisés, contrôlesLinux ciblés, manifeste des cas et rapport
métier lisible. Un témoin consommateur limité peut utiliser Manifesto ; cela
n'impose pas sa suite complète ni sa réparation de compilationWindows.
Pas d'ajout d'un protocole d'événements, schémaDB, relationFGA ou framework de
replay sous couvert d'écrire des tests. Une telle nécessité ouvre un lot produit
avec architecte/contrat/QA, séparé et approuvé, avant correction.

## Acceptation et QA

Chaque cas a un positif, un négatif, la preuve de projection et la même identité
de cible. Tous les cas attendus achevés, aucun skipmasqué, seuilschecks fermés,
observations bornées et cleanup complet. La panne du dispositif est INCONCLUSIVE,
pas une régression fonctionnelle. Aucun succès sans consommateur réellement actif.
Revue indépendante ciblée sécurité/oracles de la nouvelle surface et vérification
infra après changements significatifs, pas pipeline de quatre agents obligatoire.

Préparer/configurer la mission avec ses commandes/cibles/bornes avant start.
Ledger `.cursor/handoffs/20261029-k6-03-droits-linux.md` au lancement. Ressources
créées enregistrées àcréation, IDs exacts/baux, restauration et stop gracieux
sur succès/échec ; données étrangères, caches et volumes préservés, aucunprune.
DetteHSM reconnueacceptée/nonbloquante, HTTPSpublic contrôlede déploiement,
interfaceutilisateur/performance/OHA/HAcomplète horscampagne. Une campagneverte
qualifie cette chaîne et ces oracles, pas toutes les projections possibles.
