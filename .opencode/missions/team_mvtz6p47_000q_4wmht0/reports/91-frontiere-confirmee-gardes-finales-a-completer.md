# Frontière confirmée, gardes finales à compléter

La vérification indépendante confirme que la protection persistante proposée
respecte le modèle de menace initial. Elle ne rend aucun rôle supplémentaire
fiable et ne prétend pas résister à un opérateur ou administrateur compromis.
Cette clarification peut donc avancer sans relâcher la sécurité demandée.

Il reste cependant des conditions concrètes avant de fermer le dernier point :

- conserver la preuve que les fichiers sont nés dans un espace privé, pas seulement
  observer leurs permissions après une éventuelle exposition ancienne ;
- empêcher une sortie ou un profil de run d'être créé dans le bundle des outils,
  afin que le lanceur ne puisse pas lui-même modifier cet ensemble ;
- conserver fichiers et données tant que la fin des processus manque, y compris
  démarrage incertain, journal absent, interruption du lanceur ou bail expiré ;
- prouver sur des fixtures inertes la persistance des permissions et ces refus.

Le contrat de l'architecte doit intégrer ces conditions avant correction et tests
ciblés. **Les essais réels restent gelés.** Il ne s'agit pas de rouvrir les correctifs
déjà validés ou de créer un autre superviseur ; la revue reste bornée à ce fragment.

## Preuves

[Avis indépendant complet](../../../../.cursor/review-briefings/20261030T2027Z-security-k6-windows-r1-trust-9ef802f.md),
retour result_mvvf7u6v_0000_hvh0gc, lectures seules sans nouveau test/runtime.
Code bd7a2bf inchangé. Les22tests et le préflight statique déjà réussis ne couvrent
pas encore à eux seuls ces dernières conditions. Aucun nouveau k6/Chrome, GET ou
WSL exécuté ; aucune admission ou clôture globale déduite de cet avis.
