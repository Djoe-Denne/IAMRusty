# Auteur conservé sans élargir les permissions

## Situation et décision

Le conducteur privé reste à implémenter. L'auteur a lu le contrat ratifié et
confirmé que les 22 fichiers du consommateur sont identiques à la source gelée.
Il n'a lancé ni code nouveau, test, runtime ou collecte.

Son profil lui interdit les outils d'équipe, donc il ne peut pas effectuer le
changement de spécialité demandé. L'outil du lead ne vise que la session appelante.
Il n'est pas nécessaire de modifier les permissions, créer un autre auteur ou
changer le modèle pour poursuivre ce travail déjà autorisé.

Le lead conserve donc **la même session, le même arbre et le même modèle OpenAI**.
L'auteur applique comme complément les obligations écrites du profil spécialisé :
raisonner à partir du contrat et des tests, vérifier ses hypothèses, signaler une
conception incompatible et ne pas élargir le périmètre. Son profil natif demeure
implementer ; aucun handoff réussi n'est prétendu.

## Suite et limites

Le brief est corrigé avant reprise. L'auteur démarre directement les tests de
régression et l'implémentation du conducteur. Aucun nouveau plan ou GO utilisateur
n'est requis. Le contrat, les tests attendus et les conditions avant capture ne
changent pas. **Aucune collecte ni relance Chromium** n'est admise.

## Preuves techniques

- Retour auteur `result_mvv0pt4r_0002_ww2z41`, session conservée, HEAD physique b316
  et index vide ; trois fichiers du conducteur encore absents.
- Contrat/brief précédents intégrés dans `b6d32b3` ; source du consommateur cbab.
- Instructions spécialisées lues dans `.cursor/agents/hard-implementer.md`.
  La sélection de modèle frontmatter n'est pas appliquée : le catalogue OpenAI
  approuvé reste seul applicable.
- Outil team_handoff exposé : session appelante uniquement, confirmé au catalogue
  et au code installé. Pas de tentative de contournement par API, écriture de
  base, changement de permissions ou modèle. CLI opencode non présente dans PATH ;
  aucun client ou service installé/démarré pour résoudre ce simple routage.
