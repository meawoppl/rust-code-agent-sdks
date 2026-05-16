//! Name-conformance test: our `src/protocol/` tree must mirror upstream's
//! `app-server-protocol/src/protocol/` tree, and every wire field name we
//! use must appear in the upstream struct of the same name in the
//! same-named file.
//!
//! ## Why this exists
//!
//! This crate hand-translates upstream's `app-server-protocol` types. We've
//! historically invented struct names (e.g. `CommandExecutionApprovalParams`
//! where upstream has `CommandExecutionRequestApprovalParams`) and field
//! names (e.g. `call_id` where upstream has `item_id`). Neither `cargo
//! build` nor the typed dispatch in `messages.rs` can catch that — both
//! compile fine, and the only signal at runtime is a deserialization error
//! against a live CLI.
//!
//! ## How it works
//!
//! - Walks `src/protocol/**/*.rs` recursively.
//! - For each file with a relative path under `src/protocol/` (e.g.
//!   `v2/item.rs`), looks for the same-named file under
//!   `tests/test_data/upstream/`.
//! - Parses both with `syn`, computes each struct's wire field-name set
//!   (applying `serde(rename_all)` + `serde(rename)`, skipping
//!   `flatten`/`skip`), and asserts ours is a subset of upstream's.
//! - `mod.rs` files are skipped (re-exports / module wiring).
//! - Structs listed in [`INTENTIONALLY_LOCAL`] are skipped — those are
//!   convenience wrappers without an upstream counterpart.
//!
//! ## Refreshing the snapshot
//!
//! `tools/sync-upstream-bindings.sh [TAG]` pulls fresh upstream source files
//! at the given tag. After bumping, run `cargo test -p codex-codes --test
//! protocol_name_conformance` to see what divergence the new tag introduces.
//!
//! ## Opting a struct out
//!
//! If you add a struct under `src/protocol/` that intentionally has no
//! upstream counterpart (a local convenience wrapper), add it to
//! [`INTENTIONALLY_LOCAL`] with a one-line reason in a code comment. That
//! comment IS the justification record for future readers.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use syn::{Attribute, Expr, Fields, Item, ItemStruct, Lit, Meta};

// ── Opt-out list ────────────────────────────────────────────────────
//
// Each entry: `(relative_file, struct_name)`. Skipped by the conformance
// test. Add a comment justifying each entry — it's the durable record of
// why this name exists in our crate but not upstream.
const INTENTIONALLY_LOCAL: &[(&str, &str)] = &[
    // Local convenience wrapper around `thread/start` response — we only
    // strictly model `id` and route the rest through a flatten extras
    // field. Upstream uses its `Thread` struct (in `thread_data.rs`).
    ("v2/thread.rs", "ThreadInfo"),
    // Same: wraps upstream's `ThreadStartResponse` with an extras-bearing
    // `extra: Value` that we deliberately flatten. `flatten` is
    // incompatible with `deny_unknown_fields`, so this struct cannot be
    // strict.
    ("v2/thread.rs", "ThreadStartResponse"),
];

// ── Parsing + rename computation ────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum RenameAll {
    None,
    CamelCase,
    SnakeCase,
    KebabCase,
    ScreamingSnakeCase,
    LowerCase,
    UpperCase,
    PascalCase,
}

impl RenameAll {
    fn parse(s: &str) -> Self {
        match s {
            "camelCase" => Self::CamelCase,
            "snake_case" => Self::SnakeCase,
            "kebab-case" => Self::KebabCase,
            "SCREAMING_SNAKE_CASE" => Self::ScreamingSnakeCase,
            "lowercase" => Self::LowerCase,
            "UPPERCASE" => Self::UpperCase,
            "PascalCase" => Self::PascalCase,
            _ => Self::None,
        }
    }

    fn apply(self, rust_name: &str) -> String {
        match self {
            RenameAll::None | RenameAll::SnakeCase => rust_name.to_string(),
            RenameAll::CamelCase => to_camel_case(rust_name),
            RenameAll::KebabCase => rust_name.replace('_', "-"),
            RenameAll::ScreamingSnakeCase => rust_name.to_uppercase(),
            RenameAll::LowerCase => rust_name.to_lowercase().replace('_', ""),
            RenameAll::UpperCase => rust_name.to_uppercase().replace('_', ""),
            RenameAll::PascalCase => to_pascal_case(rust_name),
        }
    }
}

fn to_camel_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut upper_next = false;
    for (i, ch) in s.chars().enumerate() {
        if ch == '_' {
            upper_next = true;
        } else if upper_next {
            for c in ch.to_uppercase() {
                out.push(c);
            }
            upper_next = false;
        } else if i == 0 {
            for c in ch.to_lowercase() {
                out.push(c);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn to_pascal_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut upper_next = true;
    for ch in s.chars() {
        if ch == '_' {
            upper_next = true;
        } else if upper_next {
            for c in ch.to_uppercase() {
                out.push(c);
            }
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// Extract every `serde(...)` argument list from an attribute set. Returns
/// each argument as a string in the form serde emits them — `rename_all =
/// "x"`, `rename = "x"`, `flatten`, `skip`, etc. — for later inspection.
fn serde_args(attrs: &[Attribute]) -> Vec<String> {
    let mut out = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        if let Meta::List(list) = &attr.meta {
            let parser = syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated;
            if let Ok(items) = list.parse_args_with(parser) {
                for item in items {
                    out.push(meta_to_string(&item));
                }
            }
        }
    }
    out
}

fn meta_to_string(meta: &Meta) -> String {
    match meta {
        Meta::Path(p) => p
            .get_ident()
            .map(|i| i.to_string())
            .unwrap_or_else(|| quote_path(p)),
        Meta::NameValue(nv) => {
            let name = nv
                .path
                .get_ident()
                .map(|i| i.to_string())
                .unwrap_or_else(|| quote_path(&nv.path));
            let val = match &nv.value {
                Expr::Lit(el) => match &el.lit {
                    Lit::Str(s) => s.value(),
                    _ => String::new(),
                },
                _ => String::new(),
            };
            format!("{} = \"{}\"", name, val)
        }
        Meta::List(list) => list
            .path
            .get_ident()
            .map(|i| i.to_string())
            .unwrap_or_else(|| quote_path(&list.path)),
    }
}

fn quote_path(p: &syn::Path) -> String {
    p.segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn struct_rename_all(attrs: &[Attribute]) -> RenameAll {
    for a in serde_args(attrs) {
        if let Some(rest) = a.strip_prefix("rename_all = \"") {
            if let Some(value) = rest.strip_suffix('"') {
                return RenameAll::parse(value);
            }
        }
    }
    RenameAll::None
}

fn field_explicit_rename(attrs: &[Attribute]) -> Option<String> {
    for a in serde_args(attrs) {
        if let Some(rest) = a.strip_prefix("rename = \"") {
            if let Some(value) = rest.strip_suffix('"') {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn field_is_skipped_on_wire(attrs: &[Attribute]) -> bool {
    for a in serde_args(attrs) {
        // `skip` removes the field from both sides; `flatten` injects its
        // inner fields into the parent and has no wire name of its own.
        if a == "skip" || a == "flatten" {
            return true;
        }
    }
    false
}

fn wire_names_for_struct(s: &ItemStruct) -> BTreeSet<String> {
    let rename_all = struct_rename_all(&s.attrs);
    let Fields::Named(named) = &s.fields else {
        // Tuple/unit structs have no named fields and so no wire field names
        // to compare. Return empty — caller will treat that as "trivially
        // conforming."
        return BTreeSet::new();
    };
    let mut out = BTreeSet::new();
    for field in &named.named {
        if field_is_skipped_on_wire(&field.attrs) {
            continue;
        }
        let rust_name = field
            .ident
            .as_ref()
            .expect("named field has ident")
            .to_string();
        let wire_name = field_explicit_rename(&field.attrs)
            .unwrap_or_else(|| rename_all.apply(&rust_name));
        out.insert(wire_name);
    }
    out
}

fn structs_in_file(file: &syn::File) -> Vec<&ItemStruct> {
    file.items
        .iter()
        .filter_map(|it| match it {
            Item::Struct(s) => Some(s),
            _ => None,
        })
        .collect()
}

fn find_struct<'a>(file: &'a syn::File, name: &str) -> Option<&'a ItemStruct> {
    structs_in_file(file)
        .into_iter()
        .find(|s| s.ident == name)
}

fn parse_file(path: &Path) -> syn::File {
    let src = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    syn::parse_file(&src)
        .unwrap_or_else(|e| panic!("parse {} as Rust: {}", path.display(), e))
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Walk `dir` recursively, returning every `.rs` file's relative path.
fn walk_rs_files(dir: &Path, prefix: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk_rs_files(&path, prefix));
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            if let Ok(rel) = path.strip_prefix(prefix) {
                out.push(rel.to_path_buf());
            }
        }
    }
    out
}

// ── The test ────────────────────────────────────────────────────────

#[test]
fn wire_field_names_match_upstream_by_file_path() {
    let our_root = crate_root().join("src/protocol");
    let upstream_root = crate_root().join("tests/test_data/upstream");

    let mut failures: Vec<String> = Vec::new();
    let mut checked = 0u32;

    for rel in walk_rs_files(&our_root, &our_root) {
        // `mod.rs` files are pure plumbing (mod declarations, re-exports,
        // tests). They don't define wire types in our convention.
        if rel.file_name() == Some(std::ffi::OsStr::new("mod.rs")) {
            continue;
        }

        let our_path = our_root.join(&rel);
        let upstream_path = upstream_root.join(&rel);

        if !upstream_path.exists() {
            failures.push(format!(
                "our `src/protocol/{}` has no upstream counterpart at \
                 `tests/test_data/upstream/{}`. Either rename our file to \
                 match upstream's layout or extend the sync script and \
                 re-snapshot.",
                rel.display(),
                rel.display()
            ));
            continue;
        }

        let ours_file = parse_file(&our_path);
        let upstream_file = parse_file(&upstream_path);

        for ours_struct in structs_in_file(&ours_file) {
            let name = ours_struct.ident.to_string();
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if INTENTIONALLY_LOCAL.iter().any(|(f, n)| *f == rel_str && *n == name) {
                continue;
            }

            let Some(upstream_struct) = find_struct(&upstream_file, &name) else {
                failures.push(format!(
                    "{}::{} has no struct of the same name in upstream's \
                     {}. Either rename ours to match upstream, move it to \
                     the correct file, or add it to INTENTIONALLY_LOCAL \
                     with a comment.",
                    rel.display(),
                    name,
                    rel.display()
                ));
                continue;
            };

            let ours_fields = wire_names_for_struct(ours_struct);
            let upstream_fields = wire_names_for_struct(upstream_struct);

            let extra: Vec<&String> = ours_fields.difference(&upstream_fields).collect();
            if !extra.is_empty() {
                failures.push(format!(
                    "{}::{} has wire fields not in upstream: {:?}\n  \
                     ours:     {:?}\n  upstream: {:?}",
                    rel.display(),
                    name,
                    extra,
                    ours_fields,
                    upstream_fields
                ));
            }
            checked += 1;
        }
    }

    assert!(
        checked > 0,
        "name-conformance test walked zero structs — paths probably wrong"
    );

    if !failures.is_empty() {
        panic!(
            "\n── Protocol name-conformance failures ({} checked, {} failed) ──\n{}\n\n\
             If upstream genuinely changed shape, refresh the snapshot:\n  \
             tools/sync-upstream-bindings.sh\n",
            checked,
            failures.len(),
            failures.join("\n\n")
        );
    }
}
