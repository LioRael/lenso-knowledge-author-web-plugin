# Plugin card: `lenso.knowledge-author.web`

## User job

An authorized teammate turns one solved problem into a reusable answer: find or create the draft, reload its exact current revision, edit it, and publish only the revision they reviewed.

## First observable behavior

`GET /knowledge` returns a focused author workspace. After entering an organization and Bearer credential, the author sees a cursor-backed draft library rather than a generic Overview or administration dashboard.

## Contract

- Plugin ID: `lenso.knowledge-author.web`
- Release: `0.1.0`
- Implementation: linked native Rust only
- Root slot: `web`
- Provides: `lenso.http.endpoint@1` descriptor `1.1.0`
- Requires exactly one: `lenso.auth@1` descriptor `1.0.0`
- Requires exactly one: `lenso.knowledge-base@1` descriptor `1.1.0`
- Configuration: none; organization is explicit per request
- State and lifecycle: stateless; no prepare or deactivate work

## Ownership and final authorization

The Plugin owns route descriptions, embedded HTML/CSS/JavaScript, strict request decoding, credential evidence selection, ActorAssertion forwarding, and intentional HTTP error representation. It owns no article, slug, draft, revision, publication, idempotency, organization, role, credential, or authorization fact.

Auth authenticates the selected credential and returns an assertion. The Web Plugin accepts only a `user` actor and attaches the assertion to the downstream context. Knowledge Base independently verifies the assertion's audience and validity, organization membership, configured caller, and `knowledge-base.articles.edit` role before applying an operation. Knowledge Base remains the final decision point.

## Failure semantics

- Missing or invalid credential: `401` plus `WWW-Authenticate: Bearer`.
- Unsupported actor kind or final Knowledge Base denial: `403`.
- Missing article: `404`.
- Slug, idempotency, or optimistic-revision conflict: `409`.
- Invalid generated request: `400`.
- Unknown Domain Error: Runtime protocol violation.
- Dependency Runtime Failure: unchanged Runtime Failure; no fabricated response or provider fallback.

The embedded page persists only the organization preference. It deliberately does not persist the Bearer credential across reloads; production Hosts should normally use their own secure session ingress policy.

## Deletion proof

Removing the `lenso.knowledge-author.web` Instance removes only `/knowledge` and `/api/knowledge/articles*`. A Plan containing the Knowledge Base Provider without this Web Instance still resolves and serves its Capability to other callers. Drafts, publications, the public Help Center, and search remain intact; no Kernel branch or hidden registration is required.

## Non-goals

- Article storage, revision calculation, publication state, or search indexing.
- Organization membership, roles, authentication, or authorization policy.
- Public article discovery or customer support workflows.
- Socket ownership, ingress limits, Host activation, or Console navigation.
