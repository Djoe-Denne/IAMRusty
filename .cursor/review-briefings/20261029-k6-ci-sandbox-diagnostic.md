# Diagnostic borné — Chromium sans preuve sandbox sur Linux natif

## Faits

Le commit `919f1bde047ebff0ca50c6edc6d77f8453ce938b` est publié et la référence origin/master vérifiée. Run GitHub Actions `40963859987`, événement push, workflow k6 dédié : tests locaux, acquisition, versions et inspect réussis. Étape browser exit2, diagnostic SANDBOX_UNPROVEN, cleanup PASS ; artefacts non publiés. Aucune réussite Chromium ou HTTP.

Le stderr natif est volontairement privé ; la CI ne fournit donc pas la cause précise. Le code source k6 v2.3.0 browser_type.go:439–478 ne définit pas de flag no-sandbox/disable-setuid-sandbox par défaut. Une politique de namespaces/AppArmor du runner est une hypothèse, pas encore un fait mesuré. L'absence de libnspr4.so sur WSL est indépendante ; versions et inspect réussissent sur le runner hébergé.

## Changement local autorisé à l'auteur

Même session, ownership ops/tests/k6/**. Améliorer uniquement le diagnostic fermé d'échec sandbox à partir des sorties déjà capturées, sans flux brut ni debug. Une correspondance bornée et testée sur messages connus de refus sandbox/namespaces peut distinguer la cause tout en conservant le verdict non réussi. Un diagnostic ne prouve pas à lui seul que la sandbox est active.

Recherche officielle Chromium/Ubuntu et k6 épinglé avant choix de motifs ; tracer sources retenues. Tests négatifs de confidentialité et messages inconnus/sentinelles : aucun chemin, environnement ou stderr brut publié, aucun échec transformé en PASS. Le superviseur conserve le bail et le prochain essai CI par push normal après intégration ; aucun worker ne lance CI/runtime. Pas de configuration de sécurité modifiée, installation de paquet, wrapper de navigateur, fallback, sysctl, AppArmor, no-sandbox, nouvel environnement ou nouveau harness de preuve. Ne pas modifier le workflow hors nécessité démontrée.

## Résultat attendu

Petit delta source/tests/doc, diagnostic exact/limites, commandes réellement effectuées sous Windows seulement. Ne pas chercher à rendre le runner vert par affaiblissement. Aucun replay aveugle du même échec : le prochain essai doit fournir l'information discriminante manquante. Si une preuve de cause est impossible sans extension, rapporter la limite plutôt que fabriquer une autorité.
