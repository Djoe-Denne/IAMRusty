# Authentification : deux causes de démarrage localisées

La mission doit encore démontrer le refus des mauvais certificats par Envoy et
le parcours OAuth dans un vrai navigateur. Ces preuves restent bloquées par les
démarrages d'IAM, de Manifesto et d'Envoy ; aucun succès global n'est déclaré.

Le diagnostic privé a maintenant localisé deux problèmes distincts :

- IAM refuse l'identifiant de clé fourni par les deux configurations du laboratoire.
  Cet identifiant ne respecte pas le format aléatoire normalisé exigé par le registre
  de signature. La correction doit aligner les configurations, sans affaiblir le
  validateur, changer de clé ou réinitialiser le registre.
- Le vérificateur dédié de Manifesto ne fournit pas la politique d'autorité désormais
  obligatoire dans le SDK. L'architecte prépare un contrat limité à cette route ;
  aucune activation de compatibilité globale ni confusion avec l'autorité IAM.

Le diagnostic Envoy est confié au spécialiste réseau, en lecture seule. Les journaux
restent privés : seuls des correspondances avec le code public et des faits bornés
sont restitués. Les migrations ont réussi ; rallonger les attentes ne corrigerait
pas ces refus immédiats.

Je poursuis les corrections bornées et leurs régressions avant toute nouvelle
livraison d'image ou reprise de workload. Aucune décision utilisateur supplémentaire
n'est nécessaire dans le mandat actuel.

## Annexe technique

- Base application : fad3f8d ; SDK02e5 ; images applicatives encore cf02c4a.
- IAM : template setup/app.rs:1267, erreur publique InvalidSigningKeyMaterial,
  configuration key_id test-rs256-kid-01 ; validate_metadata exige UUID v4 simple.
- Manifesto : grant_snapshot_user_id_extractor et require_principal_policy.
- Journaux : captures bornées, identité du Pod vérifiée après capture, aucune sortie brute.
- Envoy réel, navigateur et deux répliques : toujours non qualifiés.

## Contrat ratifié, corrections attribuées

Le contrat Manifesto a été lu puis ratifié explicitement. Il impose une autorité
de service déclarée, sans valeur permissive par défaut, uniquement pour son
vérificateur dédié. Les vérifications cryptographiques et les refus existants
restent actifs ; aucune identité humaine ou permission FGA n'en est déduite.

Les deux corrections ont des auteurs et arbres isolés. Les régressions sont
nommées au plan avant leur exécution. Aucun nouveau build Linux, redémarrage
applicatif ou test navigateur n'a encore été lancé à ce stade.
