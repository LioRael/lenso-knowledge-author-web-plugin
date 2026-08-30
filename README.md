# Lenso Knowledge Author Web Plugin

`lenso.knowledge-author.web` is a removable linked-native workspace for the job “turn one solved problem into a reviewed, published answer.” It lists an organization's article drafts, reloads an exact draft, creates and updates drafts, and publishes the exact revision the author reviewed.

The Plugin owns only the `/knowledge` presentation, typed HTTP decoding, authentication evidence selection, ActorAssertion forwarding, and HTTP error representation. It has no database and owns no article, revision, publication, idempotency, membership, role, or authorization fact. Every operation delegates to exactly one bound `lenso.knowledge-base@1` Provider after authenticating through exactly one `lenso.auth@1` Provider.

## Routes

| Operation | HTTP route | Knowledge Base operation |
| --- | --- | --- |
| `knowledge.author.web.page` | `GET /knowledge` | Embedded static asset |
| `knowledge.author.web.articles.list` | `GET /api/knowledge/articles` | `list_articles` |
| `knowledge.author.web.articles.create` | `POST /api/knowledge/articles` | `create_draft` |
| `knowledge.author.web.articles.get` | `GET /api/knowledge/articles/{article_id}` | `get_draft` |
| `knowledge.author.web.articles.update` | `PATCH /api/knowledge/articles/{article_id}` | `update_draft` |
| `knowledge.author.web.articles.publish` | `POST /api/knowledge/articles/{article_id}/publish` | `publish_article` |

All article routes require a Bearer credential. Invalid or missing credentials produce `401`; a non-user actor or Knowledge Base denial produces `403`; absent articles produce `404`; and slug, idempotency, or revision conflicts produce `409`. Unknown generated Domain Errors become Runtime protocol violations, and dependency Runtime Failures cross the endpoint unchanged.

## Host composition

This crate is a linked native Plugin, not a portable Bundle or an HTTP server. A Host must link the crate, retain it through `lenso_knowledge_author_web_plugin::link()`, make linked factories available, activate one `web` root Instance, and bind:

- one `lenso.auth@1` Provider;
- one `lenso.knowledge-base@1` descriptor `1.1.0` Provider; and
- Web Ingress's `many lenso.http.endpoint@1` requirement to this Instance.

The Knowledge Base Provider remains the final authorization owner. Its configuration must allow this exact caller Instance for the selected organization and require the appropriate Auth audiences, organization membership, and `knowledge-base.articles.edit` role. The author workspace never grants itself access.

The generic `lenso run` command does not distribute arbitrary linked-native Web Plugins. The current Console has no Web-shell contribution contract, so installation does not add Console navigation; the Host explicitly routes `/knowledge`.

The embedded page remembers only the organization preference in browser local storage. The Bearer credential remains in page memory and must be entered again after a reload; production Hosts should normally supply credentials through their own secure session ingress policy.

Removing the Instance removes only the author routes and assets. Existing drafts and publications, the public Help Center, and the Knowledge Base Provider continue unchanged.

## Verification

Run from this repository:

```sh
cargo fmt --all -- --check
cargo check --locked --workspace --all-targets
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets -- -D warnings
./scripts/check-repository-boundary.sh
./scripts/check-public-packages.sh
```

The repository pins source dependencies to immutable Git revisions. Crates.io publication is intentionally deferred until the corresponding owner packages are available there.
