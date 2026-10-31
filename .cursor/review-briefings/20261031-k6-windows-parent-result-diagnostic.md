# Diagnostic borné après le premier vrai Chrome Windows

## Faits observés

Après correction du préfixe k6.exe, le lead a relancé exactement le mode browser.
Les deux inspections passent avec les options prévues. Le vrai navigateur démarre,
le quatrième enfant k6 finit avec exit0, capture complète et stderr vide. Son job
contient10processus observés (1k6 +9Chrome), tous terminés, active0 et cleanuptrue.
Les trois enfants k6 préalables ont également exit0/cleanuptrue. Inventaires avant/
après :0/0 k6/Chrome. Aucun GET applicatif.

Le parent rend pourtant exit2/PRECONDITION_REFUSED, versionk62.3.0 présente,
versionChrome non projetée, cleanupPASS/privacyPASS. La cause exacte n'est pas
encore connue : elle survient après le quatrième enfant et avant la projection
positive du résumé/version. Ne pas inventer une cause de sandbox ou de k6.

Les diagnostics child sont vides ou valides ; l'exception PowerShell courante n'est
pas enregistrée et le résumé a été retiré par le cleanup d'échec. Cette perte de
diagnostic est observée, pas un risque hypothétique.

## Travail autorisé — même auteur23/session/tâche/modèle

Conserver la correction du préfixe et ne pas modifier le cas JS, les pins, le sandbox,
le timeout, le processus ou les verdicts pour obtenir un vert. Pas de nouvelle enquête
générale, protection, revue, framework, runtime ou vrai outil par l'auteur.

Ajouter seulement le diagnostic privé minimal nécessaire au catch du parent :
type d'exception, identifiant d'erreur et numéro de ligne de script, plus présence/
taille du résumé. Pas de texte de stack, environnement, arguments, SID ou fluxFATAL.
Conserver sur échec une copie bornée du résumé courant s'il existe (64KiB maximum)
comme diagnostic privé, avant son retrait normal avec les autres données.

Étendre la seule allowlist privée de retrait aux noms fermés `parent-error.json`
et `parent-summary.json`. Ces fichiers ne sont jamais publiés et n'influencent pas
le PASS. Leur écriture ne doit pas remplacer l'erreur originale si elle échoue.
Garder profiles/sources retirés après fin prouvée ; si processus encore actif ou
cleanup non prouvé, ne pas supprimer ses données.

Test ciblé sur des données inertes connues du catch/conservation et erreurs utiles,
sans nouvelle fixture Child. Livrer immédiatement le petit delta+SHA et résultats
réels. Le lead rejouera ensuite le même cas pour obtenir la cause discriminante.
Si la source démontre déjà la cause exacte, la signaler avec preuve, pas de supposition.

## Références et portée

Contrat Windows révision3Accepted, rapport97, ledger canonique. Base canonique
source :7e3415f + deux fichiers de correction prefix byte-identiques au candidat
`result_mvw6ydwm_0001_z4ah9h`. Preuve privée du run :
`C:/Users/djden/source/repos/AIForAll/.tmp/k6-browser-r3-20261031-retry1-lease.json`.
Les journaux sont privés ; ne pas les republier dans le candidat. Aucun nouveau GO
utilisateur ni changement de politique QA requis : diagnostic de l'échec effectivement observé.
