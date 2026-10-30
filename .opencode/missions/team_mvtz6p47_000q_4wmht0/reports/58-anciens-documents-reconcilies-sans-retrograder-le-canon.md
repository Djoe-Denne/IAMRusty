# Anciens documents réconciliés sans rétrograder le canon

## Résultat

La comparaison ciblée des cinq documents historiques ne révèle aucun invariant
utile restant à intégrer. Leurs propositions ont déjà été reprises, reformulées
ou ratifiées avec des statuts précis. Les réappliquer en bloc ferait au contraire
régresser le canon vers d'anciens états Proposed ou Unimplemented.

Le lead retient donc leur disposition explicite ci-dessous et conserve leurs
sauvegardes. **Aucun arbre, fichier ou branche n'est supprimé.** Cette étape
prépare la clôture, mais ne remplace ni la publication des livrables durables ni
le retrait et l'archivage natifs à effectuer plus tard.

## Disposition des cinq documents

### Proposition initiale de l'ADR Linux

Le document de l'ancien arbre3 est remplacé par l'ADR0504 Accepted/Partial.
Séparation k6/http, Chromium et parent runtime, périmètre mission1, gardes Linux,
pins, confidentialité et verdicts non vacants sont repris dans ses décisions.
La distinction « handler public ne signifie pas exposition publique par Envoy »
a été ratifiée séparément dans `3f2e8fb` et reste explicite dans l'ADR actuelle.
Ne pas réintroduire les anciens statuts globaux de ce candidat.

### Alternative rédactionnelle du contrat Linux

Le contrat de l'ancien arbre3 est remplacé par la V1 ratifiée. Ses responsabilités,
ancrage Linux, exposition publique, acquisition, cardinalités, tests négatifs,
confidentialité, cleanup et CI sont repris dans le contrat actuel. Le refus
d'activer automatiquement le trajet 18080 est conservé aux lignes154–173.
Aucun invariant opérationnel manquant n'a été identifié.

### Proposition du diagnostic direct parent

Le contrat de l'ancien arbre9 a été consommé par la ratification du diagnostic
parent dans `14d55e9`. L'addendum actuel, lignes385–461, conserve le lancement
direct distinct, les mêmes pins et captures bornées, les métadonnées en lecture
seule, les gardes de confidentialité/identité/cleanup et l'absence de qualification
implicite. Le candidat est une photographie historique, pas un delta à réappliquer.

### Comparaison temporaire Ubuntu22 et voie durable proposée

Le contrat de l'ancien arbre12 a été récupéré puis partiellement ratifié.
Les 123 lignes sont conservées dans le contrat V1, lignes462–587, avec statuts
actualisés : comparaison22 diagnostique acceptée ; voie AppArmor toujours
**Proposed et non autorisée**. Ne restaurer ni le statut global Proposed du
candidat ni une acceptation implicite de la voie durable.

### Contrat du canal privé et de sa composition

Le contrat causal de l'ancien arbre12 a été récupéré intégralement, puis le lead
a actualisé titre et statuts de ratification. Provisionnement WSL, consommateur
privé conditionnel, préparation opérationnelle et composition pipes ont leurs
admissions distinctes. L'ancre publique reste Proposed ; aucune capture n'est
admise par cette disposition. Le dernier amendement +55/-27 a été récupéré
byte-identique avant ratification ; ne pas revenir à la préférence interop.

## Preuves, provenance et limites

Analyse READONLY de l'architecte `result_mvv1ansg_002y_f3181q`, session conservée.
Le lead a recoupé dans les sources actuelles les statuts et passages décisifs :
ADR0504:3–4/78–84, V1:154–173/385–405/462–490, causal:3–8/429–438.
La vérification ne produit aucun nouveau résultat runtime ou test.

Sauvegarde récupérable des six arbres inactifs :
`.tmp/k6-inactive-worktrees-20261030-c268b9bdec094ccbb00b3e0d01e87474`.
Fichiers, index, HEAD, status et patches ont été comparés aux originaux inchangés.
Arbre13 actif exclu. Les 25 autres fichiers des arbres10/4/7 ont été retrouvés
dans l'historique master après seule normalisation des fins de ligne pour la
comparaison des blobs ; ce résultat a été établi séparément par le lead.

Seuls détails bibliographiques non repris littéralement identifiés dans la
proposition initiale : commit k6 rapporté
`e0887846143ab176d4b5483c9d52cf3b3e009f1a`, révision CfT1706539 et timestamp de
catalogue `2026-10-29T12:22:14.500Z`. Ils sont conservés ici comme **provenance
historique non qualifiante**, relue dans le backup, pas comme nouveau pin ou
preuve d'un binaire exécuté. Aucun catalogue flottant n'est admis à l'exécution.

L'auteur du conducteur reste indépendant et actif. Pas de fusion aveugle, reset,
suppression, nouveau test, runtime, permission ou décision Accepted supplémentaire.
