# Contrôle de fin Windows : refus réel avant sondes

## Faits observés et état

Première opération du parent temporaire sous bail neuf
`2dedd5a876e34e3fa34eb480deb43680` : deux phases Windows seulement, `prepare` et
`retire`, jointes, captures bornées propres, PID/création/SID effectivement
observés avant leur fin. Le contrôle composite `K6Parent.Native.Observe` après
fin n'a pas fourni le résultat attendu : aucune identité finale déclarée valide.
Les réponses structurées des helpers n'ont donc pas été acceptées. **Aucune
phase probes, unshare, ldd, Chrome ou k6 n'a été exécutée.**

Le parent retournait0 au niveau de l'invocation PowerShell malgré son résumé
INCONCLUSIVE ; ce code n'est pas pris pour un PASS. Bail marqué nettoyage en
attente, pas clôturé vert. Lecture WSL sans réveil : AIForAll arrêtée, ARTIST
arrêtée, docker-desktop actif inchangé. L'absence des copies reste à recontrôler.

## Expérience discriminante, sans WSL

Un enfant Windows Python connu, `-I -B -c pass`, est lancé par handle possédé puis
joint. Résultat fermé, exit0 :

- observation native initiale PID/SID valide ;
- observateur natif composite indisponible après exit ;
- même handle retenu, même PID et même création via .NET ;
- heure de fin observée, processus joint, code0 ;
- aucun démarrage WSL.

Cette expérience positive localise une incompatibilité de l'observateur composite
avec un processus terminé sur cet hôte. Elle ne distingue pas quelle primitive
interne (image/token/autre) est refusée et n'est pas une cause Chromium.

## Décision locale pour la seule opération de sondes

Le parent temporaire conserve le handle exact créé ; il observe SID/image avant
exit, puis vérifie PID/création immuable/heure de fin et état terminé par ce même
handle. La méthode a été réellement éprouvée dans l'expérience Windows. Les faits
persistés indiquent explicitement `sid_observed_before=true` et
`sid_reobserved_after=false` ; aucune seconde lecture SID n'est inventée. Pas de
réouverture par nom, PID seul ou adoption d'une ancienne invocation.

L'API de production et le conducteur privé restent inchangés/non admis. Le
contrat ponctuel exige identité et join du handle possédé, pas cette primitive
composite particulière. Aucun principal fiable, droit, politique ou garde de
stockage n'est assoupli. Nouvelle opération sous bail neuf avec inspection
explicite des entrées antérieures, retrait seulement si identités privées
correspondantes, pas de nouvelle sonde identique déjà exécutée.

## Suivi du conducteur durable, à instruire séparément

`local-private-parent.ps1` utilise aussi cet observateur composite dans
`Get-ProcessObservation`/`Complete-OwnedInvocation` après exit. Un défaut de
disponibilité est donc à examiner dans ce parcours avant toute admission réelle.
S-1/S-2 restent fermés sur leurs blobs propres ; cette nouvelle limite ne les
réouvre pas et ne prouve pas un contournement de sécurité. Consultation architecte
documentaire possible en parallèle, sans code, WSL, collecte ou ACL. Les sondes
temporaires n'attendent pas une refonte du conducteur qu'elles n'appellent pas.
