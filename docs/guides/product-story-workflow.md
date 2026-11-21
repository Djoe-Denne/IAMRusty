# Product story workflow

Backlog.md manages the authoritative Markdown stories. Obsidian opens those
same files in the AIForAll vault, and the existing QMD collection `aiforall-wiki`
indexes them. Neither owns a copy. Use the Backlog CLI to create or change any
managed story, including its status, links and acceptance criteria. Never patch
the generated Markdown, rename it by hand, or edit it in Obsidian.

## Local setup

Install the qualified CLI version if it is not already available:

```bash
npm install --global backlog.md@1.56.0 --ignore-scripts
backlog --version
```

Run Backlog commands from the intended Git checkout or worktree root with no
`BACKLOG_CWD` override unless explicitly targeting a sandbox. The committed
[`backlog.config.yml`](../../backlog.config.yml) is already the project setup;
do not rerun `backlog init`. It disables Backlog's Git operations,
auto-commit and hook bypass. Git review and publication still follow
[`AGENTS.md`](../../AGENTS.md). No Backlog MCP is required.

```bash
backlog task list --plain
backlog task view STORY-001 --plain
```

The second command is an example for when that ID exists. The optional local
board is `backlog browser --host 127.0.0.1 --no-open`. Do not expose it on the
network. The CLI, not the board or Obsidian, is the workshop's editing interface.

## From an interview to a story

In OpenCode, start or resume in the current conversation with
`/product-workshop <topic or workshop reference>`.

1. **Discover.** Locate the user goal on the story map, then use examples to
   decide one behavior at a time. Do not consult the implementation, the
   retrospective audit or the wiki to decide what the product ought to do.
   Save only the useful summary in `.tmp/product-workshops/<slug>/intent.md`
   in the primary checkout so temporary worktree cleanup cannot lose it. It is
   an interview note, not a Backlog file. Keep drafts, transcripts and private
   evidence outside the vault and the Git diff.
2. **Approve.** Record the user's explicit approval of a specific intent
   revision: actor, outcome, agreed rules, numbered examples and what is out of
   scope. Track undecided questions as questions; blocking ones prevent approval
   of the affected behavior. Approval of intent is not permission to publish or
   implement it. This repository is public.
3. **Record.** Once the user approves the public wording, create one story in
   Backlog with a `--description`, one `--ac` per approved example, and `--doc`
   for relevant documentation. Where a link should be clickable in Obsidian or
   GitHub, put a Markdown link in the description via the CLI, not only in the
   documentation property. Use the actual `STORY-…` ID returned by the CLI.
   Allocate IDs from the latest shared board, with one writer at a time until
   that change is integrated; independent worktrees can otherwise allocate the
   same ID. There is no shared private-draft promotion: private interview notes
   stay outside the vault.
4. **Compare.** After approval of intent, compare its examples against a
   specified Git commit. Record **Matches**, **Differs** or **Unverified** for
   each example, with evidence and evidence limits. Keep the report separate
   from the story. Private reports stay with the interview; approved public
   reports can live under `obsidian/AI FOR ALL/projects/aiforall/product-evidence/`
   and be attached with `backlog task edit <id> --add-doc <repo-relative-path>`.
   A discrepancy is not an implementation order.
5. **Hand off if asked.** Planning and implementation require explicit scope
   from the user. The PM reads the approved story, comparison and authorized
   scope, then returns a plan. Already satisfied behavior does not need work.

`Accepted intent` is the initial board column. Move to `Ready` only after
delivery is requested, `In Progress` for work actually started, and `Done`
only when the outcome and its resolution are verified. No status change on its
own authorizes a reviewer or agent to run implementation.

Example CLI shape (use only approved content in an actual story):

```bash
backlog task create "<user goal>" \
  --description "<actor, outcome, approved rules and scope>" \
  --ac "E1: Given ..., when ..., then ..." \
  --ac "E2: Given ..., when ..., then ..." \
  --doc "docs/guides/<related-guide>.md" --plain
```

## Discoverability and freshness

- Open [Product stories](../../obsidian/AI%20FOR%20ALL/projects/aiforall/product-stories.md)
  in the vault. The story files are in `projects/aiforall/product-backlog/tasks/`
  in that same vault. Use the stable ID in cross-references: a title edit can
  rename a file, so resolve its current path with `backlog task view`.
- After the normal `aiforall-wiki` index refresh, agents can search the ID and
  related terms with QMD, then fetch the source. Prefer an exact lookup first
  (for example `qmd search STORY-001 -c aiforall-wiki`). Hybrid retrieval
  additionally depends on embedding freshness. No separate collection, mirror,
  update hook or automatic publication is needed.
- QMD reads the primary checkout's vault, not an unpublished worktree. An
  empty QMD result is not evidence that a story is absent. Check the relevant
  checkout and Backlog CLI. Use the working story for edits and a pinned Git
  revision for durable evidence.

See the [`aiforall-product-workshop`](../../.agents/skills/aiforall-product-workshop/SKILL.md)
and [`aiforall-story-handoff`](../../.agents/skills/aiforall-story-handoff/SKILL.md)
skills for facilitation and reconciliation.
