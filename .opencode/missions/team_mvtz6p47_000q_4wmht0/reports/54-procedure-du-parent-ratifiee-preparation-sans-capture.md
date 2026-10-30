# Procédure du parent ratifiée ; préparation sans capture

## Résultat attendu et décision

Le mécanisme privé est testé, mais il ne suffit pas d'activer une option pour
collecter. Un programme parent doit vérifier réellement l'instance et son stockage,
garder la session ouverte pendant la lecture, puis prouver le retrait du relevé.

La proposition de l'architecte est intégrée et ratifiée pour préparer ce programme
et vérifier sa composition. Elle renouvelle les contrôles Windows avant l'autorisation
et l'écriture, avec une fraîcheur maximale de 30 secondes par contrôle. Le lien avec
le HOME Linux ouvert et la provenance est vérifié dans la même invocation non-root.
Un fichier de métadonnées ou un callback de test ne peut pas remplacer ces observations.

La voie recommandée utilise l'interopérabilité WSL existante — exécuter une requête
Windows en lecture seule depuis Linux — seulement si elle est vérifiée disponible.
Les identités et la fin des auxiliaires Windows doivent aussi être tracées ; le
nettoyage Linux ne les prouve pas à lui seul. Aucun nouveau service ou harness de
qualification n'est admis.

## Lecture et rétention

La capture attend toujours un rendez-vous réel et un terminal local privé prêt.
L'utilisateur lit seul le relevé et confirme seulement `LU <run_id>` dans le
terminal de contrôle local. Le parent n'ouvre pas le contenu du fichier et le
couple inconnu ne traverse jamais le chat ou les outils enregistrés de l'agent.

Sans relevé prêt, aucune lecture ni relance. Après acquittement, ou à l'échéance
convenue sous 60 minutes, le seul relevé possédé est retiré et son absence vérifiée.
Les outils et le disque sont conservés. Aucun effacement physique ou cleanup
garanti après un crash n'est promis.

## Prochaine étape

L'auteur existant précisera d'abord la composition minimale et ses oracles, avant
code : commandes fixes, contrôles réels, processus Windows, terminal de contrôle,
refus et retrait. Cette relecture technique par le lead n'est pas un nouveau GO
utilisateur ; elle évite de transformer une simple composition en nouveau harness.
Les tests et la revue de cette procédure précéderont le rendez-vous de lecture.

## Preuves et limites

- Proposition de 70 lignes récupérée byte-identique, diff-check auteur et lead exit0.
- Sauvegarde : `.tmp/k6-real-parent-operation-proposed-a2ea6803c1464e7f9c1b6c18b7845649.md`.
- Consommateur cbab06e : 165 tests Linux sans skip et finding S-1 fermé ; ces
  preuves ne valident pas encore le programme parent à préparer.
- AIForAll arrêté, aucun nouveau processus ou relevé créé, aucune capture admise.
