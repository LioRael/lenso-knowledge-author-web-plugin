#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

test -f Cargo.toml
test -f Cargo.lock
test -f LICENSE
test -f README.md
test -f docs/plugin-card.md
test -f src/lib.rs
test -f src/assets.rs
test -f tests/composition.rs

if rg -n 'path\s*=\s*"|/Users/|\.\./' Cargo.toml Cargo.lock README.md docs src tests .github; then
  echo "Published sources must not depend on sibling repositories or machine-local paths." >&2
  exit 1
fi

if rg -n 'sqlx|axum|lenso-http-auth|lenso_http_auth' Cargo.toml src tests; then
  echo "Knowledge Author Web must not own transport, persistence, or an alternate Auth boundary." >&2
  exit 1
fi

rg -q 'plugin-id = "lenso\.knowledge-author\.web"' Cargo.toml
rg -q 'lenso-capability-knowledge-base.*rev = "[0-9a-f]{40}"' Cargo.toml
rg -q 'list_articles_with_context' src/lib.rs
rg -q 'get_draft_with_context' src/lib.rs
rg -q 'create_draft_with_context' src/lib.rs
rg -q 'update_draft_with_context' src/lib.rs
rg -q 'publish_article_with_context' src/lib.rs
rg -q 'Console.*no.*Web-shell|Console.*has no.*Web-shell' README.md docs/plugin-card.md
