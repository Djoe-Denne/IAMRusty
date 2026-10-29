# Une substitution de chemin doit être empêchée avant exécution

## Objectif et problème concret

La mission doit vérifier les outils avant de les exécuter. La revue du correctif de chemins a identifié un défaut HIGH : sous un ancêtre modifiable par un autre utilisateur local, celui-ci peut remplacer le nom du répertoire après le calcul de l'empreinte. Le lancement pourrait alors ouvrir un autre binaire que celui vérifié.

Cela ne signifie pas qu'une attaque s'est produite dans notre environnement : aucune exploitation n'est affirmée. Mais la garde doit interdire cette disposition de chemins. Les40tests verts ne couvrent pas cette course ; ils restent une preuve fonctionnelle partielle, pas un accord de sécurité.

## Décision dans le mandat

L'exécution des binaires et la publication sont bloquées jusqu'au correctif et à sa revue indépendante. Le finding complet est enregistré avant reprise de l'auteur. Le même auteur corrigera la protection de la chaîne de noms et ses régressions ; aucun changement de cible, autorité, TLS/sandbox ou pipeline QA ajouté.

Il faut distinguer un ancêtre public non inscriptible, un répertoire temporaire sticky correctement protégé et un ancêtre modifiable par un autre UID. Ne pas rétablir la règle excessive qui rejetait tous les ancêtres publics, ni prétendre que le seul mode0700 de l'enfant protège son nom.

## Preuves et limites

- Revue indépendante session ses_e767aa95effe5s3xLdh1pnBgQs, candidat result_mvu2j7hb_001o_58pild, HEADdb352be.
- Garde safety.py:79–94, consommateurs pinned_tools/version_checks run-linux.py:123,130–149.
- Rapport complet .cursor/review-briefings/20261029T2140Z-security-k6-private-directory-db352be.md, S-1 HIGH/CWE-367.
- Aucun PoC multi-UID ou payload hostile exécuté. Validation demandée via fixtures et substitution déterministe sans exécution hostile.
- Archives/outils ont été acquis et vérifiés, mais aucun k6/Chrome exécuté, aucun push/dispatch. Source et caches privés conservés sous le bail parent ; seul le diagnostic, les tests et la correction bornée continuent.
- Aucun container Docker créé/redémarré, aucun environnement tiers arrêté. La qualification HTTP reste également bloquée indépendamment de ce finding.
