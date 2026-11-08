# Répertoire des sessions — OAuthConnect / IAM

[Synthèse chronologique](README.md) · [Couverture machine](coverage.json)

Ce répertoire contient la racine et les **36 anciennes feuilles**. Toutes les feuilles historiques ont pour parent `ses_e4db70249ffd7v3AV0y0zQ6443` ; aucun petit-enfant ni fork n'est déclaré dans le corpus lié accessible. Le titre natif décrit souvent la première intervention, pas tous les travaux successifs d'une session réutilisée.

Les ancres explicites `ses_…` sont stables. Les identifiants `msg_…` ci-dessous sont des **métadonnées de provenance**, pas des transcripts ni des URLs de partage. Ils conservent les témoins des 116 citations du digest privé initial, avec déduplication des références répétées. Les messages privés, reasoning et entrées d'outils ne sont pas publiés. Les comptes incluent des messages conservés seulement par leurs métadonnées ; ils ne comptent pas uniquement les réponses publiques.

L'outcome natif observé des 37 captures est `succeeded`, ce qui **ne constitue aucun résultat de test**. La racine est encore susceptible d'évoluer lors de la capture ; cet outcome ne reconstitue pas son état à la frontière. Dates UTC, pagination, hash des projections privées et exclusions : [manifeste](coverage.json).

<a id="ses_e4db70249ffd7v3AV0y0zQ6443"></a>
## Racine — OAuth Connect multi-instance architecture

- **Session :** `ses_e4db70249ffd7v3AV0y0zQ6443` ; parent : aucun ; rôle natif : `orchestrator`.
- **Création :** 6 novembre 2026, 19:14:50.508 UTC.
- **Capture :** 1 983 messages avant frontière ; 2 013 observés, dont 30 ultérieurs exclus ; 82 pages.
- **Contribution :** besoin métier, arbitrages, intégration des candidats, validations rapportées, publication des lots, qualification des incidents et demande de capitalisation documentaire.

### Témoins métier et conception — 6 novembre

- Connecteur configurable et multiplicité future des fournisseurs : `msg_1b248fdb6001uGYPKb16azCKB2`, `msg_1b24e1fcb001wheF3JQCNNOPy5`.
- Évaluation de crates avant développement : `msg_1b254b8b5001FkcK5oh0XvU0Hu`.
- Profils OAuth/OIDC, champs optionnels et noms OIDC contractuels : `msg_1b25f2af3001vuhaHU8m1QS0z0`, `msg_1b269da4f001nczlZwq0IKKwEP`, `msg_1b26b92f6001h8fqYhdhvQaYjH`.
- Proposition API + JSONPath : `msg_1b26d10f6001gJhu1MrZB6CZzS`.
- Compatibilité IAM, email et instance logique versus replicas : `msg_1b260b1bb0019e35S7BtESZBQn`, `msg_1b2664e44001goPlVLH8X2ne1Z`.
- Limitation future on-behalf-of et E2E confiés à une intervention distincte : `msg_1b2751c34001udW9xF24PKyTAt`, `msg_1b2711c5b00187RbD47EZZ4EJp`.
- Déclenchement TDD avec architecte/QA planner : `msg_1b27a3b8d001WdjhYKgXJWJeZk`.

### Témoins de tranches et de schémas — 7 novembre

- Clôture configuration/projection : 114 tests, `3ba3e3c` : `msg_1b59210a3001AYvChBQE0r2uY0`.
- Vrai client composé C, onze tests ; clôture transport/secrets, 189 tests, `eabc645` : `msg_1b601d2110017Cb7hAVAi6Zzlx`, `msg_1b610334d001cH4o7OUa9teiEo`.
- Explication gzip comme choix HTTP, pas exigence OAuth : `msg_1b611421500108geQur6LoFR23`.
- Première demande de schéma, demande d'export documentaire, annonce HTML sauvegardé : `msg_1b6333b2000103rMWBZpcyKk1F`, `msg_1b63a01a4001O34E4BhhqvbJJ9`, `msg_1b6446b23001qIqVhG2bFJMkht`.
- Demande clés/rotation/responsabilités puis explication JWKS/cache et inconnues Ops : `msg_1b6be406b00149ECn5kmv2nk4x`, `msg_1b6df4c82001d38CBJYqj78v6O`.
- État OIDC 58/60 avant correction de fixtures MIME : `msg_1b6b6d236001Rhsr6JNT6MIs2s`.
- Clôture OIDC/service : 295 tests, `e8952d8`, IAM/E2E/cutover encore hors lot : `msg_1b740cd18001FUh3o6cnB2wRdZ`.

### Témoins IAM, QA et limites — 8 novembre

- Déclenchement raccord IAM : `msg_1ba8ea908001vt0fAZkzyiUJec`.
- Résultats unités 30+87 puis 62+146 : `msg_1bae3538f001HCt2pKEtNwJB6H`, `msg_1baf112bb001oijZLmywYqiFku`.
- Upgrade : sortie 8 PASS / 1 échec de montage avant migration : `msg_1bb037cd5001gWmTTpJpQFr1HU`.
- Browser transactions 17/17 et lecture des témoins métier/purge : `msg_1bb058463001MhqMPx1ikC6iwp`, `msg_1bb063ff9001C89PnKonTNy8qM`.
- Incident callback : sorties rouges et conteneur sans ID retourné, `UNKNOWN` : `msg_1bb07b6e5001Q3mUlcg104WjeY`, `msg_1bb0af843001RyI7nd4lEoHo2y`.
- Port sélectionné puis socket libérée ; erreur testcontainers initiale perdue ; précédent ancien ne prouvant pas la cause actuelle : `msg_1bb0f2515001A6Flp1FgQMF2Qn`, `msg_1bb0d4cda001IQ8tC5aPSoTbYZ`.
- Demande de relance sans approfondissement, résultat callback 17/17 sans correction, cause initiale toujours inconnue : `msg_1bb10c6eb001rhGp83sXChJO1A`, `msg_1bb12148b001gdk73cLcPhwIkv`, `msg_1bb1374930012XhbyCWFZgolUP`.
- Onze suites IT finales, total 80 : `msg_1bb144a9e001OuUBJEPwrYwZiC`.
- Rétention de ressources insuffisamment attribuées : `msg_1bb19bf0e001UwgGh3A7ohRTqh`.
- Bilan final `b732b60`, 325 unités + 80 IT, Clippy infra dette préexistante, E2E/cutover hors lot : `msg_1bb1c015d001w1lKLMxICJ1ln2`.
- **Frontière inclusive**, demande d'ingestion et documentation approfondie : `msg_1bb1ecac5001mUOtb937gHb5pE`, 8 novembre 2026 à 12:25:19.929 UTC.

## Feuilles du contrat, de la configuration et du transport

<a id="ses_e4d65cf4dffe50fPpPtQM7F2z4"></a>
### 01 — Socle compilable contrat v2

`ses_e4d65cf4dffe50fPpPtQM7F2z4` · `implementer` · **39 messages**.

Livraison du candidat v2 : deux routes POST, HMAC avant JSON, nonce canonique et PKCE avant I/O, réemploi des réponses génériques et absence de v1 implicite. L'auteur précise qu'il n'exécute pas la validation ; les ajustements du lockfile et les gates restent au parent. Ne pas confondre le nom « compilable » du lot avec un run GREEN effectué par cet auteur.

**Message de provenance :** `msg_1b2abecc8001loPYyN310046Sq`.

<a id="ses_e4d55b055ffebmDOYj90nDXYt4"></a>
### 02 — Revue tests contrat v2

`ses_e4d55b055ffebmDOYj90nDXYt4` · `test-reviewer` · **8 messages**.

Qualification du RED : 13 tests découverts, 5 PASS / 8 FAIL sur assertions de statut, sans échec de compilation ou d'environnement. Les huit rouges correspondent aux comportements v2 attendus. Aucun BLOCKER ; les findings non bloquants portent notamment sur la preuve des erreurs génériques du client.

**Message :** `msg_1b2aeeb13001YvxL2QJAeKJRWh`.

<a id="ses_e4d518c08ffeb6PmFj7FUA8axK"></a>
### 03 — Sécurité frontière OAuth v2

`ses_e4d518c08ffeb6PmFj7FUA8axK` · `security-reviewer` · **14 messages**.

PASS statique borné du candidat : HMAC par instance sur méthode/chemin/corps exact, validations avant I/O et pas de v1 exposé implicitement. Pas de finding nouveau bloquant. Le reviewer distingue explicitement ce PASS d'un GREEN exécuté et indique les limites de l'index canonique face au worktree candidat.

**Message :** `msg_1b2b0fd720010XBsMD64Jcy8gb`.

<a id="ses_e4d5ff69dffeHUsT9L7q3x7cL5"></a>
### 04 — Tests RED du contrat v2

`ses_e4d5ff69dffeHUsT9L7q3x7cL5` · `test-engineer` · **33 messages**.

Écriture puis maintien des témoins du contrat. Dernier delta : quatre corrections Clippy, dont libération des mutex avant assertions ; seize tests et leurs attentes conservés. Aucun changement de production ni résultat d'exécution supplémentaire revendiqué dans ce retour.

**Message :** `msg_1b2bb9343001g9XYBPjQAw75SC`.

<a id="ses_e4abcc502ffe9dqNmMAhcWdDhN"></a>
### 05 — Revue oracles projection pure

`ses_e4abcc502ffe9dqNmMAhcWdDhN` · `test-reviewer` · **11 messages**.

Vérification des 41 cas contre le contrat et la grammaire `jsonpath-rust` pinnée : 18 caractérisations déjà vertes et 23 signaux d'acceptation RED. Les expressions fonctions/descente récursive parsent effectivement : les refus prouvent la politique AST et non une simple erreur de syntaxe. PASS avec LOW/INFO non bloquants.

**Message :** `msg_1b548741c0011bketA84GE3mWm`.

<a id="ses_e4ab10ef8ffeh7Hf5ANNikHwbZ"></a>
### 06 — Sécurité config et projection

`ses_e4ab10ef8ffeh7Hf5ANNikHwbZ` · `security-reviewer` · **29 messages**.

Mitigation ciblée S-1 : estimation/admission avant évaluation exhaustive, cumul global et capacité liée aux entrées immuables. Aucun contournement démontré du correctif. Le PASS reste statique ; unités privées et gate qualité du parent sont encore requis à l'instant du retour.

**Message :** `msg_1b58414d6001XDJO4hzhGruCS8`.

<a id="ses_e4a392995ffeJMgQh2DVrRJNgd"></a>
### 07 — Revue oracles HTTP secrets

`ses_e4a392995ffeJMgQh2DVrRJNgd` · `test-reviewer` · **23 messages**.

Revue des trois cibles transport/secrets/builder, 32+24+9 cas, contre les sources T0 et les contrats : redirects et données exactes avant I/O, POST token et encodage, plafonds, secrets, composition. Aucun Cargo ou test exécuté par ce reviewer ; la présence statique des oracles n'est pas le résultat de leur exécution.

**Message :** `msg_1b5cf35c6001xvuJ8wc4eo9TCA`.

<a id="ses_e4a2be888ffeY3V0HEWgSmFNHD"></a>
### 08 — Sécurité secrets et builder

`ses_e4a2be888ffeY3V0HEWgSmFNHD` · `security-reviewer` · **51 messages**.

Revues successives des secrets et de la composition. Le dernier PASS porte sur C uniquement : validation globale, refus avant acquisition, association stricte des instances, vrais producteurs B→A, HMAC exact et absence de collection partielle. T2 est clos sur la preuve parent du vrai client composé recoupée avec son oracle. Aucun PASS plateforme/OIDC/IAM/E2E n'en découle.

**Message :** `msg_1b603fbb8001GYd8QTN2n4viIa`.

<a id="ses_e497d49e5ffeb7gW8lcp2En4yh"></a>
### 09 — Revue oracles routing HMAC

`ses_e497d49e5ffeb7gW8lcp2En4yh` · `test-reviewer` · **41 messages**.

Rectification T-S1 après comparaison des commits/blobs : l'assertion finale `pair.zero()` était présente dans le commit RED, avant production B. L'artefact untracked initialement relu différait du blob committé ; son ancien diff exact n'est plus reproductible. Le reviewer retire le finding sans inventer une modification tardive d'oracle.

**Message :** `msg_1b69119bb001pODgaCCQsjVRuz`.

## Feuilles OIDC, service, architecture et schémas

<a id="ses_e4970ed13ffeuXzBM9OSwBp2bk"></a>
### 10 — Revue tests OIDC cache

`ses_e4970ed13ffeuXzBM9OSwBp2bk` · `test-reviewer` · **69 messages**.

Revue indépendante des quarante premiers tests A : 28 publics dont deux préflights, douze privés. Vérification des versions et risques de compilation, des claims, du callback et du cache. PASS borné au contrat de tests au stade RED ; preuve qualifiée `static-presence`, l'exécution et la qualification RED restant au parent.

**Message :** `msg_1b69e36de001hiRoiAMIxdik4n`.

<a id="ses_e49cbab1bffeaqRcz0N7HaOmXC"></a>
### 11 — Schéma canvas OAuthConnect

`ses_e49cbab1bffeaqRcz0N7HaOmXC` · `general` · **64 messages**.

Réalisation des schémas architecture puis clés/responsabilités et supports HTML autonomes. Le retour distingue clé privée fournisseur, cache public RAM Connect, secrets HMAC/OAuth et TTL 300 s non assimilable à une rotation. Il rapporte 60 primitives SVG, 39 libellés et dix fiches cohérents entre supports, mais **pas** de rendu React/visuel ou pagination PDF vérifiés ; aucun déploiement réel prouvé.

**Message :** `msg_1b6d68e0e0011PRvMTaea6qr4s`.

<a id="ses_e491b82b6ffeIzEbWs6VJzS0eH"></a>
### 12 — Partager garde paramètres OAuth

`ses_e491b82b6ffeIzEbWs6VJzS0eH` · `mechanical-worker` · **18 messages**.

Extraction d'un prédicat partagé sur les neuf noms réservés existants ; OAuth garde sa politique et OIDC l'applique aux query names décodés de l'endpoint authorize. Pas de nouvelle normalisation, pas d'interdiction de toutes les queries, pas de suppression/écrasement silencieux de paramètres.

**Message :** `msg_1b6e7e369001wYb52UFvbGqtR3`.

<a id="ses_e493d161dffeEhTswdhuH1pWYJ"></a>
### 13 — Revue sécurité OIDC effectif

`ses_e493d161dffeEhTswdhuH1pWYJ` · `security-reviewer` · **36 messages**.

Recoupement du correctif LOW OIDC-S1 : URLs gardées, noms réservés décodés contrôlés avant GET JWKS, prédicat inchangé côté OAuth. Aucune objection statique restante sur ce finding ; la fermeture comportementale attend toujours le GREEN exact du parent au moment du retour.

**Message :** `msg_1b6ec6da2001QNGJyKN7TJq8bk`.

<a id="ses_e491163c9ffeW6qxu3LCGapug2"></a>
### 14 — Revue tests service C

`ses_e491163c9ffeW6qxu3LCGapug2` · `test-reviewer` · **33 messages**.

Revue TESTONLY des quatorze oracles composition/vrai binaire : cinq PASS de préflight/refus légitimes et neuf RED attendus contre T0. Vérifie notamment late-fail et listener/lifecycle, sans exécution. Le décompte attendu est une analyse d'oracles, pas un résultat de campagne.

**Message :** `msg_1b6fad471001ANPZLPcarJ27n7`.

<a id="ses_e4ac5aeb9ffeBsKR3K4mO0tC3L"></a>
### 15 — RED projection JSONPath pure

`ses_e4ac5aeb9ffeBsKR3K4mO0tC3L` · `test-engineer` · **285 messages**.

Session réutilisée pour mapping puis transport/OIDC. Dernier ajout OIDC-S1 : nominal query autorisée conservée, state/nonce uniques et exacts ; neuf paramètres réservés chacun littéral/percent-encodé, refus avant JWKS et compteurs stricts. Les quarante anciens tests restent inchangés ; le nouveau RED est attendu, pas encore exécuté par cet auteur.

**Message :** `msg_1b6d729a2001If50Whz7ep2tPX`.

<a id="ses_e48f887e3ffeV1xGZ388Kkirw7"></a>
### 16 — Débloquer Send avec réemploi

`ses_e48f887e3ffeV1xGZ388Kkirw7` · `emergency-thinker` · **47 messages**.

Résolution de la contrainte lifetime redondante de `DiscoveryHttp`, puis des DN identiques CA/feuille de fixture TLS. Convention IAM réemployée sans relâcher la vérification. Le retour rapporte préflights TLS positif/refus CA-SAN, 61 régressions PASS et C setup encore 3 PASS / 8 RED T0 ; ne pas globaliser ce résultat intermédiaire.

**Message :** `msg_1b712ea47001w2hJFMH5T16Ofx`.

<a id="ses_e4abd5fbbffe0kShdSQtoWBAlP"></a>
### 17 — GREEN projection JSONPath typée

`ses_e4abd5fbbffe0kShdSQtoWBAlP` · `hard-implementer` · **243 messages**.

Interventions successives : mapping/AST et diagnostic de whitespace implicite du parser ; budget S1 ; transport ; OIDC, cache et snapshot JWKS ; diagnostic des deux fixtures UserInfo signé servies en mauvais MIME. Ce dernier diagnostic ne justifie aucun assouplissement de production. La dernière tentative d'adapter Discovery Send est encore non compilée ; elle ne remplace pas le correctif final de la feuille de secours.

**Messages :**
- parser : `msg_1b54c5231001kh1EPNOoKMWl72` ;
- budget : `msg_1b57a95d70014fST2GDXLrxpuk` ;
- candidat OIDC/cache/snapshot : `msg_1b6af1a29001JWRrjrnDnT1Fxb` ;
- cause MIME UserInfo et correction des fixtures : `msg_1b6bcc1df001ujR5ix7tr4keSe` ;
- dernière tentative Send avant secours : `msg_1b7014276001lxtOQgH63Vp1j5`.

<a id="ses_e48d210feffeBIcIVpJu1VLB6w"></a>
### 18 — Revue sécurité startup C

`ses_e48d210feffeBIcIVpJu1VLB6w` · `security-reviewer` · **26 messages**.

PASS sécurité statique borné à C startup et au delta Discovery typed-Send, sans finding établi. Le reviewer recoupe les logs parent de cas ciblés puis C15, mais ne réexécute rien. Le gap §10 reste encore ouvert à cet instant ; aucune qualification globale du service n'est anticipée.

**Message :** `msg_1b7327c1c001wHRXhDOYyXu5r7`.

<a id="ses_e4ad0aa1effegB7v1FG79PA5BZ"></a>
### 19 — Socle configuration et mapping

`ses_e4ad0aa1effegB7v1FG79PA5BZ` · `hard-implementer` · **233 messages**.

Plusieurs socles puis microfix startup C : l'extracteur réellement transmis à AppState vient du helper privé HMAC-only utilisant un JWKS vide. Aucun secret HS256, identité par défaut, URL réseau ou réemploi HMAC comme clé JWT. Le parsing `auth`, les routes et le lifecycle restent inchangés. Dernier delta sans run, après état hérité 12/15 PASS.

**Message :** `msg_1b72bf8d2001w9LIVRN0mpeNIx`.

<a id="ses_e4d83442affebJCndafMmCqDr4"></a>
### 20 — Plan TDD OAuthConnect

`ses_e4d83442affebJCndafMmCqDr4` · `qa-planner` · **117 messages**.

Plan initial puis découpage A OIDC réel/claims/cache, B registry-router/HMAC indépendant, C composition mixte/listener/vrai binaire après A+B. Les coutures de temps/cache, callback, claims UserInfo et arrêt du binaire sont à réconcilier avec les contrats. Préflights et gaps sont explicites ; les fixtures ne prouvent aucun E2E fournisseur.

**Message :** `msg_1b624ba5b001eNi1tn5ooCsItQ`.

<a id="ses_e4d834425ffehYgxAsZJJl5CrU"></a>
### 21 — Contrat architecture OAuthConnect

`ses_e4d834425ffehYgxAsZJJl5CrU` · `architecte` · **206 messages**.

Contrats successifs du service puis amendement §10 de composition HMAC-only. Décision B : remplir le champ SDK obligatoire avec un extracteur à JWKS vide strictement refusant, plutôt qu'imposer une configuration JWT plateforme inutilisée. Aucun nouveau SDK/API/ADR ; ADR0413 reste décrite Accepted/Partial. Les trois échecs startup observés ne deviennent pas GREEN par rédaction du contrat.

**Message :** `msg_1b7284de2001MqStWeoJZNEvCR`.

<a id="ses_e4ac61246ffe3gDG9Fn6pXsVCy"></a>
### 22 — RED configuration OAuthConnect

`ses_e4ac61246ffe3gDG9Fn6pXsVCy` · `test-engineer` · **244 messages**.

Configuration, secrets et service, puis caractérisations C16 du helper HMAC-only réellement utilisé : refus de tokens vides, malformés, HS256/RS256 contrôlés ; HMAC valide reste nécessaire aux routes. Dernier ajout attendu PASS après correctif, **pas nouveau RED fabriqué**. Pas d'audit réseau global déduit des seuls compteurs fournisseurs.

**Message :** `msg_1b73347f8001Hw1zFRSF1niYmX`.

## Feuilles IAM v2

<a id="ses_e456895e9ffef0qzf10IpN3g63"></a>
### 23 — Plan QA IAM proportionné

`ses_e456895e9ffef0qzf10IpN3g63` · `qa-planner` · **28 messages**.

Risques isolés : non-rejeu, commit/double-purge, refus complete sans effets métier et transport HMAC borné. Préflight baseline domaine 53 PASS rapportée par parent ; upgrades legacy et régressions restent à prouver. Les rouges fonctionnels rares doivent être regroupés pour arbitrage, jamais supprimés automatiquement. Aucun run du planner.

**Message :** `msg_1ba9e6402001hAQ384ZTKXYYHa`.

<a id="ses_e455d4b88ffe4jP4krG553x1lT"></a>
### 24 — Préparer types nonce compilables

`ses_e455d4b88ffe4jP4krG553x1lT` · `mechanical-worker` · **16 messages**.

Scaffold des types nonce, modèles et fixtures, Debug expurgé. Les valeurs vides/`None` sont provisoires et commentées, destinées aux futurs témoins RED et au correctif durable ; elles ne décrivent pas le résultat final. L'inventaire des littéraux est vérifié, sans exécution de Cargo par cette feuille.

**Message :** `msg_1baa6d05d0017t7WJVAjYJUgde`.

<a id="ses_e455cc19affeYVyUoVmvxTeYz8"></a>
### 25 — Écrire témoins transport IAM v2

`ses_e455cc19affeYVyUoVmvxTeYz8` · `test-engineer` · **22 messages**.

Onze candidats transport : deux slugs et HMAC PATH/corps exacts, nonce même sans PKCE, refus syntaxe/PKCE avant I/O, erreurs génériques sans fallback, bornes 1 Mio et +1 octet, redirects sans forwarding et timeout réel dix secondes. Les candidats sont relus KEEP mais non exécutés par leur auteur.

**Message :** `msg_1baa78184001wCoTCs9BT57tIS`.

<a id="ses_e45541a57ffe0qBtjsLLZWiF6w"></a>
### 26 — Implémenter transport IAM v2

`ses_e45541a57ffe0qBtjsLLZWiF6w` · `implementer` · **27 messages**.

Implémentation authorize/complete v2 et reader borné partagé v1/v2, comptage réel des chunks, plafond inclusif 1 Mio et erreurs génériques. Connexion3s ajoutée, total10s et redirects désactivés conservés, API reqwest 0.11.27 vérifiée. Aucun oracle gzip/stream sans longueur exécuté par cet auteur ; aucune garantie RSS revendiquée.

**Message :** `msg_1baadfcbd001TXNMOOGgEj1iiF`.

<a id="ses_e454a4f5fffe9IjBHGk43oUPMh"></a>
### 27 — Adapter témoins HTTPS v2

`ses_e454a4f5fffe9IjBHGk43oUPMh` · `test-engineer` · **23 messages**.

Nominal v1 conservé avec zéro receipt v2 ; nouveau nominal v2 nonce/PKCE/HMAC complet et identité groupée ; trois négatifs mauvaise CA, mauvais SAN et redirect vers second listener trusted sans forwarding. PKI, listener et teardown existants réemployés. Candidat sans exécution par auteur.

**Message :** `msg_1bab8c878001KyQaH8X3Um5M7b`.

<a id="ses_e4544a2c3ffeHVpobs1A3pUPFc"></a>
### 28 — Persister et purger nonce SQL

`ses_e4544a2c3ffeHVpobs1A3pUPFc` · `implementer` · **23 messages**.

Writer : INSERT nonce exact, projection des deux secrets et purge atomique, originaux retournés après commit. Migration additive `TEXT NULL`, pas de backfill/default/index, `down` conservateur et `up IF NOT EXISTS`. Verrou, prédicats et horloge restent ; le legacy/corrompu est consommable avant refus domaine. Aucun GREEN SQL revendiqué au handoff.

**Message :** `msg_1babcf934001jqg85q9oGqKuyY`.

<a id="ses_e456930a8ffe5DkAMzIMbSixTP"></a>
### 29 — Cadrer coutures IAM v2

`ses_e456930a8ffe5DkAMzIMbSixTP` · `architecte` · **41 messages**.

Contrat puis choix de façade : cinq réexports de types existants par `iam_domain::port::service`, sans DTO/helper, dépendance application directe ou changement de trait. Compilation des consommateurs avant auteurs suivants : préparation, pas RED. Aucune nouvelle ADR, la couture restant bornée aux décisions existantes.

**Message :** `msg_1bacc41f8001QZWG16bJPW026b`.

<a id="ses_e45560644ffevfOmn1HTkvVYyf"></a>
### 30 — Écrire RED nonce et capacité

`ses_e45560644ffevfOmn1HTkvVYyf` · `test-engineer` · **69 messages**.

Nonce/capacité, LOGIN puis LINK/RELINK. Dernier delta : trois fonctions pour quatorze cas — forwarding de BEGIN, refus complete avant lookup/write, incohérences de contexte refusées sans I/O. Deux providers sont réellement enregistrés et la capacité vient du chemin réel consume. Six témoins visent le comportement v2 absent, huit caractérisent les gardes existantes ; résultat pas encore exécuté par auteur.

**Message :** `msg_1bad3a661001PUG02Fi6T5HAtN`.

<a id="ses_e454d6265ffe5BXwXMyYY9DGIb"></a>
### 31 — Implémenter capacité nonce durable

`ses_e454d6265ffe5BXwXMyYY9DGIb` · `hard-implementer` · **37 messages**.

Trois étapes : nonce durable et API de complétion depuis la capacité ; migration LOGIN ; migration LINK/RELINK et retrait du helper temporaire v1 après vérification des usages. Nonce32 octets indépendant de PKCE, garde après retour writer, transfert exact et unique sans retry/fallback. Règles métier conservées. Chaque handoff est un candidat source non exécuté par l'auteur ; les preuves SQL sont séparées.

**Messages :** domaine `msg_1bab4cdf4001pPlLV3u7aaICu6` ; LOGIN `msg_1bace41b9001JZbxS1UHI7sO6E` ; LINK/RELINK `msg_1bad98b2e001RgptfdYoHRAvZb`.

<a id="ses_e452b6716ffeTAqTunP21eTNX1"></a>
### 32 — Revoir sécurité nonce et transport

`ses_e452b6716ffeTAqTunP21eTNX1` · `security-reviewer` · **32 messages**.

Dernier PASS statique borné au delta LINK/RELINK et retrait du helper. Producteurs/LOGIN/transport antérieurement revus sont réutilisés après comparaison des sources inchangées. Aucun finding S-n nouveau ni vulnérabilité introduite démontrée. Aucun Cargo/runtime exécuté par le reviewer.

**Message :** `msg_1bae9ac43001RdUhoFR60t2l8X`.

<a id="ses_e45181526ffeSzYFWRbgMf3kMM"></a>
### 33 — Revoir comportement IAM v2

`ses_e45181526ffeSzYFWRbgMf3kMM` · `correctness-reviewer` · **75 messages**.

PASS statique : précédences transactionnelles, refus legacy après purge commitée, parcours LOGIN/LINK/RELINK et contrats HTTP conservés. Ce verdict ne ferme aucune preuve PostgreSQL/browser/TLS réelle, G5, fournisseur ou cutover. Les limites de lecture sont explicites.

**Message :** `msg_1baf0746b00147AdbuqYT7pALK`.

<a id="ses_e4517c25cffegyuJf741XREQmk"></a>
### 34 — Auditer preuves IAM v2

`ses_e4517c25cffegyuJf741XREQmk` · `test-reviewer` · **45 messages**.

PASS statique borné aux oracles nouveaux/transposés, avec un MEDIUM et un LOW non bloquants. Les onze IT n'ont pas encore tourné à cet instant. Le finding G7 réclame les témoins email RELINK ignoré et LINK nouvel email non vérifié ; les montages en-répondeur sont alors prouvés structurellement, pas à l'exécution.

**Message :** `msg_1baf2d065001qwCe0z3GG6SfQH`.

<a id="ses_e453b21beffe4azeET8ucSMffQ"></a>
### 35 — Transposer fixtures navigateur IAM

`ses_e453b21beffe4azeET8ucSMffQ` · `test-engineer` · **103 messages**.

Transposition des neuf fixtures puis ajout des deux témoins email G7 et d'une collision RELINK avec identité externe déjà liée ailleurs. Snapshots, tokens exacts, cible conservée, email nouvellement ajouté non vérifié, purge et replay sans nouveau complete sont observables dans les oracles. Aucun nouveau statut métier précis n'est imposé pour la collision.

**Message :** `msg_1bb014155001v0C9iNZfzIwMM7`.

<a id="ses_e4552a8ccffexnxfkjq7biFGoq"></a>
### 36 — Prouver writer et upgrade nonce

`ses_e4552a8ccffexnxfkjq7biFGoq` · `test-engineer` · **64 messages**.

Témoins SQL/migration, puis correction du prérequis de montage PG23502 : UUID `user_emails.id` explicitement fourni. Le résultat antérieur était 8 PASS / 1 échec avant l'upgrade ; aucun RED comportemental de migration ne peut en être déduit. Les oracles de snapshots, métadonnées et contraintes restent inchangés ; le parent doit relancer.

**Message :** `msg_1bb0542f4001emHOHsnnWliqWR`.

## Nouvelles feuilles exclues

Ces sessions sont postérieures à la frontière et ne sont pas des captures historiques supplémentaires :

- `ses_e44da9210ffew9111GSoH2H1JO` — « Documenter JWKS rotation TLS », rôle `general`, créée le 8 novembre à 12:32:34.866 UTC ;
- `ses_e44d9bc2fffeYsPWEPBNGJRByw` — « Inventorier toute la session », rôle `general`, créée à 12:33:29.555 UTC.

Leur parent est la même racine, mais leurs messages ne sont **ni lus ni comptés** pour cet instantané. L'histoire ne se prolonge pas automatiquement avec les nouvelles feuilles de la mission documentaire. Voir les exclusions formelles dans [coverage.json](coverage.json).
