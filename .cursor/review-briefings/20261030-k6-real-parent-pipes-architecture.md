# Fixer l'alternative déjà prévue : contrôleur Windows et pipes privés

Même session architecte12, petite précision documentaire uniquement. Canon actuel
du principal : `docs/contracts/k6-chromium-causal-observation.md`, opération du
parent Accepted pour préparation/vérification ; consommateur `cbab06e` gelé,
165 tests Linux sans skip et S-1 fermé, aucune capture réelle. Lire également
`20261030-k6-real-parent-author-plan.md`, plan reçu et disposition du lead.

Le lead a retenu l'alternative **déjà documentée** du contrôleur Windows avec
échanges de métadonnées sur les pipes de sa seule invocation WSL. Le plan auteur
ne démontre pas le lien natif de propriété des auxiliaires interop ; il n'y a eu
aucun essai et aucune preuve que l'interop soit impossible. Ne pas refaire cette
enquête ou demander un GO de choix déjà délégué.

Livrable : delta ciblé au seul contrat causal, en lisant sa version CANONIQUE
actuelle. Remplacer la préférence opérationnelle interop par la composition
sélectionnée : contrôleur Windows propriétaire de son processus WSL, handle natif
retenu et identité/fin constatées ; compagnon Linux non-root compose les APIs
existantes et reste vivant jusqu'au retrait. Aucun auxiliaire Windows interop,
nouveau service/harness, sélectionneur libre d'hôte ou canal réseau.

Préciser les observations réelles et recontrôles Windows/Lxss/SID/ACL/bail, lien
Linux HOME ouvert/ext4/provenance, échanges fermés/bornés en phases/IDs/deadline,
fraîcheur 30 secondes et refus en cas de manque/changement/lenteur. Pipes privés
du processus créé, pas un fichier ou token déclaratif preuve d'autorité. Site et
corps natif ne traversent jamais les pipes ni le contrôleur ; stdin du compagnon
réservé à ce contrôle, pas confondu avec stdin du scénario ou de lecture humaine.

Conserver terminal local réel requis avant collecte, LU opaque, deadline de
retrait, contexte vivant, zéro lecture du site par le parent et cleanup vérifié.
Le préflight des observateurs peut être fait sans armement/session/Chromium,
sous bail séparé. Paramètres d'installation privés hors Git, pas de GUID personnel
érigé en défaut universel. Plan minimal trois fichiers proposé ; détails locaux
à l'auteur après contrat intégré, pas nouvelle boucle de plan.

Pas de code, WSL, exécution, Windows mutation, CI, Cargo, installation, collecte,
ledger partagé ou subagent. Rendre une proposition documentaire gelée avec
diff-check réel, puis le lead intègre/ratifie avant l'auteur. Aucune nouvelle
qualification du conducteur ou admission de collecte à prétendre.
