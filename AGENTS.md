<!-- headroom:memory-instructions -->
## Memory

Use the `headroom_memory` MCP server for persistent cross-session knowledge.

**Before** answering questions about prior decisions, conventions, project context,
architecture, user preferences, org info, codenames, debugging history, or anything
from past sessions — call `memory_search` first.

**After** making durable decisions, discovering conventions, or learning important
facts — call `memory_save` to persist them for future sessions.

Memory is your first source of truth for anything not visible in the current conversation.

## Compilation locale

L’hôte est Windows. Kind et les tests locaux tournent dans la VM Linux de Docker Desktop.

- Compiler dans Docker pour les images Kind et les tests locaux. Kind charge l’image (`kind load`), il ne compile pas.
- `cargo build`, `cargo test` et `cargo check` sur Windows uniquement si un binaire Windows est demandé, ou si l’outil ne peut pas tourner dans Docker. Un seul `cargo` à la fois.
- Ne pas partager `target\` entre Windows (`x86_64-pc-windows-msvc`) et Docker (`x86_64-unknown-linux-gnu`). Un `cargo check` hôte ne réutilise pas les artefacts Linux.
