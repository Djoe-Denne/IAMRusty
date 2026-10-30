# Invocation Windows terminée — continuité du handle et provenance des faits

- Statut : **Proposed** ; Réalité : **Unimplemented pour le conducteur durable**.
- Date : 2026-10-30 ; mission1 k6 ; base principale et OPS gelée
  `f279e713079159766188fcbe5078d37217ca530d`.
- Entrées : briefing `20261030-k6-windows-ended-handle-observation.md`, rapport70,
  ledger, [contrat causal](k6-chromium-causal-observation.md) et
  [sondes séparées](k6-chromium-linux-probes.md). Aucun de ces canons Accepted ou
  candidat stockage/sondes n'est modifié par ce document.

## Constat et décision proposée

Le fait réel fourni par le lead porte sur un enfant Python Windows connu, créé
et joint sans WSL : identité native initiale observée, composite après exit refusé,
même handle/PID/création conservés et fin .NET observée. Le refus ne distingue pas
GetProcessTimes, QueryFullProcessImageName ou OpenProcessToken : aucune primitive
précise n'est déclarée fautive. Ce n'est ni un résultat Linux ni une cause Chromium.

Dans `local-private-parent.ps1:82–87`, ces appels composent une seule observation.
`Get-ProcessObservation:295–304` est rappelé après `WaitForExit` par
`Complete-OwnedInvocation:386–410`. Son refus rend aujourd'hui `Clean=false` ;
le caller:541–544 exige Joined **et** Clean avant sa fermeture positive. Dépendance
de disponibilité à corriger, pas contournement démontré ni réouverture de S-1/S-2.
Le journal peut toutefois recevoir le fait initial conservé après ce refus : ne
pas le présenter comme une observation complète fraîche de fin.

**Séparer l'observation pendant la vie et la preuve de fin**, sans exiger de relire
le token/image après exit. Le même handle possédé lie les faits initiaux au
processus terminé ; il ne prouve pas une nouvelle lecture SID ni l'immuabilité
du token/image pendant toute sa vie. Pas de fallback vers des faits anciens pour
une vérification qui exige encore un processus vivant.

Le lead référence et ratifie cette précision avant correction durable. Elle ne
bloque pas les sondes temporaires distinctes sous leurs propres gardes/baux, et
ne ratifie pas rétroactivement une opération ou un nettoyage incomplet.

## Contrat minimal du correctif futur

1. **Avant/pendant :** conserver l'objet Process et son handle exact acquis pour
   l'invocation créée, non clos, sans réouverture par nom/PID ni adoption. Observer
   effectivement PID/création/image/SID, comparer au principal et à l'image autorisés
   avant tout usage du protocole. Observation initiale absente ou enfant déjà fini
   avant celle-ci => refus, pas identité reconstituée depuis une configuration.
   Les exigences des contrôles vivants AUTHORIZE/WRITE/READY/POLL, leur fraîcheur
   et les gardes de stockage/bail restent intactes et refusent toute indisponibilité.
   L'état vivant est attesté par l'attente non signalée, pas par lpExitTime égal à
   zéro : cette dernière valeur est indéfinie avant la fin selon l'API native.
2. **Fin seulement :** après attente attestée de terminaison sur ce même handle,
   interroger séparément PID, création et fin avec les primitives de terminaison
   disponibles, sans appel obligatoire au token/image. Recommandation minimale :
   isoler GetProcessId/GetProcessTimes/attente du composite natif existant ; conserver
   les contrôles vivants plutôt que les affaiblir. Comparer la création dans une
   même unité exacte (FILETIME UTC si natif), pas des chaînes locales arrondies.
   Dispose/fermeture ne survient qu'après lectures et enregistrement nécessaires ;
   une simple égalité numérique de handles après fermeture ne prouve rien.
3. **Observé versus relié :** distinguer au minimum attente/fin observée, continuité
   handle/PID/création prouvée et SID/image observés **avant**. Pour cette voie :
   `sid_observed_before=true`, `sid_reobserved_after=false` et provenance analogue
   pour l'image ; ne jamais recopier Before dans un objet présenté comme Now frais.
   Si un getter .NET réutilise une donnée disponible/cachée, la décrire comme telle,
   pas comme une seconde lecture native indépendante. La lecture StartTime après
   exit ne prouve ni une nouvelle lecture token ni sa fraîcheur.
4. **Verdicts indépendants :** une attente réussie peut attester Joined même si
   le reste échoue. Clean exige toujours protocole/flux valides **et** preuve de
   continuité/fin complète ; aucun rattrapage d'un protocole déjà refusé. Code de
   sortie natif = valeur réellement obtenue après fin, ou inconnu/null ; le code2
   d'échec du parent n'est pas un exit natif observé par défaut. Une valeur d'exit
   seule n'est pas une preuve de terminaison. Identité/temps illisible, exception,
   handle fermé/substitué ou création différente => aucune fermeture positive.
5. **Portée :** préciser ces provenances dans les faits/journaux privés bornés,
   sans publier SID/image/chemins ou exception native brute. L'étiquette `joined`
   ne devient pas une attestation de tous les contrôles. Adapter explicitement
   l'enveloppe/version si ses consommateurs l'exigent, après vérification ciblée
   des usages ; pas de changement des messages Linux AUTHORIZE/WRITE ni d'autorité.
   La fin de wsl.exe ne prouve jamais retrait Linux, lecture utilisateur ou absence
   de descendants : conserver leurs oracles séparés, y compris RETENTION_UNPROVEN.

La voie durable ainsi séparée n'a pas été éprouvée sur cet hôte. Le succès de la
lecture .NET du parent temporaire ne devient pas un PASS des appels natifs futurs.
Aucun droit/principal supplémentaire, privilège debug, Stop/kill, nouvel observateur
global ou élargissement de la confiance n'est proposé. I: reste inchangé ; la
collecte privée et son préflight restent interdits dans cet état de stockage.

## Preuves attendues, sans exécution dans cet avis

- Fixture Windows native connue sans WSL : disposer d'une observation vivante
  réelle, laisser l'enfant finir normalement, conserver le handle jusqu'à preuve
  de fin et vérifier les provenances. Pas de Stop/privilège pour réussir le test.
- Négatifs ciblés : token/image indisponibles seulement après exit ne bloquent pas
  une preuve de fin indépendante complète ; indisponibles pendant la vie ou avant
  toute observation initiale => refus. Handle fermé/autre invocation, PID/création
  discordants, fin non observée, exit inconnu et erreur de lecture => non positif.
- Protocole/flux invalides restent refusés malgré une fin native valide ; interruption,
  absence de join et RETENTION_UNPROVEN ne deviennent pas verts. Sentinelles/erreurs
  restent confinées, et les faits avant/après ne sont pas fusionnés silencieusement.
- Les tests existants `test_native_handle_observer_without_wsl_and_nonjoin_oracle`
  observent le processus courant vivant ; `test_real_complete_and_unjoined_handle_paths_on_process_fixtures`
  remplace Get-ProcessObservation par un fait de fixture. Ils ne prouvent donc pas
  l'observation native d'un enfant terminé, sans que leur échec soit allégué ici.

Impact borné : observateur C# embarqué, Get-ProcessObservation/Assert-Process,
Complete-OwnedInvocation, faits privés et tests associés de
`ops/tests/k6/local-private-parent.ps1` / `tests/test_local_private_parent.py`.
Pas de code écrit ici, API publique, nouveau service/dépendance ou mécanisme de
concurrence ; pas de nouvelle ADR. Le lead intègre/référence le présent contrat
avant consommation, sans modification des Accepted par l'architecte.

## Sources Microsoft recoupées

- [GetProcessTimes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getprocesstimes) :
  handle avec droits de requête appropriés, résultat contrôlé ; lpExitTime est
  **indéfini tant que le processus n'est pas terminé**, pas une preuve live par zéro.
- [StartTime](https://learn.microsoft.com/en-us/dotnet/api/system.diagnostics.process.starttime?view=netframework-4.8.1) et
  [ExitTime](https://learn.microsoft.com/en-us/dotnet/api/system.diagnostics.process.exittime?view=netframework-4.8.1) :
  accès local lié au handle, StartTime lisible après exit sous Windows si handle
  disponible ; ExitTime peut refuser avant fin/sans association/handle. Pas de
  garantie de relecture du SID ; versions effectives du runtime à conserver.
- [Terminaison](https://learn.microsoft.com/en-us/windows/win32/procthread/terminating-a-process) et
  [WaitForSingleObject](https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject) :
  objet processus signalé à la terminaison, durée de vie du handle et accès requis ;
  ne pas fermer le handle pendant une attente. Ces documents n'identifient pas
  l'appel refusé à l'intérieur du composite observé sur cet hôte.
