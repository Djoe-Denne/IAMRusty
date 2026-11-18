const fs = require("node:fs");
const crypto = require("node:crypto");

function exportVariable(name, value) {
  const delimiter = `cache_${crypto.randomUUID()}`;
  fs.appendFileSync(process.env.GITHUB_ENV, `${name}<<${delimiter}\n${value}\n${delimiter}\n`);
}

// These values are runner-issued, never workflow inputs. A JavaScript action
// receives them; shell steps do not necessarily receive the runtime token.
if (!process.env.ACTIONS_RESULTS_URL || !process.env.ACTIONS_RUNTIME_TOKEN) {
  // sccache 0.19.0 enables GHA whenever a non-empty VERSION is present,
  // even if ENABLED is false. Clear both to actually select the local backend.
  exportVariable("SCCACHE_GHA_VERSION", "");
  exportVariable("SCCACHE_GHA_ENABLED", "false");
  process.stdout.write("Remote compiler-cache capability unavailable; local compilation remains enabled.\n");
} else {
  const token = process.env.ACTIONS_RUNTIME_TOKEN;
  const escaped = token.replaceAll("%", "%25").replaceAll("\r", "%0D").replaceAll("\n", "%0A");
  process.stdout.write(`::add-mask::${escaped}\n`);
  exportVariable("ACTIONS_RESULTS_URL", process.env.ACTIONS_RESULTS_URL);
  exportVariable("ACTIONS_RUNTIME_TOKEN", token);
}
