# Ordre de mission2 — Porter la campagne authentification vers k6 sans perdre ses oracles

**Préparation seulement. Dépend de mission1 intégrée et vérifiée.** Résultat : une
campagne reproductible k6 sur services Linux, dont les garanties correspondent
à la campagne close, pas simplement une traduction des GET/POST.

## Ancrage et contrat

Lire [mission1](20261029-k6-01-socle-smoke.md), [0504](../adr/0504-validation-fonctionnelle-linux-k6.md)
et [contrat k6](../contracts/k6-functional-linux.md), acceptés avant consommation.
Références factuelles : [état auth qualifié](../contracts/20261029-auth-security-qualified-state.md),
ADR0412 Accepted/Implemented localement, contrats exact-route et autorité8080/18080,
remote-signer-secure-transport et0314. Sources historiques : image/navigateur7b8c1f95,
SDK02e5, validation complémentaire8606173 exécutable identique ; pas pins futurs.
Revalider les sources au start et comparer les risques réellement modifiés.

## Première livraison : traçabilité avant portage

Inventorier chaque oracle de la campagne source et l'affecter explicitement à :
scénario k6 API, k6 Chromium, contrôle parent ops, ou UT/IT conservée. Inclure :
- inscription terminée puis **nouvelle** connexion avec autorité configurée ;
- P1/P2login200/P3START authentifié puis callback relink exact sansBearer ;
- N1link/N2relink-start/N3aliasrelink/callback sansJWT refusés ;
- nonce/cookies/redirections/PKCE, fournisseur effectivement appelé et même run ;
- bon certificat / mauvais nom signé mêmeCA / bon restauré, preuve côtéEnvoy ;
- les oracles principal/remote et preuves conservées dans leur niveau d'origine.

Les17IT de0412 restent des IT Windows natives, pas réécrites en k6. Ne pas
transformer tests de crypto/registry/transactions en E2E artificiels. Toute lacune
de portage est déclarée ; pas « campagne portée » si un oracle obligatoire manque.

## Implémentation attendue

- `ops/tests/k6/scenarios/auth/` et helpers ciblés du socle. Chromium réel,
  contexte vierge, compte de campagne isolé, vrais servicesIAM/Envoy/GitLabConnect
  et IdP synthétique existant. Aucun nouveau frontend. Pas de Buffer Node copié
  tel quel ; adapter aux API k6 épinglées, sans parser un JWT comme preuve de trust.
- Bearer sur START seulement quand prévu ; callback et IdP sanscredential propagé.
  Parcours d'inscription séparé ou préparation explicite : un login d'un compte
  déjà complet ne prouve pas inscription→nouveau login.
- Même run et observations avant/après ; configuration/identité Envoy et conservation
  des requêtes ou observations négatives équivalentes. Les agrégats précédemment
  recevables restent utilisables si leurs préconditions sont maintenues et prouvées.
- Stimulus mauvais certificat et restauration restent au parentops, jamais dans
  k6 via privilège système/APIadmin générique. Réutiliser garde/propriété/ACK selon
  le contrat concerné. Une relecture n'est pas un ACK faisant autorité inventé.
- GitHub réel : réutiliser les preuves existantes applicables. Un smoke externe
  éventuel est opt-in, avec configuration OAuth adaptée, secrets privés et scope
  minimal ; ne pas exiger GitLab réel ou afficher une clé utilisateur. Un token
  d'API seul n'est pas assimilé automatiquement à une application OAuth configurée.

## Acceptation

Tous les cas attendus exécutés et achevés, checks/thresholds fermés et compteurs
non nuls ; témoins positifs et refus discriminants. MauvaisSAN doit accroître le
compteur approprié, pas seulement provoquer503. Réassociation réelle, pas une
URL seulement générée. Aucun retrait de fonctionnalité ou fallback d'autorité.
Parité documentée ancien→nouveau et revue indépendante des oracles modifiés.
Les anciens pilotes restent tant que la parité n'est pas démontrée ; leur retrait
est un delta explicite, pas un nettoyage automatique des scripts historiques.

Pas de performances, HA complète, cloud/HSM, OHA ou suite Manifesto complète.
0412 ne redevient pas Unimplemented en attendant le portage, et n'atteste pas k6
avant cette preuve. Réutiliser les UT/IT encore valides ; si sourceparcours/matchers
change, appliquer les gates0412 plutôt que rejouer toutes les suites par principe.

## Exécution et livraison

Préparation/config/start distincts de l'ancienne mission ; QA proportionate avec
plan de parité et commandes exactes avant run. Windows natif/unCargo24jobs/E:
pour les IT sélectionnées ; services/k6/Chromium Linux pour E2E. Kind uniquement
dans la cible autorisée avec baux après IT requises ; pas nouveau cluster implicite.
Cleanup parent même si setupk6 échoue ; restauration certificat, fermetureChromium,
inventaire final et zéroressourcepossédéeactive sansbail. Pas traces/HAR/rawURL.
Livrables intégrés avant mission3 ; ledger `.cursor/handoffs/20261029-k6-02-auth.md`,
résultats exacts, SHA/versions/cible et limites. Un défaut produit révèle un ROUGE
à remonter : pas correction silencieuse ni assouplissement dans l'auteur de tests.
