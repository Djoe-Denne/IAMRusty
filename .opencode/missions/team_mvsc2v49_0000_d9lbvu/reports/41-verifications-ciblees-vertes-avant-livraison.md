# Vérifications ciblées réussies, préparation de la livraison

Les deux suites d'intégration Manifesto réussissent : **15 tests**, aucun échec,
ignoré ou filtré. Elles utilisent Postgres et OpenFGA réels. Les quatre conteneurs
créés ont été retirés et les 23 ressources préexistantes ont conservé leur état.
Les 24 jobs Cargo ont été conservés : tester un binaire à la fois a permis de
franchir la limite mémoire sans changer ce réglage.

La revue sécurité indépendante ne relève aucun défaut démontré dans Manifesto
ni dans le remplacement équivalent des six routes Envoy. Le contrôle infra du
sélecteur de tests confirme que le nettoyage et la propriété restent inchangés.

Je gèle maintenant ces sources validées pour la livraison. Seule l'image
Manifesto affectée sera reconstruite à partir des artefacts Linux vérifiés ;
Envoy recevra sa configuration corrigée avec son image existante. Les anciens
artefacts et configurations seront conservés, sans réinitialisation de données.

La suite complète Manifesto initialement tentée reste **non exécutée**, faute de
mémoire à la compilation. Les preuves Envoy réel, mauvais certificat et navigateur
restent ouvertes ; ces résultats ciblés ne constituent pas une clôture globale.

## Preuves

- Unitaire configuration : 1 PASS ; vérificateur SDK/crypto/routes : 4 PASS.
- IT droits : 8 PASS ; IT consentements : 7 PASS ; quatre fixtures retirées.
- Check Manifesto + monolithe et formatage : réussis.
- Envoy source : baseline discriminante puis 12 PASS ; revue indépendante bornée.
- Reprise IAM déjà prête ; nouvelle livraison Manifesto et reprise Envoy à faire.
