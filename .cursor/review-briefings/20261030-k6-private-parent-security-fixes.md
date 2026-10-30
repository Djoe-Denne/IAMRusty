# Corriger les deux conditions du conducteur — S-1/T-1 et S-2/T-2

## Contrat et état

Même auteur13/session/arbre/profil implementer/modèle OpenAI, avec obligations
spécialisées locales déjà fixées au brief d'implémentation22f7. Lire le rapport
complet `20261030T1428Z-security-k6-private-parent-4560b18.md`, rapport61 et contrat
causal ACTUEL Accepted pipes/collecte conditionnelle. Source intégrée `195059a`,
les trois fichiers du candidat et les 22 sources gelées à comparer avant édition.
Pas de full diff b316, reset/rebase, nouveau worker ou faux handoff.

Rapport persisté avant correction. Deux MEDIUM, aucun CRITICAL/HIGH ; lead exige
leur résolution avant préflight réel/collecte. La capacité adverse sur l'hôte n'a
pas été observée, ne pas inventer un défaut d'environnement ou une compromission.

## Scope et invariants

Écriture uniquement `local-private-parent.py`, `local-private-parent.ps1` et
`tests/test_local_private_parent.py` sous ops/tests/k6. Courte documentation du
comportement seulement si nécessaire. Module privé, runner/Child, classificateur,
pins, scénarios, workflow, services, contrats et ledger partagé inchangés.
Conserver composition, schémas fermés, budgets, contexte et preuves de nettoyage.

### S-1 / T-1 — ancêtres Windows avant Process.Start

Ajouter observations réelles de la chaîne des ancêtres des chemins Windows
sensibles avant lancement, puis contrôle renouvelé : identités stables, reparse
intermédiaires refusés, absence de droit de substitution par principal non fiable,
notamment DELETE_CHILD/DELETE et capacité de réécrire les permissions/propriétaire.
Le profil de comparaison n'est pas preuve seul. Refuser l'indétermination, sans
changer les ACL des parents existants, déplacer l'instance ou demander des droits.

Ne pas exiger naïvement tous les ancêtres privés/du même propriétaire : une chaîne
système protégée peut avoir des lecteurs publics. Définir et tester les droits et
principaux fiables nécessaires ; choix local conforme, pas nouvelle autorité.
Un éventuel refus sur l'hôte restera une preuve de préflight refusé, jamais une
permission de « réparer » les ACL globales ou relâcher le contrôle.

T-1 doit démontrer ROUGE avant fix : feuilles inchangées avec ancêtre substituable,
reparse intermédiaire ou identité d'ancêtre changée ; **zéro Process.Start**.
Positif d'une chaîne sûre, garde renouvelée, données de fixture seulement. Exercer
les fonctions PowerShell réellement livrées et le câblage avant lancement, pas
seulement le comparateur final ou un grep de script. Aucun WSL/principal étranger,
test destructif ou modification d'ancêtres réels nécessaire.

### S-2 / T-2 — retenir l'alerte RETENTION_UNPROVEN

Préserver ce diagnostic fermé après échec de retrait/cleanup du contexte,
jusqu'à la frame et l'avertissement Windows. Réutiliser la constante existante,
sans exception libre, site, chemin ou corps. Ne jamais déclarer une clôture normale
ou qualifier le nettoyage à partir du seul exit non nul attendu.

T-2 ROUGE avant fix : parcours complet synthétique existant, relevé connu prêt,
retrait impossible après ACK puis à échéance. Vérifier état et diagnostic final
RETENTION_UNPROVEN, non-zéro, zéro clôture normale/publication/contenu natif dans
les deux flux ; avertissement Windows réellement câblé. Conserver les parcours
positifs de retrait et les autres refus. Pas de reprise de l'ancien finding S-1
du consommateur et pas de réaudit de sa grammaire FATAL.

## Validation et livraison

Tests purs Windows/PowerShell sur fixtures et temp approuvé, ROUGE puis GREEN,
suite complète189 ou total effectivement étendu/skips précis, parseurs Python/PS,
diff-check. Rendre commandes/totaux/oracles locaux et limites, delta exact contre195,
puis geler. Si un oracle révèle un autre bug de production, remonter la preuve au
lead plutôt que refactoriser hors scope. Deux tentatives substantielles sans
progrès sur un même blocage imposent transfert avec hypothèse/preuve, pas répétition.

Pas de WSL, vrai préflight, k6/Chromium, CI, Cargo/hook, installation, push,
collecte, changement de permissions hors fixtures possédées, ledger ou subagent.
Le lead intégrera, vérifiera les fixtures Linux et fera fermer la revue de façon
bornée avant toute opération réelle. Aucun nouveau GO humain générique requis.
