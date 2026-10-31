# Diagnostic minimal ratifié : vérifier d'abord les outils présents

La mission doit toujours expliquer le trajet bloqué avant de modifier la règle
réseau. La proposition a été réduite : une connexion enregistrée par le noyau
pour établir l'adresse traduite, puis une seule chaîne de refus liée au Pod.
Le parcours de nombreuses chaînes réseau est abandonné.

J'ai lu et ratifié ce contrat précis avant code. Pour éviter un développement
inutile si l'outillage manque, la première tranche vérifie uniquement les outils,
leurs versions et les modules déjà présents. Elle n'interrogera pas encore les
connexions ni les règles de rejet. Tout manque ou refus arrête le diagnostic,
sans installation, chargement de module ou solution de repli automatique.

Cette vérification est en implémentation isolée avec tests simulés, puis revue
avant usage réel. Elle ne donnera ni preuve de traduction, ni droit de modifier
une règle. Les permissions natives restent obligatoires.

Même avec une adresse traduite établie, un compteur de refus partagé peut rester
ambigu : une telle mesure restera non concluante. Aucune règle d'essai ni trace
réseau supplémentaire n'est implicitement autorisée.

## Référence

Contrat architecte v2 : docs/contracts/local-full-envoy-iptables-evidence.md.
Allowlist : garde Python et ses tests existants ; aucune image, application, SDK
ou configuration réseau modifiée. Tests certificat et navigateur toujours ouverts.
