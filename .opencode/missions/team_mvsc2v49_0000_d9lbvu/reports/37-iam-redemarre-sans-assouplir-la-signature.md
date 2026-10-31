# IAM démarre avec la configuration corrigée

La mission vise toujours les preuves de sécurité d'authentification via Envoy et
un vrai navigateur. Un des trois blocages de démarrage est maintenant levé : IAM
a terminé son déploiement et son unique Pod est prêt.

La seule correction livrée au runtime est l'identifiant de clé du laboratoire,
mis au format exigé par le registre existant. La clé, l'émetteur et les politiques
n'ont pas été changés. Le même binaire a été utilisé : pas de reconstruction pour
masquer le défaut, ni réinitialisation du registre ou nouvelle tentative des jobs.

La configuration a été créée séparément, puis le volume du Deployment possédé a
été mis à jour sous contrôle de son identité. Aucun secret vivant n'a été lu,
aucun ancien secret supprimé. Seul le Pod IAM a été renouvelé.

Un contrôle infra indépendant examine ce résultat borné. Manifesto reste en
correction et Envoy en diagnostic. La disponibilité d'IAM ne prouve pas encore
le login, le refus d'un mauvais certificat, le navigateur ou deux répliques.

## Preuves et limites

- Source opérations ef4bc1f, SDK02e5 ; image binaire cf02c4a inchangée.
- Reprise ciblée exit0, étape ready, un Pod prêt.
- Deployment UID inchangé : 6db3ccee-f15e-4c8f-a05d-789a59253b23.
- Pod 6463ed50… remplacé par df614d2c-ced6-497a-97a1-15e6c6f6f53b.
- Configuration créée UID ef3f1b84-d7b1-47b7-9cec-6d96c1e14cbc.
- Preuve privée : session6b7…/iam-kid-recovery-95b120ae2da749238c0ee2a507520be7/summary.json.
- Tests source réutilisés : baseline discriminante puis 26 PASS ; hook canonique normal réussi.

## Cause Envoy désormais confirmée

Envoy refuse au démarrage une expression de correspondance de routes trop
complexe pour sa limite existante. Le diagnostic a confirmé le message public
de son moteur RE2 et l'expression précise du dépôt, sans publier de journal brut.

Le correctif attribué remplace cette seule expression par les six routes exactes
qu'elle représentait déjà, avec la même méthode POST et les mêmes règles
d'autorisation. La limite ne sera pas relevée et aucune route supplémentaire
ne sera ouverte. Les tests et le démarrage réel corrigé restent à faire.
