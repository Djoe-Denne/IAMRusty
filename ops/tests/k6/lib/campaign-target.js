// Public, closed campaign metadata only. No credentials or trust overrides.
// Legacy consumers retain their historical profile until the native parent is qualified.
export function campaignTarget() {
  if (__ENV.K6_CAMPAIGN_FILE === undefined) {
    return {
      origin: 'http://127.0.0.1:18080',
      provider: 'http://localhost:18081/oauth/authorize',
      issuer: 'http://127.0.0.1:8080/iam', audience: 'aiforall', secureCookie: false,
      cookieName: 'iam-oauth-browser', cookiePath: '/iam',
    };
  }
  try {
    const text = open(__ENV.K6_CAMPAIGN_FILE);
    if (text.length > 8192) throw new Error();
    const value = JSON.parse(text);
    const fields = ['audience', 'issuer', 'origin', 'profile', 'provider', 'run_id', 'version'];
    if (Object.keys(value).sort().join(',') !== fields.join(',') || value.version !== 1
        || value.profile !== 'iam-native-parity' || !/^[a-f0-9]{32}$/.test(value.run_id)
        || value.origin !== 'https://127.0.0.1:18443'
        || value.provider !== 'https://localhost:18445/oauth/authorize'
        || typeof value.issuer !== 'string' || !/^https:\/\/[A-Za-z0-9.-]+(?::[0-9]+)?\/iam$/.test(value.issuer)
        || value.audience !== 'aiforall'
        || (__ENV.K6_RUN_ID !== undefined && __ENV.K6_RUN_ID !== value.run_id)) throw new Error();
    return { ...value, secureCookie: true, cookieName: '__Host-iam-oauth-browser', cookiePath: '/' };
  } catch { throw new Error('NATIVE_CAMPAIGN_INVALID'); }
}
