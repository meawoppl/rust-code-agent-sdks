//! Name-conformance test: every wire field name we use must appear in the
//! corresponding upstream struct.
//!
//! ## Why this exists
//!
//! This crate hand-translates upstream's `app-server-protocol` types. We've
//! historically invented struct names (e.g. `CommandExecutionApprovalParams`
//! where upstream has `PermissionsRequestApprovalParams`) and field names
//! (e.g. `call_id` where upstream has `item_id`). Neither `cargo build` nor
//! the typed dispatch in `messages.rs` can catch that — both compile fine,
//! and the only signal at runtime is a deserialization error against a live
//! CLI.
//!
//! This test parses a pinned snapshot of upstream's Rust source under
//! `tests/test_data/upstream/v2/`, parses our own crate source, computes the
//! wire field-name set for each side (applying `serde(rename_all)` and
//! `serde(rename)` rules), and asserts ours ⊆ upstream for every mapping in
//! [`MAPPINGS`].
//!
//! ## Refreshing the snapshot
//!
//! `tools/sync-upstream-bindings.sh [TAG]` pulls fresh upstream source files
//! at the given tag into `tests/test_data/upstream/v2/`. After bumping, run
//! `cargo test -p codex-codes --test protocol_name_conformance` to see what
//! divergence the new tag introduces.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use syn::{Attribute, Expr, Fields, Item, ItemStruct, Lit, Meta};

// ── Conformance mapping table ───────────────────────────────────────
//
// One entry per (our struct, upstream struct) pair we want to enforce. Each
// entry asserts: every wire field name our struct emits must also appear in
// the upstream struct's wire field names. Upstream is the source of truth;
// we may model a subset.
//
// Grow this table as types are aligned. New pairs are cheap to add; the
// hard work is in the mapping decision itself (which upstream type is our
// type a wrapper for?).
struct Mapping {
    /// File under `codex-codes/src/` containing our struct.
    ours_file: &'static str,
    /// Our struct name as it appears in that file.
    ours_name: &'static str,
    /// File under `tests/test_data/upstream/` containing upstream's struct
    /// (mirrors the upstream layout: e.g. `v1.rs` or `v2/item.rs`).
    upstream_file: &'static str,
    /// Upstream struct name as it appears in that file.
    upstream_name: &'static str,
}

const MAPPINGS: &[Mapping] = &[
    // The two approval-flow request methods and their responses. Both live
    // under upstream's `v2/item.rs` (NOT `v2/permissions.rs`, which models
    // a different, newer permissions endpoint that happens to share some
    // field shape).
    Mapping {
        ours_file: "protocol.rs",
        ours_name: "CommandExecutionRequestApprovalParams",
        upstream_file: "v2/item.rs",
        upstream_name: "CommandExecutionRequestApprovalParams",
    },
    Mapping {
        ours_file: "protocol.rs",
        ours_name: "CommandExecutionRequestApprovalResponse",
        upstream_file: "v2/item.rs",
        upstream_name: "CommandExecutionRequestApprovalResponse",
    },
    Mapping {
        ours_file: "protocol.rs",
        ours_name: "FileChangeRequestApprovalParams",
        upstream_file: "v2/item.rs",
        upstream_name: "FileChangeRequestApprovalParams",
    },
    Mapping {
        ours_file: "protocol.rs",
        ours_name: "FileChangeRequestApprovalResponse",
        upstream_file: "v2/item.rs",
        upstream_name: "FileChangeRequestApprovalResponse",
    },
    // Initialize handshake — lives in upstream's `v1.rs`.
    Mapping {
        ours_file: "protocol.rs",
        ours_name: "InitializeParams",
        upstream_file: "v1.rs",
        upstream_name: "InitializeParams",
    },
    Mapping {
        ours_file: "protocol.rs",
        ours_name: "InitializeResponse",
        upstream_file: "v1.rs",
        upstream_name: "InitializeResponse",
    },
    Mapping {
        ours_file: "protocol.rs",
        ours_name: "InitializeCapabilities",
        upstream_file: "v1.rs",
        upstream_name: "InitializeCapabilities",
    },
    Mapping {
        ours_file: "protocol.rs",
        ours_name: "ClientInfo",
        upstream_file: "v1.rs",
        upstream_name: "ClientInfo",
    },
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
/// each argument as a string in the form serde emits them — `rename_all = "x"`,
/// `rename = "x"`, `flatten`, `skip`, etc. — for later inspection.
fn serde_args(attrs: &[Attribute]) -> Vec<String> {
    let mut out = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        // Each `#[serde(...)]` has a list-style meta; parse the inside as
        // comma-separated `Meta` items and convert each back to a string.
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
                    // Non-string serde values (numbers, bools) aren't part of
                    // the rename/skip/flatten surface we care about; emit a
                    // placeholder so the calling matcher just won't match.
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

fn find_struct<'a>(file: &'a syn::File, name: &str) -> Option<&'a ItemStruct> {
    file.items.iter().find_map(|it| match it {
        Item::Struct(s) if s.ident == name => Some(s),
        _ => None,
    })
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

// ── The test ────────────────────────────────────────────────────────

#[test]
fn wire_field_names_are_subset_of_upstream() {
    let our_src = crate_root().join("src");
    let upstream_src = crate_root().join("tests/test_data/upstream");

    let mut failures: Vec<String> = Vec::new();

    for m in MAPPINGS {
        let our_path = our_src.join(m.ours_file);
        let upstream_path = upstream_src.join(m.upstream_file);

        let ours_file = parse_file(&our_path);
        let upstream_file = parse_file(&upstream_path);

        let Some(ours_struct) = find_struct(&ours_file, m.ours_name) else {
            failures.push(format!(
                "could not find struct `{}` in {}",
                m.ours_name,
                our_path.display()
            ));
            continue;
        };
        let Some(upstream_struct) = find_struct(&upstream_file, m.upstream_name) else {
            failures.push(format!(
                "could not find struct `{}` in {}",
                m.upstream_name,
                upstream_path.display()
            ));
            continue;
        };

        let ours_fields = wire_names_for_struct(ours_struct);
        let upstream_fields = wire_names_for_struct(upstream_struct);

        let extra: Vec<&String> = ours_fields.difference(&upstream_fields).collect();
        if !extra.is_empty() {
            failures.push(format!(
                "{} → {} (upstream {}): our struct has wire fields not in upstream: {:?}\n  \
                 ours:     {:?}\n  upstream: {:?}",
                m.ours_name, m.upstream_name, m.upstream_file, extra, ours_fields, upstream_fields
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "\n── Protocol name-conformance failures ──────────────────\n{}\n\n\
             If upstream genuinely changed shape, refresh the snapshot:\n  \
             tools/sync-upstream-bindings.sh\n",
            failures.join("\n\n")
        );
    }
}

// ── Mapping table is non-empty ──────────────────────────────────────
//
// Cheap sentinel so this file never silently becomes a no-op (e.g. if
// MAPPINGS gets emptied during a refactor).
#[test]
fn mappings_table_is_not_empty() {
    assert!(
        !MAPPINGS.is_empty(),
        "MAPPINGS table is empty — conformance test does nothing"
    );
}
