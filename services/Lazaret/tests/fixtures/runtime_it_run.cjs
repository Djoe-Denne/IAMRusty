// Bounded campaign observer only: starts no dependency and deletes no resource.
// Exact Cargo args are supplied by the lead. Engine events do not grant ownership;
// only a matching fresh fixture created record permits an exact-ID graceful stop.
const fs = require('node:fs');
const path = require('node:path');
const cp = require('node:child_process');
const crypto = require('node:crypto');

const root = path.resolve(__dirname, '../../../..');
const [pkg, target] = process.argv.slice(2);
if (!pkg || !target) throw Error('package and single test target required');
const run = `it-${crypto.randomUUID().replaceAll('-', '')}`;
const evidence = path.join(root, '.tmp', `itfixtures-${run}`);
fs.mkdirSync(evidence, { recursive: true });
const registry = path.join(root, '.tmp', `docker-resources-${new Date().toISOString().slice(0, 10).replaceAll('-', '')}.log`);
const witnessed = new Set();
const issues = [];
const resources = [];

function native(exe, args, timeout = 30000) {
    const result = cp.spawnSync(exe, args, { cwd: root, encoding: 'utf8', timeout });
    if (result.status !== 0) throw Error(`${exe} ${args[0]} failed: ${result.stderr || result.error}`);
    return result.stdout.trim();
}
function save(name, data) {
    fs.writeFileSync(path.join(evidence, name), JSON.stringify(data, null, 2));
}
function record(data) {
    const fd = fs.openSync(registry, 'a');
    try {
        fs.writeSync(fd, JSON.stringify({ ts: new Date().toISOString(), actor: 'tests-t', task: run, ...data }) + '\n');
        fs.fsyncSync(fd);
    } finally { fs.closeSync(fd); }
}
function inventory() {
    const text = native('docker', ['ps', '-a', '--no-trunc', '--format', '{{json .}}']);
    return text ? text.split(/\r?\n/).map(line => JSON.parse(line)) : [];
}
function observe(event) {
    if (event.Action !== 'create' && event.Action !== 'pull') return;
    const kind = event.Type;
    const id = event.Actor?.ID || event.id || 'UNKNOWN';
    if (kind === 'container' && event.Action === 'create') witnessed.add(id);
    const identity = `${kind}:${id}:${event.timeNano}`;
    if (resources.includes(identity)) return;
    resources.push(identity);
    record({
        kind, id: kind === 'image' && !id.startsWith('sha256:') ? 'UNKNOWN' : id,
        event: 'engine_creation_witness', prior_state: 'absent', safe_to_delete: 'ask',
        safe_reason: 'Independent Engine event, ownership pending fresh fixture ledger; not cleanup authority',
        observed_actor_id: id, engine_time_nano: String(event.timeNano), session_ledger: evidence,
    });
}
function decodeLines(file) {
    const bytes = fs.readFileSync(file);
    if (bytes[0] === 0xff && bytes[1] === 0xfe) return bytes.subarray(2).toString('utf16le').split(/\r?\n/);
    return bytes.toString('latin1').split('\n').map(line => {
        const raw = Buffer.from(line, 'latin1');
        try { return new TextDecoder('utf-8', { fatal: true }).decode(raw); }
        catch { return new TextDecoder('windows-1252', { fatal: true }).decode(raw); }
    });
}
function foreignHold(id) {
    const latest = new Map();
    for (const file of fs.readdirSync(path.join(root, '.tmp')).filter(name => /^docker-resources-.*\.log$/.test(name)).sort()) {
        for (const line of decodeLines(path.join(root, '.tmp', file))) {
            if (!line.includes(id)) continue;
            let row;
            try { row = JSON.parse(line); } catch { return true; }
            if (row.id === id && row.task !== run) latest.set(row.task || 'unknown', row);
        }
    }
    return [...latest.values()].some(row => row.safe_to_delete !== 'yes');
}

async function main() {
    const before = inventory();
    const beforeIds = new Set(before.map(row => row.ID));
    save('before.json', before);
    save('source.json', { head: native('git', ['rev-parse', 'HEAD']), pkg, target, run });
    const since = native('docker', ['info', '--format', '{{.SystemTime}}']);
    const eventArgs = ['events', '--since', since, '--format', '{{json .}}'];
    const observer = cp.spawn('docker', eventArgs, { cwd: root, stdio: ['ignore', 'pipe', 'pipe'] });
    let pending = '';
    let observerError = '';
    observer.stdout.on('data', bytes => {
        const text = bytes.toString('utf8');
        fs.appendFileSync(path.join(evidence, 'engine-events.jsonl'), text);
        pending += text;
        let newline;
        while ((newline = pending.indexOf('\n')) >= 0) {
            const line = pending.slice(0, newline); pending = pending.slice(newline + 1);
            if (line.trim()) { try { observe(JSON.parse(line)); } catch (error) { issues.push(String(error)); } }
        }
    });
    observer.stderr.on('data', bytes => { observerError += bytes.toString('utf8'); });
    await new Promise((resolve, reject) => { observer.once('spawn', resolve); observer.once('error', reject); });
    const args = ['test', '--locked', '-j', '1', '-p', pkg, '--test', target];
    const start = Date.now();
    const stdout = fs.createWriteStream(path.join(evidence, 'cargo.stdout.log'));
    const stderr = fs.createWriteStream(path.join(evidence, 'cargo.stderr.log'));
    const outputFinished = Promise.all([
        new Promise(resolve => stdout.once('finish', resolve)),
        new Promise(resolve => stderr.once('finish', resolve)),
    ]);
    const env = { ...process.env, CARGO_TARGET_DIR: 'E:\\cargo-target\\AIForAll',
        RUSTYCOG_TEST_RUNNER_MODE: 'local', RUSTYCOG_TEST_RUN_ID: run,
        RUSTYCOG_TEST_LEDGER_DIR: evidence, RUST_TEST_THREADS: '1' };
    // Empty is a configured (invalid) hostname, not an unset override.
    delete env.RUSTYCOG_TEST_RUNNER_HOST;
    const child = cp.spawn('cargo', args, {
        cwd: root,
        env,
        stdio: ['ignore', 'pipe', 'pipe'], timeout: 900000,
    });
    child.stdout.pipe(stdout); child.stderr.pipe(stderr);
    let spawnError;
    child.once('error', error => { spawnError = String(error); });
    const exit = await new Promise(resolve => child.once('close', resolve));
    await outputFinished;
    const seconds = (Date.now() - start) / 1000;
    const until = native('docker', ['info', '--format', '{{.SystemTime}}']);
    // A bounded Engine replay drains creation events independently of callbacks.
    const replay = native('docker', [...eventArgs, '--until', until]);
    for (const line of replay.split(/\r?\n/).filter(Boolean)) observe(JSON.parse(line));
    observer.kill();
    if (observerError.trim()) issues.push(`Engine witness stderr: ${observerError.trim()}`);
    const owned = new Map();
    for (const file of fs.readdirSync(evidence).filter(name => /^fixture-.*\.jsonl$/.test(name))) {
        for (const line of decodeLines(path.join(evidence, file)).filter(line => line.trim())) {
            const row = JSON.parse(line);
            if (row.run_id !== run) { issues.push(`foreign fixture ledger ${file}`); continue; }
            if (String(row.event).startsWith('unknown')) issues.push(`opaque fixture event in ${file}`);
            if (row.event !== 'created') continue;
            const id = row.container_id;
            if (!/^[a-f0-9]{64}$/.test(id) || beforeIds.has(id) || !witnessed.has(id)) {
                issues.push(`unproved fixture container ${id}`); continue;
            }
            owned.set(id, path.join(evidence, file));
        }
    }
    const afterRun = inventory();
    save('after-run.json', afterRun);
    for (const [id, ledger] of owned) {
        if (foreignHold(id)) { issues.push(`foreign/ambiguous hold ${id}`); continue; }
        let row = afterRun.find(item => item.ID === id);
        record({ kind: 'container', id, event: 'reclassified', prior_state: 'absent', safe_to_delete: 'yes',
            safe_reason: 'Fresh fixture ledger and independent Engine create witness match exact ID; campaign batch ended', session_ledger: ledger });
        if (row?.State === 'running') {
            // Permanent permission: reversible graceful stop only, never rm/prune.
            const inspected = JSON.parse(native('docker', ['inspect', '--format', '{{json .State}}', id]));
            if (inspected.Running) native('docker', ['stop', '--time', '-1', id], 120000);
            record({ kind: 'container', id, event: 'stopped', safe_to_delete: 'yes',
                safe_reason: 'Exact-ID owned fixture graceful stop, volatile fixture memory released', session_ledger: ledger });
        }
    }
    const after = inventory();
    save('after-cleanup.json', after);
    const runningOwned = after.filter(row => owned.has(row.ID) && row.State === 'running').map(row => row.ID);
    const output = fs.readFileSync(path.join(evidence, 'cargo.stdout.log'), 'utf8') + '\n' + fs.readFileSync(path.join(evidence, 'cargo.stderr.log'), 'utf8');
    const summary = { run, evidence, registry, pkg, target, exit, spawnError, seconds,
        before, afterRun, after, owned: [...owned.keys()], runningOwned, issues,
        resultLines: output.split(/\r?\n/).filter(line => /^(test result:|running \d+ tests|test .*FAILED|thread .*panicked|\s*Finished|error:)/.test(line)) };
    save('summary.json', summary);
    console.log(JSON.stringify(summary, null, 2));
    process.exitCode = exit === 0 && !runningOwned.length && !issues.length ? 0 : 1;
}
main().catch(error => { console.error(String(error)); process.exitCode = 1; });
