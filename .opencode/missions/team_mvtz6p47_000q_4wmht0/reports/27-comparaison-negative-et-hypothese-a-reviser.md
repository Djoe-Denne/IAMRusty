# Échec aussi sur Ubuntu22.04 : l'explication doit être affinée

## Résultat nouveau

La comparaison ne démarre pas le corps du scénario sur Ubuntu22.04 non plus, alors que sa restriction AppArmor mesurée vaut0. Les109tests Linux passent sans skip sur chacune des deux plateformes. Les outils, versions et inspections passent ; les enfants du lanceur sont arrêtés correctement.

Ce résultat ne prouve pas l'absence de toute politique AppArmor, et ne réfute pas sa responsabilité possible sur24.04. Il montre surtout que « restriction globale à1 » n'est pas une explication suffisante de tous les échecs k6 observés. Aucune qualification Chromium/HTTP n'est obtenue et aucun artefact d'échec n'est publié.

## Recherche après cet essai

Le source k6v2.3.0 confirme que sa variable K6_BROWSER_ARGS vide est ignorée : ce n'est pas une instruction supprimant tous les arguments par défaut. La liste par défaut vérifiée n'injecte pas no-sandbox ou disable-setuid-sandbox. L'hypothèse simple d'un argument vide désactivant la sandbox n'est donc pas retenue.

Le parseur k6 peut toujours masquer le FATAL natif, et son code de sortie n'est conservé qu'en catégorie NONZERO. La table officielle épinglée distingue notamment configuration invalide, exception de script, seuil échoué et interruption : cette information peut être projetée sous forme fermée sans exposer de texte natif. Sur22.04, aucun lancement direct n'a encore été effectué : son erreur ne peut pas être déduite de celle de24.04.

## Prochain essai retenu

Compléter la comparaison déjà admise par son diagnostic direct22 facultatif, après refus k6 effectif et prérequis réussis, dans le même job/hôte/pins. Le lancement direct reste unique, séparé et jamais qualifiant. Conserver aussi une catégorie fermée du code k6 exact, sans l'appliquer à Chromium. Ce complément doit distinguer un refus natif du navigateur sur22 d'un problème propre au lancement k6 ; il ne change aucun réglage hôte et ne présume pas la cause.

Les gardes devront empêcher le direct si les prérequis ou le cleanup précédent n'ont pas réussi, ou si le cas a finalement été complet. La comparaison dispose déjà d'un résultat JSON fermé ; son usage local par le parent ne doit pas devenir un second framework ou une publication de flux natifs. Source/tests et contrôle ciblé avant publication ; aucun relancement identique à l'aveugle.

## Annexe technique

- Publication `20d495ee8d3547750c426f5da666a9aacee284bb` sur origin/master vérifiée après push exit0 ; source d47 inchangée après revue PASS indépendante. Index général local préservé et exclu du commit publié ; sous-index k6 seul publié.
- Run [40976916763](https://github.com/Djoe-Denne/IAMRusty/actions/runs/40976916763), même HEAD/événement push, deux jobs failure ;109tests22/1.639s/OK et109tests24/1.477s/OK, sans skip.
-22 : tools PASS ; attendu1/démarré0/achevé0/itération1/requête0, summary VALID, native NONZERO, renderer absent et namespaces NOT_OBSERVED ; SANDBOX_UNPROVEN/exit2, enfants cleanup PASS, parent INCONCLUSIVE.
-22 : AppArmor ENABLED, restriction ZERO, clone ONE, limite POSITIVE, parent seccomp/NoNewPrivs ZERO ; kernel6.8.0-1070-azure, UID1001. Versions exécutées2.3.0/156.0.8078.73 et hashes ELF concordants. Setup job annonce image20261025.340.1, environnement ImageVersion projette20261026.341.1 : conserver cette différence de sources, ne pas inventer une image unique équivalente à24.04.
-24 : même refus pré-cas et direct NO_USABLE_SANDBOX_REPORTED/SIGNAL/PRIVACY_FAILED qu'avant ; cleanup enfants PASS, aucun upload. Cette répétition est incidente au push et ne démontre pas une cause nouvelle.
- Reviewer pool14 arrêté normalement après persistance ; native merge sans branche. Arbres auteurs modifiés et sauvegardés encore présents ; aucun cleanup/archivage global acquis.

Sources relues après cet essai :

- https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/common/browser_options.go#L84-L104
- https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/chromium/browser_type.go#L439-L483
- https://github.com/grafana/k6/blob/v2.3.0/errext/exitcodes/codes.go
- Documentation officielle k6 sur les options navigateur, recoupée avec ce source épinglé ; les exemples no-sandbox ou d'images flottantes ne sont pas repris.
