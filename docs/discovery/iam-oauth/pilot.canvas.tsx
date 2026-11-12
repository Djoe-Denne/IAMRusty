import { Callout, Grid, H1, H2, Link, Pill, Row, Stack, Stat, Table, Text, useCanvasState, useHostTheme } from "cursor/canvas";

const revision = "f7b557f76920f47650997062bd23e335b6a85be9";
const features: Record<string, string> = {
  F01: "Démarrer un parcours OAuth",
  F02: "Autoriser un retour navigateur une seule fois",
  F03: "Créer ou retrouver le compte plateforme",
  F04: "Lier ou relier une identité à un compte",
};
const criteria = [
  { id: "IAM-OAUTH-C01", feature: "F01", title: "Démarrer LOGIN avec une redirection et un state propres au parcours", tests: "T06 · IT", k6: "direct", source: "services/IAMRusty/http/src/handlers/auth.rs#L290-L389", note: "Redirection303 et state/redirect inspectés ; un profil k6 synthétique seulement." },
  { id: "IAM-OAUTH-C02", feature: "F01", title: "Émettre un cookie navigateur sécurisé et host-only sous HTTPS", tests: "T01 · Unitaire", k6: "direct", source: "services/IAMRusty/http/src/oauth_browser.rs#L140-L161", note: "Secure, HttpOnly, SameSite=Lax, host-only ; variante HTTP locale distincte." },
  { id: "IAM-OAUTH-C03", feature: "F02", title: "Refuser un callback sans la bonne liaison navigateur/provider/intention ou expiré", tests: "T03 · IT", k6: "not_mapped_in_pilot", source: "services/IAMRusty/http/src/oauth_browser.rs#L222-L274", note: "Refus400 IAM avant complete. Les refus403 de route Envoy ne prouvent pas ce comportement." },
  { id: "IAM-OAUTH-C04", feature: "F02", title: "Ne consommer une transaction qu'une fois entre deux replicas", tests: "T04 · IT", k6: "not_mapped_in_pilot", source: "services/IAMRusty/infra/src/repository/oauth_transaction_write.rs#L70-L110", note: "Deux listeners, un writer : un gagnant et une purge. Pas d'obligation de dupliquer cette IT en E2E." },
  { id: "IAM-OAUTH-C05", feature: "F01", title: "Conserver PKCE S256 et le nonce attendu sans downgrade", tests: "T02 · Unitaire / T05 · IT", k6: "partial", source: "services/IAMRusty/http/src/oauth_browser.rs#L294-L330", note: "k6 vérifie S256 positif ; les refus de downgrade et le verifier/nonce durable sont examinés dans les autres tests." },
  { id: "IAM-OAUTH-C06", feature: "F03", title: "Demander la complétion d'un nouveau compte OAuth", tests: "T06 · IT", k6: "direct", source: "services/IAMRusty/http/src/handlers/auth.rs#L583-L647", note: "202 et registration_required ne sont pas une connexion complète." },
  { id: "IAM-OAUTH-C07", feature: "F03", title: "Compléter le compte puis se reconnecter avec une session plateforme persistée", tests: "T06 · IT", k6: "partial", source: "services/IAMRusty/http/src/handlers/auth.rs#L1048-L1086", note: "Q02 caractérisé :200 exact pour route/IT/k6. Prédicat JS vérifié hors navigateur ; pas de nouvel E2E. Le décodage des claims ne vérifie pas la signature." },
  { id: "IAM-OAUTH-C08", feature: "F04", title: "Conserver la cible LINK et le redirect autorisés, sans retargeting par query", tests: "T07 · IT", k6: "not_mapped_in_pilot", source: "services/IAMRusty/application/src/usecase/link_provider.rs#L219-L245", note: "Les assertions DB distinguent propriétaire et autre compte ; relink-identity seul n'est pas ce stimulus." },
  { id: "IAM-OAUTH-C09", feature: "F03", title: "Refuser LOGIN sans email fournisseur, même si l'identité est déjà liée", tests: "T08 · IT / U01 · Unitaire", k6: "not_mapped_in_pilot", source: "services/IAMRusty/domain/src/service/oauth_service.rs#L188-L249", note: "Q01 caractérisé :401 authentication_failed. RepositoryError distincte :500 login_failed (U02, mapping seulement). Pas de changement produit." },
  { id: "IAM-OAUTH-C10", feature: "F04", title: "Autoriser LINK et RELINK sans email fournisseur sans réattribuer l'email du compte", tests: "T08 · IT", k6: "not_mapped_in_pilot", source: "services/IAMRusty/domain/src/service/provider_link_service.rs#L195-L239", note: "Ne pas appliquer la règle email du LOGIN aux opérations LINK/RELINK." },
  { id: "IAM-OAUTH-C11", feature: "F03", title: "Préférer l'identité provider+ID liée à un email fournisseur ayant changé", tests: "T09 · IT", k6: "not_mapped_in_pilot", source: "services/IAMRusty/domain/src/service/oauth_service.rs#L188-L249", note: "UUID stable, email_verified vrai/faux ; ne prouve pas tous les cas d'identités non liées." },
  { id: "IAM-OAUTH-C12", feature: "F02", title: "Refuser proprement une panne de consommation writer, sans effet aval", tests: "T10 · IT", k6: "not_mapped_in_pilot", source: "services/IAMRusty/http/src/oauth_browser.rs#L277-L292", note: "503 générique, rollback puis présentation possible. À distinguer d'un retry après consommation commitée." },
  { id: "IAM-OAUTH-C13", feature: "F04", title: "Authentifier START RELINK, sans exiger de Bearer au callback lié", tests: "T11 · IT", k6: "direct", source: "services/IAMRusty/http/src/handlers/auth.rs#L1224-L1327", note: "IAM direct401 et edge403 ne sont pas contradictoires : contexte et frontière différents." },
];
const k6Labels: Record<string, string> = { direct: "Correspondance directe", partial: "Correspondance partielle", not_mapped_in_pilot: "Non rapproché en k6" };

export default function IamOAuthDiscoveryPilot() {
  const theme = useHostTheme();
  const [filter, setFilter] = useCanvasState("aiforall-iam-oauth-discovery-pilot-filter", "all");
  const selected = criteria.filter(c => filter === "all" || c.feature === filter || (filter === "questions" && ["IAM-OAUTH-C07", "IAM-OAUTH-C09"].includes(c.id)));
  return <Stack gap={22} style={{ padding: 28, maxWidth: 1500, margin: "0 auto", color: theme.text.primary }}>
    <Stack gap={6}>
      <Text size="small" tone="secondary">AIForAll · Identité plateforme · Proposition du 12 novembre 2026</Text>
      <H1>IAM/OAuth : des comportements reliés à leurs preuves</H1>
      <Text tone="secondary">Pilote code-first comparé aux ADR 0412/0413. Ni score global de couverture, ni rapport Allure exécuté.</Text>
    </Stack>
    <Callout tone="info" title="Caractérisation ciblée Q01/Q02">
      Deux tests de mapping401/500 et les17IT du binaire transactionnel passent. Fixture1créée/1retirée,0fuite. Pas de nouveau run Chromium/k6 ni de PASS global du catalogue. Classement encore proposé ; aucun comportement produit modifié.
    </Callout>
    <Grid columns="repeat(auto-fit, minmax(170px, 1fr))" gap={18}>
      <Stat value={Object.keys(features).length} label="Fonctionnalités proposées" />
      <Stat value={criteria.length} label="Critères avec assertions examinées" />
      <Stat value={criteria.filter(c => c.k6 !== "not_mapped_in_pilot").length} label="Correspondances k6, dont 2 partielles" />
      <Stat value="2" label="Oracles actuels caractérisés" tone="info" />
    </Grid>
    <Stack gap={8}>
      <H2>Les réponses actuelles deviennent les oracles</H2>
      <Text><strong>Q01 — LOGIN sans email :</strong>401 / authentication_failed. Une erreur repository suit une autre branche :500 / login_failed. Deux tests de mapping distincts vérifient aussi la confidentialité des causes. Les400 transactionnels et le503 writer restent séparés.</Text>
      <Text><strong>Q02 — Complétion :</strong>200 exact dans la route, l'IT et désormais le check k6. Prédicat réel exercé :200 accepté,201 refusé ; pas de nouveau run Chromium.</Text>
      <Text tone="secondary">Limite transversale : IdP protocolaire local et mocks ne qualifient ni fournisseur réel, ni équivalence GitLab sub, ni cutover.</Text>
    </Stack>
    <Stack gap={10}>
      <H2>Explorer les critères proposés</H2>
      <Row gap={6} wrap>
        <Pill active={filter === "all"} onClick={() => setFilter("all")}>Tous</Pill>
        <Pill active={filter === "questions"} onClick={() => setFilter("questions")}>Oracles Q01/Q02</Pill>
        {Object.entries(features).map(([id, title]) => <Pill key={id} active={filter === id} onClick={() => setFilter(id)}>{id} · {title}</Pill>)}
      </Row>
      <Table headers={["Critère / code observé", "Fonctionnalité proposée", "Assertions lues", "Lien k6", "Limite ou question"]}
        rows={selected.map(c => [<Stack key={c.id} gap={4}><Link href={`https://github.com/Djoe-Denne/IAMRusty/blob/${revision}/${c.source}`}>{c.id}</Link><Text size="small">{c.title}</Text></Stack>, features[c.feature], c.tests, k6Labels[c.k6], c.note])}
        rowTone={selected.map(c => ["IAM-OAUTH-C07", "IAM-OAUTH-C09"].includes(c.id) ? "warning" : "neutral")}
        striped stickyHeader />
      <Text size="small" tone="secondary">Non rapproché en k6 ne signifie pas sans test : les assertions IT sélectionnées portent déjà ces comportements. Une correspondance partielle ne valide pas tout le critère.</Text>
    </Stack>
    <Stack gap={8}>
      <H2>Comment les ADR alimenteraient la suite</H2>
      <Text>Décision → critère candidat → preuve dans le code → assertion identifiée → résultat pour une version et un environnement. Garder ces cinq états séparés.</Text>
      <Text tone="secondary">Le statut Accepted d'une ADR n'est pas un résultat d'exécution. Partial n'efface pas ses tranches livrées. Un référentiel dérivé du seul code ne peut pas découvrir toutes les attentes absentes du produit.</Text>
      <Text size="small" tone="tertiary">Source : docs/discovery/iam-oauth/catalogue.json · Code à f7b557f · Caractérisation du 12 novembre 2026 :2unités et17IT PASS. Référence E2E historique :54a19ba, slice locale uniquement. Les liens pointent vers le code à la révision examinée.</Text>
    </Stack>
  </Stack>;
}
