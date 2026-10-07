# Revue vivante — périmètre signature IAM après clarification produit

Statut : **Proposed / analyse non contraignante**, pas une ADR canonique. Date : 2026-10-06. Références parent : root `f5c0ba7`, SDK publié `4cdf202`. Lecture statique uniquement ; aucun code, Cargo, test, profile, runtime, cleanup, commit ou statut ADR modifié. Cette analyse remplace les hypothèses « pas de besoin org » / suppression du périmètre tenant ; elle ne change pas les décisions Accepted.

## 1. Ruling produit désormais explicite

Une **autorité de signature plateforme partagée par défaut** pour particuliers et entreprises ; seulement sur demande d'une entreprise, une clé dédiée gérée, ou un signer KMS qu'elle contrôle. Activation rare. « Une clé » signifie ici un domaine/signer commun par défaut, pas l'interdiction des epochs de rotation et de leur courte coexistence.

Ce besoin justifie le port de signature et un opt-in par organisation. Il ne justifie pas, par lui-même, une flotte de KMS cloud livrée dès maintenant, un portail JWKS distinct par tenant, un moteur générique de policies ou un recalcul coûteux à chaque étape de préparation de test. Une décision Accepted est un contrat à respecter, pas une preuve de nécessité commerciale de tous ses détails.

## 2. Ce que fait effectivement la source actuelle

- **Défaut plateforme confirmé.** `services/IAMRusty/infra/src/token/jwt_encoder.rs:603–630` : `generate_access_token(user_id)` émet `iss=self.issuer`, audience configurée et `org=None`. `:196–228` : `require_active_signing_epoch` exige explicitement `TrustScope::Platform`, aucune organisation et une epoch Active. Il n'existe donc pas dans ce chemin normal de sélection automatique « utilisateur de l'entreprise → signer dédié ».
- **Configuration org explicite.** `application/src/usecase/organization_signer.rs:105–178` : configure crée un binding Organization, vérifie le public/issuer et le scope provider, fait le challenge puis le remplacement atomique. `:198–209` : disable révoque les clés org ; ce n'est pas un commutateur d'émission qui réactive automatiquement la plateforme. Le canal admin Hive est fixé par ADR0306 §1–8 et ses routes configure/test/rotate/disable. Pas de génération systématique d'une clé lors de chaque signup dans le chemin d'émission inspecté.
- **Produit encore incomplet.** Configurer/publier une clé org et accepter un JWT de ce trust domain ne prouve pas que login/refresh produit des tokens entreprise dédiés. Le codec normal ci-dessus les exclut volontairement. ADR0305:34–38 documente déjà le gap de switch d'identité / émission org. Ne pas vendre comme livrée cette partie du besoin ; ne pas transformer cela silencieusement en nouveau flux de login.
- **Matrice réellement partielle.** `infra/src/signing/probe.rs:48–61` : challenge organisationnel PEM et OpenBao Transit ; AWS/GCP/Azure et RemoteHttp refusés dans CE chemin. `infra/src/signing/remote.rs::RemoteSigningProvider` existe sur un autre chemin : « aucun remote/provider ne fonctionne » serait faux. `infra/src/signing/transit.rs:85–115,187–230` : create RSA2048 non exportable et sign de digest SHA256/PKCS1v15. Les tests `infra/tests/signing_provider_ports.rs` couvrent des stubs Transit ; ils ne constituent pas à eux seuls une preuve du lab OpenBao réel.
- **Génération et publication à distinguer.** Le bootstrap plateforme `setup/src/app.rs::setup_jwt` charge un matériau/config et le lie au registry ; le helper de tests génère du RSA pour ses listeners. `infra/src/signing/rotate.rs:197–238` génère réellement un RSA2048 en service pour une rotation PEM d'organisation. Pas d'affirmation « tous les utilisateurs déclenchent du keygen ». `application/src/usecase/token.rs:172–193` publie le snapshot writer canonique filtré/validé, même vide : **un JWKS global**, avec des métadonnées/keys org, pas un endpoint JWKS par organisation démontré.
- **Trust ≠ permission.** SDK `rustycog-http/src/jwt_handler.rs:357–448` valide typ/alg/kid/signature/aud, issuer de la clé et owner org, puis recheck la confiance du snapshot. ADR0305 exige le principal `(iss,sub)` ; une clé dédiée n'accorde pas un rôle métier. Cette lecture ne prouve pas une isolation complète de tous les consommateurs/AuthZ : ne pas extrapoler au-delà du contrat et des gates intégrés.

## 3. Verdict sur « réinventer la roue »

**Partiellement oui, mais pas toute la fonctionnalité.** Déléguer création/conservation/signature privées à OpenBao Transit ou un KMS est une opération standard ; le port `domain/src/port/signing.rs::SigningProvider`, les adapters Transit/remote et le composition root sont déjà la bonne couture. La base principale de ce port est ADR0304 §19 / ADR0309. **ADR0104 est Proposed et concerne les overrides cross-hexagone Hive→IAM, pas directement le moteur crypto.**

IAM reste responsable des claims, du mapping key→issuer/owner, de la publication JWKS et du lifecycle compatible avec ses tokens. Transit versionne des clés, mais ne remplace pas automatiquement ce contrat JWT/tenant ni ses tests. Aucun service étudié ne fournit tel quel les limites numériques et l'admission exacte propres au dépôt ; ne pas confondre KMS (Sign/GetPublicKey) et fournisseur d'identité managé (login/token/JWKS).

La stratégie minimale cohérente est donc **plateforme par défaut + org opt-in, peu de backends réellement supportés**, avec délégation de crypto privée quand justifiée. OpenBao2.6.2 est déjà dans le lab ; DEV/volatil et sans HA n'est pas une garantie de sécurité/exploitation production. « Pas de cloud » dans le standard de session borne les expérimentations/déploiements du lab, **pas** le besoin produit futur d'une entreprise de brancher son KMS. Les adapters cloud manquants sont un gap technique, pas une interdiction commerciale.

## 4. Essentiel, hardening légitime, mécanismes révisables

**Essentiel pour l'opt-in annoncé :** binding explicite org/issuer/kid/alg/provider et audience commune ; signature privée non exportée avec KMS externe ; preuve de possession/matching public avant activation ; remplacement/epoch atomique ; prépublication/Pending lorsque nécessaire, Active pour mint, Revoked et courte rétention Retiring pour les JWT existants ; registry/publisher canonique et empty sans résurrection ; fraîcheur monotone60s ; secrets/credentials et lifecycle possédés. Exigences distinctes de SSO/fédération de login.

**Hardening justifié même si activation rare :** refus inter-org, contrôle d'accès admin/s2s, absence de secret dans les events, erreurs redacted, limites finies d'entrée/publication, admission transactionnelle et garde de churn. « Rare en usage prévu » n'empêche pas un admin compromis ou une mauvaise boucle de config de saturer un JWKS partagé. L'idempotence évite de facturer une simple répétition ; les refus ne doivent pas endommager l'Active existante.

**Complexité à réévaluer, sans suppression automatique :** reparse/sérialisation répétée des variants, rescans complets lors du remplissage de test, matrice de providers anticipés/non livrés, plumbing PEM de provisioning/rotation là où Transit peut porter les clés. La comptabilité exacte sous lock est une manière de satisfaire une borne, pas l'unique algorithme possible. Tout remplacement doit garder snapshot cohérent, taille réelle du DTO, réserve plateforme, race inter-org, no-op complet et refus sans effet ; simplifier n'autorise pas un précheck mémoire ni un budget approximatif.

**Chiffres corrigés (ADR0310:22–27) :** plafond global768KiB =786432octets ; réserve plateforme64KiB =65536 ; part globale de toutes les clés org704KiB =720896 ; entrée réservée4096 ; org8 epochs publiables /4 nouveaux kids par3600s ; plateforme16 epochs. **720896 n'est ni720×896 ni un plafond par organisation. Le720 du test est un maximum d'itérations, pas une limite de720 entreprises.** Ces chiffres sont ratifiés et restent à honorer tant qu'aucune alternative n'est ratifiée.

## 5. Une question architecturale prioritaire

**Après opt-in entreprise, indisponibilité du KMS = échec d'émission pour ce domaine (recommandé), ou fallback automatique plateforme autorisé ?**

Recommandation : opt-in exclusif pour le domaine concerné, fail-closed sur outage ; retour au mode plateforme uniquement par désactivation explicite autorisée et transition documentée. C'est une proposition, pas le comportement actuel revendiqué. Le default partagé des entreprises n'ayant pas opt-in reste inchangé. Le choix doit encadrer le futur chemin d'émission entreprise, absent du codec normal inspecté ; ne pas l'inventer dans une correction de performance.

Une clé dédiée réduit le périmètre de falsification **si** les consommateurs appliquent le scope issuer/org et AuthZ adéquats. Elle ne chiffre pas les données utilisateur, n'est pas du login SSO, et n'empêche pas IAM/plateforme de demander une signature que les credentials/ACL du KMS autorisent. Un contrôle exclusif entreprise demande aussi une politique de permission sur ce KMS ; « clé non exportable » ne signifie pas « plateforme incapable de signer ».

## 6. RUSTSEC-2023-0071 — pas de conclusion d'exploitabilité ici

Le security-reviewer possède la vérification de l'advisory/version exacte et de la reachability des opérations concernées. Le raccourci antérieur « toute signature RSA en process est exposée » est **non prouvé et ne doit pas guider une migration**. Cette feuille n'a pas refait cette enquête.

Traitement architectural correct après son verdict : circonscrire le chemin effectivement applicable ; puis correction validée de ce chemin (backend/librairie appropriée ou délégation de l'opération vulnérable), et tests de non-régression de crypto/erreurs/transport. **opt-level=3, seed JWKS, cache du matériau de test ou accélération des tests ne sont pas une remédiation de sécurité.** Pas de patch/version miracle ni de hausse de timeout. Aucun verdict sur OpenBao ou tous les providers par extension.

## 7. Temps de tests : leviers, sans promesse ni retrait de scénarios

Les mesures parent restent35min d'exécution,561 tests et signing_admission410s/4tests, contre4m01 et282 auparavant. Pas de nouvelle mesure par cette feuille. Les messages libtest « >60s » peuvent inclure l'attente du lock `serial_test` : ils n'attribuent pas60s au body/bootstrap. Aucune attribution « setup=100% » ni économie10–14min soutenue sans timers.

Priorités maintenues de `docs/local/20261006-iam-it-runtime-contract.md` :
1. Configuration ciblée num-bigint-dig du workspace consommateur, recommandation upstream versionnée, puis matériau neutre pré-généré/par processus dans les seules fixtures ne testant pas son renouvellement. Ce sont deux gains potentiellement superposés, pas à additionner aveuglément. RSA strength/probes réels/matériaux distincts des scénarios de rotation restent.
2. DTO/accounting et préparation du test de capacité : mesurer/remplacer les répétitions sans réduire frontier, quota, refus ni courses réellement exécutées. Déplacer des coûts hors du chronomètre n'est pas accélérer une exécution complète.
3. Coûts fixtures/migrations/down-up/drain instrumentés avant reset de schéma partagé ; une stratégie de reset doit prouver absence de state leak et conserver de vrais tests de migrations. ADR0200/0201/0202 et ownership demeurent.

nextest est compatible en principe avec llvm-cov, mais son **processus par test** perd les singletons par binaire et les locks serial in-process : pas une accélération gratuite avec Postgres/WireMock3000 et2CPU. Le split CI peut raccourcir le chemin critique en ajoutant des runners, pas réduire le travail ni prouver le budget2CPU. Une répartition unit/IT peut supprimer les exécutions doublées, mais tous les scénarios doivent toujours être exécutés/couverts et les temps globaux publiés. OpenBao/Transit ne rend pas les tests tenant/rotation/admission inutiles.

## 8. ADR / suite autorisée

**Pas de nouvelle ADR ni modification de statut dans cette analyse.** Le besoin clarifié confirme globalement0304/0305/0306 ;0308 et0310 restent les contrats de sécurité. Choisir un backend derrière le port déjà prévu est borné par0304/0309 ; retirer PEM d'un mode supporté, fixer l'exclusivité/outage de l'opt-in, changer une API/limite/lifecycle coûteuse nécessitera le contrat précis et, selon seuil, un draft forward Proposed puis accord humain.0104 n'est pas une Accepted à réécrire silencieusement.

Une future ADR honnête doit consigner besoin particulier/entreprise, activation rare, alternatives OpenBao/KMS, portée réellement livrée et gaps, décisions humaines datées et accrétion de hardening en réponse aux risques observés. Le constat utilisateur « implémentation peu suivie » motive une revue de périmètre et des petits gates de décision ; ce n'est pas une preuve d'absence d'autorisation ou une justification pour réécrire l'histoire/accuser un intervenant. Pas de génération rétroactive de masse.

**Sources de méthode :** INDEX et briefs `20261004-security-iam-infra-final`, `20261004-correctness-iam-core-final`, `20261003-auth-fixes-interface-contract`, `20261005-session-standards` ; ADR0104/0304/0305/0306/0308/0309/0310 ; handbook `docs/platform/authn-jwt.md` ; wiki ciblé (conception/pointeur, pas canon), parfois périmé sur providers. GrepAI et Serena disponibles, aucun fallback infra silencieux.