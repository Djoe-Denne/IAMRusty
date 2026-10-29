# Contrat proposé — scénarios fonctionnels k6 sous Linux

2026-10-29 — **Proposed / Unimplemented**, décision structurelle architecte,
transcrite par lead. Lié à [0504](../adr/0504-validation-fonctionnelle-linux-k6.md).
Ratification avant implémentation en mission1 ; aucun démarrage par ce document.

## Structure V1 et responsabilités

Placement proposé à confirmer à la ratification : `ops/tests/k6/` avec `README.md`,
`config/`, `lib/`, `scenarios/smoke/`, puis `scenarios/auth/` et `scenarios/authz/`.
Créer seulement les dossiers/fichiers utiles à la mission courante. Données de
campagne privées hors Git ; exemples de configuration sans credentials.

Parent ops → k6 : configuration validée, identité de cible observée, identifiant
de run, paramètres de phase et références privées aux credentials. Ce manifeste
ne prouve pas à lui seul la propriété de la cible.
k6 → parent : assertions identifiées, compteurs et résultat de phase non secrets.
Parent seul : préflight, préparation de données possédées, pause/restart ciblés,
stimulus de duplication/replay, certificat mauvais nom puis restauration, collecte
des observations et cleanup. Réutiliser les mécanismes existants ; une entrée
Linux mince est permise si nécessaire, pas un second framework ni une API admin.

## Exécution et réseau

Runner Linux hors cluster en V1 ; endpoints de services Linux effectivement
déployés et cible ancrée avant appels. Pas de compilation/tests Rust via Docker.
Pour la parité OAuth locale : navigateur `http://127.0.0.1:18080`, issuer
`http://127.0.0.1:8080/iam`, callbacks18080 exacts et IdP sur origine enregistrée.
Ces loopbacks doivent désigner les bons endpoints depuis runner ET Chromium.
Sinon STOP de préparation ; pas de changement implicite des identités/allowlists.
Pas de port public nouveau ni d'accès aux contextes protégés. Kind n'est pas
l'environnement des IT Rust ; les E2E consomment un laboratoire déployé séparément.

k6 v2.3.0 est la version stable candidate observée. Mission1 épingle sa version,
sa provenance/checksum et la version exacte Chromium effectivement testée ; aucun
tag `latest`/`master` ou module distant flottant. Pas de dépendance Node supposée
(notamment `Buffer`, filesystem/process et modules npm Playwright).

## Verdict fonctionnel, pas charge

Une VU, une itération explicite par scénario, exécution séquentielle quand les
données sont partagées. Pas de seuil de performance. Délais bornés servent à
éviter un run infini et à vérifier le contrat métier, pas à inventer un SLO.

Succès = processus k6 réussi ET tous les checks obligatoires réussis ET cardinalité
exacte des cas démarrés/achevés/attendus ET observations/restauration/cleanup
obligatoires présents. `checks: ['rate==1']` seul ne prouve pas la complétude.
Setup échoué, cas absent, skip, abort ou preuve manquante interdisent PASS global.
Les refus403 attendus sont des résultats métier ; ne pas imposer aveuglément
`http_req_failed==0` qui les classerait comme pannes. Adapter explicitement les
statuts attendus et vérifier leur refus discriminant avec témoins positifs.
Tests du dispositif : check volontairement faux, scénario manquant et échec précoce
doivent produire un verdict non réussi. Contrôles locaux sans nouveau test serveur.

## Cookies, secrets et sorties

Contexte Chromium neuf et fermé en finally. Aucun transfert implicite de cookie
jar entre `k6/http` et `k6/browser`. Bearer seulement sur les routes autorisées,
jamais transmis à l'IdP ni aux callbacks publics. Corréler le même run ; ne pas
réutiliser des tokens historiques ou déclarer valide une seule structure JWT.

Pas de JWT, cookie, code/state/verifier, clé ou credentials en console, exceptions,
tags/métriques, rapports, traces, captures ou noms de fichiers. Les URL de callback
contiennent des secrets : désactiver/normaliser leurs tags et sorties avant capture.
Pas de `--http-debug`, debugCDP, HAR, captures écran ou métriques brutes par défaut.
Nommer les requêtes avec labels stables sans query. Vérifier par sentinelles que
stdout/stderr et chaque sortie choisie ne publient pas les secrets. Résultats et
manifestes d'artefacts allowlistés et bornés, avec versions/SHA/pin/cible ; absence
de fuite observée sur ce périmètre, jamais promesse d'un filtre universel.

## Cycle de vie

`setup`/`teardown` k6 ne constituent pas un cleanup garanti après échec ou interruption.
Le parent assure inventaire avant/après, baux, registre contemporain des ressources
créées, restauration et stop gracieux des seuls IDs possédés même sur échec.
Aucun prune/reset/kill forcé/adoption par nom ni nettoyage de données étrangères.
Les actions perturbatrices sont explicites, bornées et séparées du code k6.
Une pause opérateur/refus natif n'est jamais levé par le lanceur.

## Conservation des preuves et dettes

Mission2 construit la table de traçabilité ancien oracle → k6, parent ops ou
UT/IT conservée. Les17IT0412 et preuves principal/remote restent à leur niveau.
Aucune suppression des anciens pilotes avant parité démontrée. Ne pas réexécuter
toutes les suites par principe ; une équivalence de sources doit être documentée.

Mission3 vérifie propagation/reprise/duplication/replay de droits réels. Même sub
sous deux issuers reconnus, témoins légitimes pour chacun ; aucun fallback UUID,
ou grant historique ressuscité. Le délai de projection vient de son contrat :
**les60s de fraîcheur JWT ne sont pas un délai de propagation OpenFGA**.
Tout défaut produit reste ROUGE ; correction séparée et revue dans son contrat.

Qualification HSM réelle : dette reconnue/acceptée non bloquante pour ces trois
campagnes, à tracer avec déclencheur et critères de fermeture. ADR0309 demeure
Partial tant que son critère explicite n'a pas été amendé/ratifié ou satisfait.
Ne pas modifier un statut pour transformer la dette en preuve matérielle acquise.

### Dette reconnue : qualification d'un signataire matériel réel

Décision utilisateur2026-10-29 : différée et acceptée, non bloquante pour les
trois campagnes k6. Responsabilité : périmètre IAM/signing, arbitrage architecte
et propriétaire du déploiement lors du choix d'équipement. Déclencheur : sélection
du fournisseur matériel ou projet exigeant explicitement cette capacité.
Fermeture : vrai Sign/GetPublicKey, preuve de possession et binding versionné,
rotation/retrait de confiance et refus sans fallback, sur l'équipement retenu,
avec erreurs et cleanup vérifiés. Aucun vendor choisi, délai/coût inventé ou
capacité HSM annoncée disponible. Cette dette ne change pas le critère ni la
RéalitéPartial actuels de0309 ; un changement de critère nécessite son propre
amendement architecte puis ratification.
