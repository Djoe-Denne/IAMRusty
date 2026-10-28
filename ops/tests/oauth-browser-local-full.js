/* Sequence for an EXISTING permitted browser driver, not a runner or permission
 * to call arbitrary-code MCP operations. No installs, HAR/trace/screenshots,
 * APIRequestContext, forged cookies or global Authorization. Credentials never
 * leave this function. Parent must correlate loaded config and gateway/IAM
 * observations before declaring N1/N2/N3 or the campaign PASS.
 *
 * Future use by a profile with a permitted JS driver: await oauthBrowserProof(
 *   dedicatedPage, {runId, expectedEmail}); do not print the page's body/URL.
 */
async function oauthBrowserProof(page, {runId, expectedEmail}) {
  const origin = "http://127.0.0.1:18080";
  if (!/^[a-z0-9]{12,32}$/.test(runId) || expectedEmail !== `${runId}@example.invalid`)
    throw new Error("invalid dedicated proof account");
  const fail = () => { throw new Error("OAuth browser assertion failed (redacted)"); };
  let callbackBearer = false, observedAuthorize = 0, callbackCookie = false, negativeCookie = false;
  let idpCredentials = false, loginBearer = false, loginCookie = false;
  const pendingHeaders = [];
  const requestObserver = request => {
    const url = new URL(request.url());
    if (url.origin === "http://localhost:18081" && url.pathname === "/oauth/authorize") {
      observedAuthorize++;
      pendingHeaders.push(request.allHeaders().then(headers => {
        idpCredentials ||= Boolean(headers.authorization || headers.cookie);
      }));
    }
    if (url.origin === origin && ["/iam/api/auth/gitlab/callback",
        "/iam/api/auth/gitlab/relink-callback"].includes(url.pathname)) {
      pendingHeaders.push(request.allHeaders().then(headers => {
        callbackBearer ||= Boolean(headers.authorization);
      }));
    }
    if (url.origin === origin && url.pathname === "/iam/api/auth/gitlab/login")
      pendingHeaders.push(request.allHeaders().then(headers => { loginBearer ||= Boolean(headers.authorization); }));
  };
  const responseObserver = response => {
    const url = new URL(response.url());
    if (url.origin === origin && ["/iam/api/auth/gitlab/callback",
        "/iam/api/auth/gitlab/relink-callback"].includes(url.pathname))
      pendingHeaders.push(response.allHeaders().then(headers => { callbackCookie ||= Boolean(headers["set-cookie"]); }));
    if (url.origin === origin && ["/iam/api/auth/gitlab/link", "/iam/api/auth/gitlab/relink-start",
        "/iam/api/auth/gitlab/relink/callback"].includes(url.pathname) && [401, 403].includes(response.status()))
      pendingHeaders.push(response.allHeaders().then(headers => { negativeCookie ||= Boolean(headers["set-cookie"]); }));
    if (url.origin === origin && url.pathname === "/iam/api/auth/gitlab/login" && response.status() === 303)
      pendingHeaders.push(response.allHeaders().then(headers => {
        const cookie = headers["set-cookie"] || "";
        loginCookie ||= cookie.includes("iam-oauth-browser=") && /HttpOnly/i.test(cookie)
          && /SameSite=Lax/i.test(cookie) && /Max-Age=600(?:;|$)/i.test(cookie)
          && /Path=\/iam(?:;|$)/i.test(cookie) && !/;\s*(Secure|Domain=)/i.test(cookie);
      }));
  };
  page.on("request", requestObserver);
  page.on("response", responseObserver);
  try {
    // Same page/context owns its native cookie jar for the entire run.
    async function login() {
      const response = await page.goto(origin + "/iam/api/auth/gitlab/login");
      if (!response || ![200, 202].includes(response.status())) fail();
      const url = new URL(response.url());
      if (url.origin !== origin || url.pathname !== "/iam/api/auth/gitlab/callback") fail();
      return {status: response.status(), body: await response.json()};
    }
    let result = await login();
    if (result.status === 202) {
      if (result.body.operation !== "registration_required" || !result.body.registration_token) fail();
      const status = await page.evaluate(async ({registration_token, username}) => {
        const response = await fetch("/iam/api/auth/complete-registration", {
          method: "POST", credentials: "same-origin", redirect: "error",
          headers: {"Content-Type": "application/json"},
          body: JSON.stringify({registration_token, username})});
        return response.status;
      }, {registration_token: result.body.registration_token, username: runId});
      if (![200, 201].includes(status)) fail();
      // 202 was preparation, never counted as P2. Require a new complete login.
      result = await login();
    }
    if (result.status !== 200 || result.body.operation !== "login"
        || !result.body.access_token || result.body.user.email !== expectedEmail) fail();
    // Observation only, never local JWT verification or an authorization bypass:
    // the real gateway must verify this same token at P3.
    const claims = JSON.parse(Buffer.from(result.body.access_token.split(".")[1], "base64url").toString("utf8"));
    if (claims.iss !== "http://127.0.0.1:8080/iam" || claims.aud !== "aiforall") fail();
    const userId = result.body.user.id;
    const cookies = await page.context().cookies(origin + "/iam/api/auth/gitlab/callback");
    const nonce = cookies.find(cookie => cookie.name === "iam-oauth-browser");
    if (!nonce || nonce.domain !== "127.0.0.1" || nonce.path !== "/iam"
        || !nonce.httpOnly || nonce.secure || nonce.sameSite !== "Lax") fail();
    const start = await page.evaluate(async token => {
      const response = await fetch("/iam/api/auth/gitlab/relink-start", {
        method: "GET", credentials: "same-origin", redirect: "error",
        headers: {Authorization: "Bearer " + token}});
      return {status: response.status, body: await response.json()};
    }, result.body.access_token);
    if (start.status !== 200 || typeof start.body.auth_url !== "string") fail();
    const authorization = new URL(start.body.auth_url);
    if (authorization.origin !== "http://localhost:18081" || authorization.pathname !== "/oauth/authorize") fail();
    const relink = await page.goto(start.body.auth_url); // Unchanged URL; no JWT.
    if (!relink || relink.status() !== 200) fail();
    const relinkUrl = new URL(relink.url());
    if (relinkUrl.origin !== origin || relinkUrl.pathname !== "/iam/api/auth/gitlab/relink-callback") fail();
    const linked = await relink.json();
    if (linked.user.id !== userId || linked.user.email !== expectedEmail || !Array.isArray(linked.emails)) fail();
    const negatives = [];
    for (const path of ["/iam/api/auth/gitlab/link", "/iam/api/auth/gitlab/relink-start",
        "/iam/api/auth/gitlab/relink/callback"]) {
      const observed = await page.evaluate(async path => {
        const response = await fetch(path, {method: "GET", credentials: "same-origin", redirect: "error"});
        return {status: response.status};
      }, path);
      if (![401, 403].includes(observed.status)) fail();
      negatives.push({path, status: observed.status});
    }
    await Promise.all(pendingHeaders);
    if (callbackBearer || callbackCookie || negativeCookie || idpCredentials || loginBearer
        || !loginCookie || observedAuthorize < 2) fail();
    return {runId, slug: "gitlab", P1: "browser observed", P2: "login200",
      P3: "exact relink callback", negatives,
      verdict: "PENDING_GATEWAY_IAM_IDP_CORRELATION",
      required: ["same-run provider authorize/token/profile+PKCE counters", "loaded Envoy config identity",
        "N1-N3 no IAM dispatch/transaction/consumption or vendor exchange", "same-SHA final IT gates"]};
  } catch (_) {
    // Browser exceptions often contain callback URLs (state/code). Never expose
    // the original error, cause, response body or credentials to the driver log.
    throw new Error("OAuth browser proof failed; sensitive details redacted");
  } finally {
    page.off("request", requestObserver);
    page.off("response", responseObserver);
  }
}

if (typeof module !== "undefined") module.exports = {oauthBrowserProof};
