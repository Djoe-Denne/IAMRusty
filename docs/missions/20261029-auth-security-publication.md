# Authentification — publication des livrables durables

## Périmètre et état publié

La campagne a corrigé et qualifié les frontières d'authentification existantes,
sans qualification production, HA ou HSM. Son résultat canonique est
[l'état qualifié du 29 octobre](../contracts/20261029-auth-security-qualified-state.md).
Les [conclusions des revues finales](../reviews/20261029-auth-security-final-evidence.md)
sont publiées séparément afin de ne pas dépendre uniquement des traces locales.

Les 57 commits applicatifs locaux et le commit wiki/préparation k6 `bf41bd9` ont
été publiés sur `master` de `github.com/Djoe-Denne/IAMRusty`, destination confirmée
par l'utilisateur. Le SDK sélectionné est le commit publié
`02e5db23aac6016aab062c3078696d5f7fd464ef` de
`github.com/Djoe-Denne/rustycog`, branche
`mission/authz-principal-sdk-pool3-20261028` ; pas de fusion distante automatique
de cette branche dans une branche protégée.

## Décisions d'intégration à conserver

- Identité et persistance de l'autorité suivent le
  [contrat principal ratifié](../contracts/authz-trust-domain-boundary.md) et
  [ADR 0314](../adr/0314-principal-authority-through-openfga.md).
- Deux propositions de transport distant avaient été préparées. Le contrat retenu
  est [remote-signer-secure-transport](../contracts/remote-signer-secure-transport.md),
  ratifié après arbitrage : corps Sign limité à 4096 octets, public à 16384,
  délai de 10 secondes incluant l'identité du workload. La proposition alternative
  ADR0315 et sa page wiki n'ont pas été ratifiées : leur absence de master est
  intentionnelle, pas un oubli à réparer par fusion.
- Les instantanés de contrats restés Proposed dans les worktrees ne remplacent pas
  les versions ratifiées de master. Les corrections ultérieures de provenance
  et d'oracles Envoy ne doivent pas être écrasées par une ancienne copie auteur.
- Les trois [missions k6](20261029-k6-recherche-et-enchainement.md) sont préparées,
  non exécutées. ADR0504 reste Proposed/Unimplemented ; la dette matérielle est
  acceptée mais ADR0309 reste Accepted/Partial.

## Contrôle des worktrees conservés

L'audit a examiné les 18 worktrees natifs encore présents de la mission :
changements committés et fichiers non committés. Les chemins des livrables
courants sont présents dans master, hormis les deux documents de l'alternative
ADR0315 écartée. Douze occurrences de fichiers locaux sont identiques après
normalisation CRLF ; d'autres sont des versions antérieures que master a fait
évoluer. Cette comparaison ne prouve pas à elle seule l'équivalence fonctionnelle ;
elle complète les intégrations et revues de mission.

Les 14 arbres isolés locaux sont propres. Quatre arbres SDK ciblés ont un HEAD
ancêtre du pin publié. Deux arbres applicatifs ont un HEAD ancêtre de master ;
huit arbres historiques Telegraph/HTTP ont des historiques séparés après squash,
mais aucun chemin de leur delta n'est absent de master. Ces anciens lots n'ont
pas été requalifiés fonctionnellement par cet audit documentaire.

Aucune fusion globale ni ajout récursif des worktrees n'est effectué. Les autres
anciens worktrees enregistrés dans le dépôt ne sont pas tous couverts par ce lot.

## Ce qui reste local, et pourquoi

Logs et captures brutes, données et certificats générés par les runs, profils
navigateur, baux et inventaires runtime, état natif des sessions, commandes
exploratoires et historique chronologique des transmissions restent locaux.
Les décisions et conclusions retenues sont reprises dans les contrats, les
conclusions de revue et ce document, pas par export intégral du ledger de travail.

Un README ignoré du dossier de certificats IAM décrit encore l'ancien schéma
HS256/RS256 et prétend que des clés sont committées : il n'est pas promu comme
documentation actuelle. Les guides JWT versionnés et contrats ratifiés font
référence, pas cette trace obsolète. Les manifests de `ops/certs/mesh-e2e-run/`
sont des sorties de campagne, pas une nouvelle configuration distribuée.

La règle réutilisable est décrite dans
[Livrables durables et worktrees](../guides/livrables-durables-et-worktrees.md).
Aucun nouveau code, test fonctionnel ou runtime n'est ajouté par cette
réconciliation documentaire ; les résultats antérieurs restent datés et bornés.

## Consolidation complémentaire des branches

Après demande explicite de fusion, les onze résidus non committés ont été
préservés par des commits de livraison sur leurs branches existantes. Une revue
indépendante de contenu a comparé ces snapshots au canon052817d : aucune perte
fonctionnelle bloquante établie ; conserver les versions finales évite de
réintroduire des limites Admin, mappings de ports, oracles PATCH ou pins SDK
antérieurs. Les différences Manifesto sont seulement des fins de ligne, normalisées
selon les attributs Git dans le commit1f4f886, sans changement de contenu normalisé.

La revue a retrouvé deux précisions Proposed absentes du contrat principal. Leur
[conservation non normative](../reviews/20261029-authz-proposals-carried-forward.md)
a été rédigée par architecte puis explicitement ratifiée par le lead **pour leur
conservation seulement**. Aucune nouvelle exigence Accepted ni qualification
implicite de migration, d’invitation ou de tests.

Les instantanés historiques restent distincts des versions à publier. Les
contrôles fmt tentés dans leurs longs chemins Windows ont échoué en environnement
(erreur206 ou exécution indéterminée), jamais présentés comme PASS. Le contrôle
exact `cargo fmt --all -- --check` a réussi depuis la racine canonique courte.
Il qualifie le format de la version conservée, pas celui des anciens snapshots.
Aucune nouvelle campagne fonctionnelle n’est déduite de cette consolidation.
