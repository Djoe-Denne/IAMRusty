const fs = require("fs");
const path = require("path");
const crypto = require("crypto");

const vault = "C:/Users/djden/source/repos/AIForAll/obsidian/AI FOR ALL";
const repo = "C:/Users/djden/source/repos/AIForAll";
const transcripts = "C:/Users/djden/.cursor/projects/c-Users-djden-source-repos-AIForAll/agent-transcripts";
const manifestPath = path.join(vault, ".manifest.json");
const ingestedAt = "2026-09-27T09:20:00Z";
const skipSession = "91f31192-d8a5-4867-bc8a-160e6d93b9e4";
const head = "2473baa7a54595931a4eff76a33431f56776db20";

const pagesCreated = [
  "projects/aiforall/decisions/0304-access-jwt-trust.md",
  "projects/aiforall/decisions/0605-gold-path-kind.md",
  "projects/manifesto/decisions/0009-0011-gates-preprod.md",
  "journal/2026-09-27.md",
];
const pagesUpdated = [
  "projects/aiforall/aiforall.md",
  "projects/aiforall/decisions/index.md",
  "projects/aiforall/decisions/0300-events-authz.md",
  "projects/aiforall/decisions/0603-tranche-locale.md",
  "projects/aiforall/decisions/0604-j3-overlay-demo-monolith.md",
  "projects/aiforall/concepts/jwt-issuer-vs-consumer.md",
  "projects/aiforall/concepts/orchestrator-agent-harness.md",
  "projects/aiforall/references/cursor-history-2026-09.md",
  "projects/manifesto/decisions/index.md",
  "projects/manifesto/decisions/0008-apparatus-p4-k8s.md",
  "projects/iamrusty/iamrusty.md",
  "projects/iamrusty/decisions/index.md",
  "projects/hive/hive.md",
  "projects/lazaret/concepts/workload-identity.md",
  "index.md",
  "log.md",
];

function walk(dir, acc = []) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walk(p, acc);
    else if (e.name.endsWith(".jsonl")) acc.push(p);
  }
  return acc;
}

function sha256(file) {
  return crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");
}

const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
const sources = manifest.sources;
let added = 0;
let updated = 0;
let skippedCurrent = 0;
let parents = 0;
let nested = 0;

for (const file of walk(transcripts)) {
  const key = file.replace(/\\/g, "/");
  if (key.includes("/" + skipSession + "/")) {
    skippedCurrent += 1;
    continue;
  }
  const st = fs.statSync(file);
  const hash = "sha256:" + sha256(file);
  const prev = sources[key];
  const isNested = key.includes("/subagents/");
  if (!prev) {
    sources[key] = {
      ingested_at: ingestedAt,
      size_bytes: st.size,
      modified_at: st.mtime.toISOString(),
      content_hash: hash,
      source_type: "document",
      project: "aiforall",
      pages_created: pagesCreated,
      pages_updated: pagesUpdated,
    };
    added += 1;
    if (isNested) nested += 1;
    else parents += 1;
  } else if (prev.content_hash !== hash) {
    prev.ingested_at = ingestedAt;
    prev.size_bytes = st.size;
    prev.modified_at = st.mtime.toISOString();
    prev.content_hash = hash;
    prev.pages_updated = pagesUpdated;
    updated += 1;
    if (isNested) nested += 1;
    else parents += 1;
  }
}

const adrDir = path.join(repo, "docs", "adr");
let adrTouched = 0;
for (const name of fs.readdirSync(adrDir)) {
  if (!name.endsWith(".md")) continue;
  const file = path.join(adrDir, name);
  const key = file.replace(/\\/g, "/");
  const st = fs.statSync(file);
  const hash = "sha256:" + sha256(file);
  const prev = sources[key];
  if (!prev) {
    sources[key] = {
      ingested_at: ingestedAt,
      size_bytes: st.size,
      modified_at: st.mtime.toISOString(),
      content_hash: hash,
      source_type: "document",
      project: "aiforall",
      pages_created: [],
      pages_updated: pagesUpdated.concat(pagesCreated),
    };
    adrTouched += 1;
    added += 1;
  } else if (prev.content_hash !== hash) {
    prev.ingested_at = ingestedAt;
    prev.size_bytes = st.size;
    prev.modified_at = st.mtime.toISOString();
    prev.content_hash = hash;
    const merged = new Set([...(prev.pages_updated || []), ...pagesUpdated, ...pagesCreated]);
    prev.pages_updated = [...merged];
    adrTouched += 1;
    updated += 1;
  }
}

const project = manifest.projects.aiforall;
project.last_synced = ingestedAt;
project.last_commit_synced = head;
project.sync_note = "JWT 0304-0309, gold path 0605, IdP Connect crates, gates 0009-0011";
project.conversations_ingested = (project.conversations_ingested || 0) + parents;
const pages = new Set(project.pages_in_vault || []);
for (const p of pagesCreated.concat([
  "projects/aiforall/decisions/0304-access-jwt-trust.md",
  "projects/aiforall/decisions/0605-gold-path-kind.md",
  "projects/manifesto/decisions/0009-0011-gates-preprod.md",
])) pages.add(p);
project.pages_in_vault = [...pages];

manifest.stats.total_sources_ingested = Object.keys(sources).length;
manifest.stats.total_pages = (manifest.stats.total_pages || 0) + pagesCreated.length;
manifest.stats.last_commit_synced = head;

fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2) + "\n");
console.log(JSON.stringify({
  added,
  updated,
  skippedCurrent,
  parentsTouched: parents,
  nestedTouched: nested,
  adrTouched,
  sources: manifest.stats.total_sources_ingested,
  pages: manifest.stats.total_pages,
  conversations_ingested: project.conversations_ingested,
}));
