# Rapport 59 — arrêt ciblé demandé, aucun shell du worker identifié

L'utilisateur autorise explicitement l'arrêt du processus shell bloqué, plutôt qu'un nouveau message que le worker ne pourrait lire pendant son outil. Lead inspecte les processus PowerShell/pwsh avec PID, parent, date de création, commande et enfants, sans arrêt global.

Observation actuelle : hors shell éphémère de diagnostic, seul PowerShell PID2564 existe. Sa commande est le seul exécutable PowerShell, son parent est WindowsTerminal.exe PID11940, et son enfant est ollama.exe PID11912. **Aucun rattachement au worker architecte ou au script0503 n'est établi.** Ce terminal hébergeant un autre workload n'est donc pas arrêté sur simple nom de processus.

Aucun processus tué. Le contrôle précédent n'avait trouvé ni fichiers0503/contrat créés ni modification Git. Le blocage d'outil affiché peut avoir une autre origine, mais permission/transport/état UI ne sont pas démontrés. Ne pas annoncer un kill ou un déblocage réussi. L'autorisation d'arrêt ciblé reste applicable si un PID du shell réellement possédé est ensuite identifié ; elle n'autorise pas l'arrêt du terminal Ollama, du service OpenCode ou d'un worker arbitraire.
