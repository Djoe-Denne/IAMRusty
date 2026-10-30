# Proposition d'observation précise intégrée, sans autoriser son utilisation

## Objectif et situation

La mission doit qualifier Chromium Linux sans exposer ses journaux natifs. Le
dernier essai établit un SIGABRT sur Ubuntu 22.04, avant le nettoyage du superviseur,
mais pas la cause. L'architecte a livré une proposition pour identifier un emplacement
source public précis sans exporter de texte natif libre.

## Décision du lead

La proposition est intégrée mais reste **Proposed**, donc non ratifiée. Aucun
emplacement pertinent pour cet arrêt n'est actuellement connu ; un catalogue vide
ou spéculatif ne justifierait ni implémentation ni nouvel essai. La classification
courte des formats continue indépendamment dans le contrat actuel.

Si cette classification ne suffit pas, la proposition identifie l'information
manquante : un emplacement structuré, consultable par un opérateur via un canal
privé réel, avec destinataire et rétention définis. Aucun tel canal n'est établi
ou autorisé ici. Ce constat ne donne pas permission de publier une ligne native,
un chemin inconnu ou une empreinte encodée de son contenu.

## Vérifications et limites

Le nouveau contrat a été récupéré à l'identique du candidat puis sauvegardé.
Le lead a seulement précisé une borne documentaire : les 256 KiB concernent le
budget total des artefacts publiables, pas une enveloppe stdout. Le lien canonique
et la photographie des résultats ont été actualisés sans modifier les garanties
Accepted. `git diff --check` réussit sur les deux arbres ; aucun nouveau test
Linux, runtime, code applicatif ou permission n'a été ajouté.

Les 129 tests Linux et le signal précédents restent valides à leur portée. HTTP,
Chromium et nettoyage du runtime parent restent non qualifiés. La mission n'est
pas clôturée ; aucun nouvel arbitrage utilisateur n'est demandé à ce stade.

## Annexe technique

- Candidat architecte `result_mvuadcmx_001i_q7enq7`, session conservée
  `ses_e75fb3daeffeCew1RlUmBKejVR`, base documentaire `1346c019`.
- Nouveau document : `docs/contracts/k6-chromium-causal-observation.md`.
- Sauvegarde récupérable de la proposition exacte :
  `.tmp/k6-causal-contract-proposed-40e6a17d662d443b96f4e706c49b41c7.md`.
- Ancienne proposition de comparaison du worktree architecte préservée ; aucune
  fusion aveugle du contrat obsolète, aucune ratification implicite de l'exception
  AppArmor et aucune preuve native postérieure au run `40982576736`.
