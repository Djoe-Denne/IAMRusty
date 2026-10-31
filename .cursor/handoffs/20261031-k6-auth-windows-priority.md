# Mission k6 authentification — priorité Windows

## Objectif et contraintes

Demande utilisateur : modifier l'ordre de mission2 pour concentrer le portage
sur Windows et reporter le rattrapage Linux. Donner un avis sur un skill d'usage
du pipeline existant, sans en créer un sur la seule demande d'avis.

Travail documentaire direct ; aucun lancement de mission2, nouveau worker,
scénario, runtime ou test fonctionnel. Préserver les oracles manuels, les limites
du socle Windows et les contrats Accepted dans leur portée actuelle.

## Décisions et changements

- `docs/missions/20261029-k6-02-portage-authentification.md` : Windows natif pour
  k6/Chrome/lanceur/parent, première livraison de parité Windows ; Linux ultérieur
  non bloquant. La cible des services doit rester autorisée et joignable, sans
  déploiement/tunnel implicite. Le socle minimal PASS ne qualifie pas HTTP/OAuth.
- La portée Windows mission2 est à contractualiser par architecte avant code ;
  aucune extension silencieuse du contrat Windows mission1 ou ratification par
  cet edit. Les IT Rust restent Windows natif, un cargo/24jobs/E:.
- Le document de recherche/enchaînement porte la priorité actuelle en tête et
  distingue l'objectif Linux historique. Mission3 Linux reste différée, pas lancée.
- Aucun skill k6 dédié retrouvé par matching exact dans `.agents/skills/*/SKILL.md`.
  Le guide `ops/tests/k6/WINDOWS.md` décrit l'entrée et les limites existantes :
  HTTP validate/run/selftest restent fermés sans cible admise. Pas de promesse
  d'un pipeline OAuth déjà implémenté.

## Avis sur le skill

Oui, un skill projet court `aiforall-k6-functional` apporterait une procédure
actionnable, tandis que wiki/QMD servent à retrouver les connaissances. Il
pointerait vers le guide et les contrats plutôt que les dupliquer : choix de
l'entrée supportée, pins/variables, répertoire neuf, exécution native, oracle
strict, diagnostics privés, nettoyage possédé et ajout de scénarios sans perdre
les checks. Windows seulement qualifié à ce jour ; HTTP/OAuth à étendre dans
mission2, Linux non qualifié et différé.

Pas de scripts supplémentaires, nouveaux gardes ACL/verrous/sandbox, framework
ou catalogue de fonctions ajouté. Aucune création de skill avant demande explicite.
L'ingestion précédente a actualisé le wiki mais pas rafraîchi l'index QMD ; ne
pas présenter ses résultats indexés anciens comme la photographie actuelle.

## Vérification / suite

- Deux documents contrôlés : quinze cibles de liens locaux existantes, priorité
  Windows et rattrapage Linux différé explicites, ancienne obligation d'exécution
  `services/k6/Chromium Linux pour E2E` retirée de mission2. `git diff --check` : exit0.
- Premier extracteur de liens mal échappé dans le code imbriqué : contrôle corrigé
  par découpage littéral avant le résultat valide ; pas de panne MCP ni défaut des docs.
- Publication sur master des deux documents et de ce handoff seulement, en excluant
  les changements étrangers. Reçu final : `.tmp/k6-auth-windows-priority-20261031-publication.json`.
- Aucun test k6/Chrome/HTTP, cargo test ou runtime lancé pour cet edit documentaire.
  Pas de création de skill ni de réingestion/réindexation QMD implicite.
