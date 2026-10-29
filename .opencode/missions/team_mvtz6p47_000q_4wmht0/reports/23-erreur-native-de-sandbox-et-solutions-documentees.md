# Chromium révèle une sandbox indisponible ; choix du prochain essai

## Objectif et résultat nouveau

La mission doit qualifier un vrai Chromium Linux avec sa sandbox, sans affaiblir les protections. L'expérience comparative a révélé le message natif « No usable sandbox » que k6 ne montrait pas. Le démarrage reste refusé : aucun PASS navigateur ou HTTP n'est acquis.

Les 96 tests Linux réussissent sans skip ; les outils, versions et deux inspections passent. Le contrôle des processus enfants rapporte cleanup PASS pour les deux parcours. Cela ne qualifie pas le nettoyage du runtime parent. Le diagnostic direct échoue aussi au contrôle de confidentialité : seule sa catégorie fermée est visible, aucun flux natif ou artefact d'échec n'est publié.

## Faits et interprétation

Sur le même hôte et les mêmes binaires, AppArmor est activé ; la restriction AppArmor des namespaces non privilégiés vaut1, leur création générale vaut1 et leur limite est positive. Le parent n'a pas de filtre seccomp. k6 échoue avant le corps du cas ; Chromium direct signale une sandbox inutilisable puis termine par signal. Un signal peut aussi résulter du superviseur : pas de numéro ou cause de crash inventé.

Ces faits concordent avec l'incompatibilité documentée de Chrome for Testing extrait dans un chemin privé sur Ubuntu24.04. Ils ne constituent pas une mesure du refus AppArmor appliqué à ce processus, ni une preuve que cette cause est exclusive. Aucun journal noyau, profil ou réglage hôte n'a été modifié ou sondé hors contrat.

## Recherche après cet essai

La documentation officielle Chromium156.0.8078.73 décrit trois solutions : désactivation globale de la restriction, profil AppArmor ciblé, helper SUID. La première affaiblit une protection utile et reste exclue. Le profil ciblé reste une exception à la politique hôte, pas un changement neutre ; le helper implique un binaire privilégié et un mécanisme ancien. Aucun de ces remèdes n'est appliqué implicitement.

Le guide officiel Puppeteer recoupe l'erreur et les restrictions Ubuntu23.10+. Il avertit aussi que sa section sur le helper SUID est en grande partie obsolète. Un retour à Ubuntu22.04 pourrait discriminer une dépendance à l'environnement sans modifier la politique de l'hôte, mais ne serait pas une comparaison équivalente ni une bonne cible durable par défaut : GitHub annonce son retrait le17avril2027, avec dépréciation depuis septembre2026.

## Décision de suite

Ne pas relancer le même essai ni appliquer un remède privilégié pour obtenir du vert. Consulter l'architecte sur le plus petit essai utile et la voie durable : le contrat fixe actuellement Ubuntu24.04 et interdit les changements de politique dans le diagnostic. La recommandation est de préserver ces limites et de préférer d'abord une comparaison sans privilège si elle reste informative ; toute exception de politique doit être explicitement arbitrée avant code.

## Annexe technique et preuves

- Publication vérifiée : `cf9f5ca467ad43e2cac3877314d8ca09f5d28884` sur `origin/master`, push exit0 et ls-remote concordant. Hook du commit documentaire exécuté normalement ; `git diff --check` exit0.
- Run [40973768642](https://github.com/Djoe-Denne/IAMRusty/actions/runs/40973768642), événement push, même HEAD ; job123796669490 terminé failure. Image Ubuntu24.04 `20261025.353.1`, pas la version de provisionneur `20261021.618`.
- `python3 -B -m unittest discover -s ops/tests/k6/tests -p 'test_*.py'` :96tests/1.364s/OK, aucun skip. Acquisition et étape outils success ; k6/browser exit2, diagnostic direct exit1 ; publication skipped.
- k6 : summary VALID, attendu1/démarré0/achevé0/itération1/requête0 ; renderer absent, lectures de namespaces NOT_OBSERVED ; `SANDBOX_UNPROVEN`, cleanup enfants PASS.
- Direct : `NO_USABLE_SANDBOX_REPORTED`, native_exit SIGNAL, termination EXITED ; `PRIVACY_FAILED`, confidentiality FAIL, cleanup enfants PASS. HTTP/Chromium INCONCLUSIVE ; parent_runtime_cleanup INCONCLUSIVE pour les deux.
- Logs lus et filtrés dans Context Mode ; erreurs de syntaxe des scripts d'extraction corrigées avant lecture, sans nouveau test ni panne MCP. Aucun runtime local créé/redémarré. Arbre pool9 arrêté, native merge sans commit ; proposition originale déjà récupérée, vérifiée, sauvegardée et ratifiée dans14d55e9, arbre modifié encore préservé.

Sources consultées après le run :

- https://chromium.googlesource.com/chromium/src/+/156.0.8078.73/docs/security/apparmor-userns-restrictions.md
- https://raw.githubusercontent.com/puppeteer/puppeteer/main/docs/troubleshooting.md
- https://github.com/actions/runner-images/issues/14254 (annonce officielle du retrait Ubuntu22.04, contenu API vérifié)
- https://github.com/actions/runner-images/blob/main/README.md

Les guides non épinglés décrivent le contexte courant, pas le comportement prouvé de notre binaire ; la documentation Chromium épinglée et les observations du run priment.
