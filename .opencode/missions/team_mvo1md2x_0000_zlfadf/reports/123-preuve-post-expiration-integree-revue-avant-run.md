# Preuve après expiration intégrée, revue avant l'exécution unique

## Étape atteinte

Le même auteur a confirmé le raisonnement natif high puis préparé la preuve122. Formatage et compilation sans exécution réussis ; aucun test ou runtime lancé. Le lead a inspecté le patch complet, intégré exactement le fichier compilé et vérifié son empreinte et le diffcheck. Huit artefacts sont archivés avec copies aux empreintes identiques.

Le test précédent117 reste inchangé. Le nouvel ajout mesure l'éligibilité, fixe une seule deadline depuis l'entrée en reconstruction, inclut factory/SQL/réseau dans ce budget, puis vérifie les sélections avant et après expiration, le nouveau propriétaire, le succès du vrai transport, l'étatpublished, les bytesSQS et les comptes métier. Le wrapper de publisher est uniquement un observateur de test qui délègue au transport réel, pas un hook de production.

La création utilisée ici produit une notification, aucune livraison optionnelle et une intention outbox : le test conserve honnêtement les comptes1/0/1 de ce chemin, sans prétendre prouver le chemin de livraison1/1/1 ou les droits/replay verticaux.

## Revue ciblée et limites

Une revue indépendante fresh est admise pour examiner les nouveaux risques de preuve : budgetE/R/D sans remise à zéro, horloges, cycles chevauchant l'expiration, succès transport réel et cleanup avant erreurs. Briefing et onze copies exactes sousTI fournis avant admission, sans conclusions du développeur. Aucun nouveau harness ou test de revue exécuté.

Le nouveau run122 reste **non consommé**. Il ne sera lancé qu'après verdict, contrôle des sources et préflight/bail frais. La verticale finale et la publication ne sont pas autorisées par cette étape. Les historiques109/110/117 restent séparés ; même un futur PASS122 ne fermerait que la brancheAC3 contrôlée.

Le reviewer signale un refus de lecture de la politique ContextMode externe. Le lead fournit une copie supplémentaire aux empreintes identiques sousTI/.tmp/post-expiry122-review-inputs/context-policy.md ; aucune permission contournée, même session de revue et scopeRO. Douze entrées locales sont désormais disponibles. Aucun test ou source modifié pour cette correction d'entrée.

## Annexe

- Source `services/Telegraph/tests/outbox_runtime_tests.rs`, SHA256 `0d50bb5dd66f8415dce557156a2d00bb7713ada9ee87f84e277aabab7d7234e3` ; patch `bcbaba79e7c38a1870892bc32c2941ac360eba09bd3098eecb6993d8b04f3694`, baseline403e8c90, ajout442lignes seulement.
- ArchiveTI/.tmp/component-post-expiry122-integrated/ :8fichiers source/patch/manifest/fmt/no-run/bail. no-run status0,27,06s ; diffcheck0 ; aucun test exécuté.
- Nouveau filtre exact : `committed_claim_recovers_same_event_after_measured_expiry_with_fixed_rebuild_deadline` dans cibleoutbox_runtime_tests. Préfixe des traces réutiliséTELEGRAPH_CONTROLLED_CLAIM117 mais stages122_ distincts : ne pas confondre les runs.
- Brief `.cursor/review-briefings/20261027-post-expiry122.md`, copiesTI/.tmp/post-expiry122-review-inputs/, tâche de revue `task_mvqriylt_0008_zb17jv` fresh/test-reviewer après préparation terminée.
- Fenêtre : t0avantlecture/factory,Rmesuré<=30s,Dfixet0+R+25<=55s,cleanup6s/phase<=61s. Précondition conservatrice de cohérenceUTC/monotone100ms sans élargirD, pas SLA produit.
- Localisation : helpers/test additionnels uniquement. Impact : preuve prospective de reprise aprèsexpiration. Limites : compilation/source review ne valent pas exécution, pas de nouvelle preuve verticale ou d'acceptation globale/pin/pub.
