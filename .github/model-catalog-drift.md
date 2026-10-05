## Required follow-up: model-catalog drift

**Model catalog status: not automatically checked by this workflow.** A clean
wire-schema result does not establish that the model enum or picker is current.
Complete this reconciliation when handling Codex or Claude drift, and record
the result even when no model changes are found.

- [ ] Record the catalog source, upstream revision or CLI version, and fetch
  time. For Codex, compare `codex-rs/models-manager/models.json` and, when
  available, a fresh authenticated catalog. For Claude, inspect the installed
  CLI's model registry, including floating aliases and context selectors.
  Bundled catalogs can lag server-side releases; account-visible models do not
  establish availability for every account. Do not include credentials in reports.
- [ ] Compare against `codex-codes/src/models.rs` or
  `claude-codes/src/models.rs`. Report added identifiers, changed labels,
  visibility or alias targets, and removed identifiers separately. Record
  unavailable sources as **unchecked**, never as an empty or unchanged catalog.
- [ ] For accepted additions, update enum variants, `known()`, display labels,
  string conversions, serde round trips, and alias classification as applicable.
  Preserve unknown-string passthrough. Do not automatically delete existing
  variants or change alias semantics based on one catalog observation.
- [ ] Assess Rust API compatibility before choosing a release version: adding
  a variant to an exhaustive public enum can break downstream matches.
  `Custom(String)` does not make an enum non-exhaustive. Ask before any minor or
  major bump, per repository policy.
- [ ] Run the required formatting, clippy, tests, and types-only/WASM checks.
  Update the changelog, crate version, lockfile, and version references through
  a PR. After merge, verify that `publish-crates.yml` published the crate.
- [ ] Follow through with an agent-portal dependency/lockfile update and a
  picker/launch/schedule argument round-trip check. Record the upstream release
  and downstream PR links; distinguish merged support from verified deployment.

### Requirements for future automatic model updates

Detection must run independently of wire-schema changes, normalize catalogs
into deterministic snapshots with provenance, and distinguish additions,
metadata changes, removals, and fetch/extraction failures. Keep one update PR
per provider and avoid repeated no-change reports or version bumps.

Generate catalog/code/test updates deterministically. Automatic merging and
publishing must wait until the public enum compatibility policy is resolved
(for example, an approved migration to `#[non_exhaustive]`, or a separately
evolving data catalog). Require passing CI; leave removals, alias changes, and
ambiguous discovery results for review. Reuse the existing publish-on-main
workflow, then open the downstream agent-portal dependency update.

Runtime discovery in agent-portal, with the compiled catalog as fallback, is a
separate follow-up so new picker entries need not wait for SDK and portal
releases. Catalog presence must not be presented as proof of inference access.
