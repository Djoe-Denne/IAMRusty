# Choisir un diagnostic réellement discriminant avant de coder

La lecture Envoy fonctionne, mais ne suffit pas à expliquer pourquoi l'entrée
d'authentification ne répond pas. Aucune règle réseau n'a été changée.

Le contrat proposé prévoit une lecture fermée des règles et compteurs existants
sur les deux nœuds concernés. Il explicite une limite importante : même avec ces
compteurs, l'adresse effectivement traduite et l'attribution du rejet au trafic
visé pourraient rester inconnues.

Je n'ai donc pas ratifié ce contrat à ce stade. Avant d'ajouter un analyseur de
règles potentiellement lourd, j'ai demandé à l'architecte d'examiner une lecture
plus petite du suivi des connexions existant, si l'outil est déjà disponible et
si elle permet de relier sans ambiguïté les adresses avant et après traduction.
Sa disponibilité et son caractère discriminant ne sont pas établis.

Ce travail reste documentaire : aucun outil installé, accès kernel ajouté,
commande distante exécutée ou règle d'essai appliquée. Une nouvelle capacité
devra avoir un contrat précis ratifié avant code et rester soumise aux permissions
natives. Si une mesure reste ambiguë, elle ne donnera pas autorité pour corriger
la règle réseau. Aucune décision utilisateur de routine n'est demandée.

## Référence technique

Contrat auteur : local-full-envoy-iptables-evidence.md,61lignes, Proposed,
worktree de l'architecte ; limites source traduite et compteurs partagés explicites.
La preuve navigateur et le contrôle bon/mauvais certificat restent non exécutés.
