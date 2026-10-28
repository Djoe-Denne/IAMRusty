# Diagnostic de reprise autorisé, avec expertise et contexte adaptés

## Décision humaine et but

Après le rapport109, l'utilisateur demande de lancer le diagnostic, de choisir explicitement le type d'agent et son contexte initial, avec un niveau de raisonnement assez élevé. Cela autorise le diagnostic passif unique de la frontière arrêt/reconstruction décrit dans la question : mêmes production, assertions et délais ; pas de correction automatique ou publication.

## Choix du lead

Un **hard-implementer en contexte fresh**, d'abord strictement lecture seule, examine le croisement entre annulation asynchrone, prise en charge SQL et reconstruction du dispatcher. Ce profil convient mieux qu'un agent mécanique ou documentaire à cette frontière. Le contexte neuf évite d'hériter de conclusions accumulées ; seules les sources exactes, traces et décisions utiles sont transmises, avec le verrou30s explicitement marqué comme hypothèse, pas comme cause imposée.

Demande de raisonnement **élevé**, à sélectionner et confirmer dans l'enveloppe native de la politiquev2 déjà approuvée. Le catalogue contient des efforts high/xhigh ; aucun modèle explicite n'est imposé et aucun score n'est inventé pour contourner le routage. Si l'agent ne peut pas appliquer un effort élevé dans son enveloppe, il doit le signaler avant travail substantiel ; pas de substitution silencieuse ou modification de politique.

L'expert ne coordonne pas d'autres agents et n'édite aucune production. Il rend un diagnostic sur les preuves disponibles et un plan minimal d'observation discriminant. Le test-engineer existant reprendra ensuite l'instrumentation test-only selon ce plan, sans répétition de l'enquête. Ces étapes sont séquentielles car l'instrumentation dépend du point de mesure réellement utile ; la documentation peut évoluer indépendamment.

## Contrat et limites

- AI1f2 + correction testSHA6ac7be0e, SDKd60 propre/index129 inchangé. Échec109 réel après reconstruction, attente PUBLISHED25s.97/105 et les preuves antérieures restent inchangés.
- Inspection RO des logs109 et des sources précises ; hypothèses alternatives exigées. Aucun nouveau runtime, Cargo, nouvelle dépendance/harness/feature ou test pendant l'enquête.
- Plan d'observations SQL readonly autour de l'arrêt/reconstruction et de l'attente publiée, dans les futures/délais existants. Champs sûrs : identité, état, tentatives, next_at/locked_by/locked_until/updated_at, présence d'erreur et heures de lecture ; pas payload/JWT/erreur brute.
- Après plan intégré : instrumentation par auteur tests, compilation sans exécution, inspection du diff et hash par lead, préflight/bail frais, puis **un seul run diagnostique**. Aucun allongement de délai, changement d'assertion/backoff/sampler/production ou retry jusqu'au vert.
- Un arrêt/reconstruction légitime peut modifier la ligne ; ne pas introduire de snapshot gelé ni healer SQL. Un diagnostic absent reste inconclusif, pas un PASS ou un défaut produit inventé.
- Ressources109 déjà nettoyées :0possédé actif, SMTP Exited2/volumes ASK conservés,11préexistants états préservés. Nouvel inventaire requis avant le seul nouveau run ; pas adoption ni arrêt global Docker/WSL.

Aucune nouvelle tentative technique sur ce blocage n'a encore été exécutée à cette étape. La correction temporelle a apporté un progrès vérifié ; les échecs historiques ne sont pas deux corrections infructueuses du même défaut de reprise. Pas d'escalade emergency automatique justifiée actuellement.

## Admission effective

Enquête admise : `task_mvqg5whz_000i_4cy85z`, pool16/hard-implementer, contextefresh, politique existante/catalogue sans override de modèle ou score. Treize entrées copiées et empreintes vérifiées sous **TI/.tmp/recovery-diagnostic110-inputs/** avant publication : autorisation/faits109/log/résumé/cleanup, source test exécutée, fixture, runtime/setup, dispatcher et deux contrats, politique ContextMode locale. Chemins TI fournis explicitement pour respecter le sandbox réellement autorisé. La demande d'effort élevé est transmise ; sa confirmation native reste attendue et n'est pas annoncée comme acquise.

En parallèle uniquement, la même auteure documentaire reçoit109/110 et actualise une proposition distincte, sans toucher les documents staged ni annoncer une validation. Aucun Cargo, instrumentation ou nouveau run libéré par cette admission de recherche.
