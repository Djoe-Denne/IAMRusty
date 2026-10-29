# Prochaine comparaison : Ubuntu22.04 temporaire, sans changement de sécurité

## Objectif et choix

La qualification durable doit rester sur Ubuntu24.04, avec un vrai navigateur sandboxé. L'architecte recommande d'abord une comparaison temporaire sur Ubuntu22.04 : mêmes binaires et scénario, sans privilège ni modification de sécurité. Cette expérience peut établir une dépendance à l'environnement ; elle ne prouvera pas AppArmor seul et ne qualifie pas24.04.

Le lead ratifie cette seule comparaison dans le mandat de mission1, avant code. Les précisions sur le déclenchement CI et le nombre de lancements sont intégrées. L'exception AppArmor proposée comme voie durable reste non autorisée et ne fait pas partie de cette implémentation.

## Périmètre minimal

Une entrée diagnostique fermée, un cas k6/browser après vérification des outils et inspections, sans lancement Chromium direct supplémentaire demandé. Le parcours nominal24.04 reste inchangé. Même un cas et un témoin de sandbox réussis sur22.04 ne donneront pas de PASS HTTP/Chromium de mission : résultat non qualifiant et exit non nul, aucune publication de fichier.

Le workflow conserve son job24.04 et son échec visible. Le push normal nécessaire à l'expérience22.04 peut relancer incidemment ce job ; ce n'est ni un retry manuel ni un A/B à une seule variable. La référence historique reste le run40973768642. Aucun dispatch, nouveau secret, paquet, service, tunnel ou politique hôte ajouté.

## Vérification prévue

Même auteur pour les fichiers couplés, avec tests du sélecteur de plateforme fermé, refus des commandes nominales sur22.04 et impossibilité de produire un résultat qualifiant. Les gardes existantes de pins, confidentialité, identité, sandbox et cleanup restent nécessaires. Une revue indépendante bornée vérifiera le nouveau chemin de contrôle après intégration ; pas de réaudit global imposé.

## Preuves et limites

- Candidat final architecte `result_mvu6v2hg_000n_ev67mv`, session `ses_e75fb3daeffeCew1RlUmBKejVR` :123lignes ajoutées au seul contrat, diff-check auteur exit0. Récupération vérifiée identique par SHA256 puis sauvegarde de la proposition originale avant ratification.
- Comparaison22.04 Accepted par le lead avant code ; voie durable AppArmor Proposed et non autorisée. Briefing d'auteur préparé, admission après commit de ces entrées.
- Aucun nouvel essai lancé. Ubuntu22.04 est en dépréciation et ne devient pas la cible durable. Le refus Chromium24.04 reste non résolu ; HTTP reste sans cible conforme autorisée.
