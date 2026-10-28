# 30 — Cible de l'installation Windows à confirmer

## Objectif et stade
L'utilisateur souhaite une installation durable, plutôt qu'une solution ponctuelle.
Il faut confirmer si « l'installer » désigne Winget ou ripgrep avant de modifier
l'environnement Windows. Aucune installation ou commande de mutation lancée.

## Faits et continuité
Le [rapport29](29-ripgrep-windows-recommandation.md) vérifie que rg est absent du
PATH du sandbox et que `.local/bin` y est déjà présent. L'utilisateur précise
que Winget n'est pas installé. Un alias WindowsApps existant ne prouve pas une
installation fonctionnelle ; aucun diagnostic App Installer n'a encore été fait.

## Options et recommandation
- **Winget** : gestionnaire de paquets pour les besoins récurrents, éventuellement
  utilisé ensuite pour installer ripgrep. Périmètre différent du seul outil rg.
- **ripgrep uniquement** : installation durable de l'outil de recherche, sans
  installer un gestionnaire de paquets.

Recommandation : confirmer la cible, puis vérifier les prérequis et le périmètre
utilisateur/système nécessaires, sans réinitialiser les MCP ou décharger le8B.
Pas de changement global implicite ni d'élévation supposée.

## Décision exacte demandée
« Tu souhaites installer Winget lui-même, ou seulement ripgrep (`rg`) ? »

Inconnue bloquante : le référent du pronom, pas une panne technique. Aucun Cargo,
Docker/Kind/WSL, backend, cache ou fichier applicatif touché.
