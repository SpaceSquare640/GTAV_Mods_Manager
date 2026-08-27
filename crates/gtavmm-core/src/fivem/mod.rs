// SPDX-License-Identifier: AGPL-3.0-only

//! FiveM support: server `resources\` dependency-order resolution (this module) and a
//! client-side asset-mod provider (`providers::FiveMClientProvider`).
//!
//! **The `resources\` dependency graph is this project's concrete differentiator**
//! versus txAdmin, which only offers a manual `ensure`-order CFG editor (see the
//! project's competitive-analysis notes) — this resolves a correct load order
//! automatically from each resource's declared dependencies.
//!
//! `fxmanifest.lua`'s `dependency 'name'` / `dependencies {'a', 'b'}` syntax is
//! official, documented FiveM syntax (confirmed before this module was written, not
//! assumed) and is what this module parses. Only bare quoted resource-name literals
//! are handled — Lua string concatenation, variables, or computed dependency names
//! are not evaluated (no FiveM manifest in the wild is known to need that, but if one
//! does, it will silently not be picked up as a dependency rather than error).

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashSet};
use std::path::{Path, PathBuf};

use crate::error::{CoreError, CoreResult};

/// One discovered resource folder: its name (the folder name, which is also FiveM's
/// resource identifier), its path, and the dependency names its manifest declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceInfo {
    pub name: String,
    pub path: PathBuf,
    pub dependencies: Vec<String>,
}

const MANIFEST_FILE_NAMES: &[&str] = &["fxmanifest.lua", "__resource.lua"];

fn find_manifest(resource_dir: &Path) -> Option<PathBuf> {
    MANIFEST_FILE_NAMES
        .iter()
        .map(|name| resource_dir.join(name))
        .find(|p| p.is_file())
}

/// Extracts every single-quoted or double-quoted string literal in `s`, in order of
/// appearance. Used both for `dependency '...'` (one literal) and `dependencies {...}`
/// (many literals inside the braces).
fn extract_quoted_strings(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = s.char_indices().peekable();
    while let Some((_, c)) = chars.next() {
        if c == '\'' || c == '"' {
            let quote = c;
            let mut literal = String::new();
            for (_, c2) in chars.by_ref() {
                if c2 == quote {
                    break;
                }
                literal.push(c2);
            }
            out.push(literal);
        }
    }
    out
}

/// Parses the dependency names declared by a `fxmanifest.lua`/`__resource.lua`'s
/// `dependency 'name'` and `dependencies {'a', 'b'}` directives. Does not strip Lua
/// comments (`--`) first — a commented-out dependency line would still be picked up.
/// This is a known, documented simplification: real manifests essentially never
/// comment out an active dependency declaration, and a false-positive dependency is
/// far safer than a silently-dropped real one (worst case, the resolver treats an
/// already-satisfied ordering as still valid).
fn parse_dependencies(contents: &str) -> Vec<String> {
    let mut deps = Vec::new();

    for (idx, _) in contents.match_indices("dependencies") {
        let rest = &contents[idx + "dependencies".len()..];
        if let Some(open) = rest.find('{') {
            if let Some(close_rel) = rest[open..].find('}') {
                let block = &rest[open + 1..open + close_rel];
                deps.extend(extract_quoted_strings(block));
            }
        }
    }

    // `dependency` is a distinct keyword from `dependencies` (different spelling, not
    // a prefix match), so this loop's matches never double-count the block above.
    for (idx, _) in contents.match_indices("dependency") {
        let rest = &contents[idx + "dependency".len()..];
        let line_end = rest.find('\n').unwrap_or(rest.len());
        let line = &rest[..line_end];
        if let Some(first) = extract_quoted_strings(line).into_iter().next() {
            deps.push(first);
        }
    }

    deps
}

/// Recursively finds every resource folder (any directory containing a manifest file)
/// under `resources_root`, without descending into an already-found resource's own
/// subfolders (a resource's internal `html\`/`stream\`/etc. subfolders are never
/// themselves separate resources).
pub fn discover_resources(resources_root: &Path) -> CoreResult<Vec<ResourceInfo>> {
    let mut found = Vec::new();
    let mut it = walkdir::WalkDir::new(resources_root)
        .min_depth(1)
        .into_iter();

    while let Some(entry) = it.next() {
        let entry = entry.map_err(|e| {
            CoreError::Io(std::io::Error::other(format!(
                "walking {}: {e}",
                resources_root.display()
            )))
        })?;
        if !entry.file_type().is_dir() {
            continue;
        }
        if let Some(manifest_path) = find_manifest(entry.path()) {
            let contents = std::fs::read_to_string(&manifest_path)?;
            found.push(ResourceInfo {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: entry.path().to_path_buf(),
                dependencies: parse_dependencies(&contents),
            });
            it.skip_current_dir();
        }
    }

    Ok(found)
}

/// Resolves a correct load order for every resource under `resources_root`, ordering
/// each resource after every dependency it declares that is *also present locally*
/// (via Kahn's algorithm — a min-heap over ready-to-load resource names keeps the
/// result deterministic rather than depending on filesystem iteration order).
///
/// A dependency name that doesn't match any discovered local resource is treated as
/// already satisfied rather than an error — many manifests depend on resources
/// shipped with the server itself (e.g. `chat`, `spawnmanager`) that this function
/// has no way to see from a `resources\` folder alone.
///
/// Returns `CoreError::DependencyGraph` if a genuine cycle exists among the resources
/// that *were* discovered (impossible to produce any valid order).
pub fn resolve_load_order(resources_root: &Path) -> CoreResult<Vec<String>> {
    let resources = discover_resources(resources_root)?;
    let known: HashSet<&str> = resources.iter().map(|r| r.name.as_str()).collect();

    let mut in_degree: BTreeMap<String, usize> = BTreeMap::new();
    let mut dependents: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for r in &resources {
        in_degree.entry(r.name.clone()).or_insert(0);
    }
    for r in &resources {
        for dep in &r.dependencies {
            if !known.contains(dep.as_str()) {
                continue;
            }
            *in_degree.entry(r.name.clone()).or_insert(0) += 1;
            dependents
                .entry(dep.clone())
                .or_default()
                .push(r.name.clone());
        }
    }

    let mut ready: BinaryHeap<Reverse<String>> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(name, _)| Reverse(name.clone()))
        .collect();

    let mut order = Vec::new();
    while let Some(Reverse(name)) = ready.pop() {
        order.push(name.clone());
        if let Some(deps) = dependents.get(&name) {
            for d in deps {
                let deg = in_degree
                    .get_mut(d)
                    .expect("dependent tracked in in_degree");
                *deg -= 1;
                if *deg == 0 {
                    ready.push(Reverse(d.clone()));
                }
            }
        }
    }

    if order.len() != resources.len() {
        let stuck: Vec<&str> = resources
            .iter()
            .map(|r| r.name.as_str())
            .filter(|n| !order.iter().any(|o| o == n))
            .collect();
        return Err(CoreError::DependencyGraph {
            reason: format!("circular dependency detected among: {}", stuck.join(", ")),
        });
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_manifest(dir: &Path, name: &str, body: &str) {
        let resource_dir = dir.join(name);
        std::fs::create_dir_all(&resource_dir).unwrap();
        std::fs::write(resource_dir.join("fxmanifest.lua"), body).unwrap();
    }

    #[test]
    fn parses_single_dependency_directive() {
        let deps = parse_dependencies("fx_version 'cerulean'\ndependency 'core-lib'\n");
        assert_eq!(deps, vec!["core-lib".to_string()]);
    }

    #[test]
    fn parses_dependencies_block() {
        let deps = parse_dependencies("dependencies {\n    'a',\n    'b',\n    'c'\n}\n");
        assert_eq!(
            deps,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn resolves_load_order_respecting_dependencies() {
        let dir = tempfile::tempdir().unwrap();
        write_manifest(dir.path(), "core-lib", "fx_version 'cerulean'\n");
        write_manifest(dir.path(), "framework", "dependency 'core-lib'\n");
        write_manifest(
            dir.path(),
            "job-script",
            "dependencies {'framework', 'core-lib'}\n",
        );

        let order = resolve_load_order(dir.path()).unwrap();
        let pos = |name: &str| order.iter().position(|n| n == name).unwrap();
        assert!(pos("core-lib") < pos("framework"));
        assert!(pos("framework") < pos("job-script"));
    }

    #[test]
    fn missing_dependency_is_treated_as_already_satisfied_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        write_manifest(dir.path(), "job-script", "dependency 'chat'\n");
        let order = resolve_load_order(dir.path()).unwrap();
        assert_eq!(order, vec!["job-script".to_string()]);
    }

    #[test]
    fn circular_dependency_is_reported_as_an_error() {
        let dir = tempfile::tempdir().unwrap();
        write_manifest(dir.path(), "a", "dependency 'b'\n");
        write_manifest(dir.path(), "b", "dependency 'a'\n");
        assert!(matches!(
            resolve_load_order(dir.path()),
            Err(CoreError::DependencyGraph { .. })
        ));
    }

    #[test]
    fn does_not_descend_into_a_resource_own_subfolders() {
        let dir = tempfile::tempdir().unwrap();
        let resource_dir = dir.path().join("my-resource");
        std::fs::create_dir_all(resource_dir.join("html")).unwrap();
        std::fs::write(
            resource_dir.join("fxmanifest.lua"),
            "fx_version 'cerulean'\n",
        )
        .unwrap();
        // A stray manifest-named file inside the resource's own subfolder must not be
        // picked up as a second, separate resource.
        std::fs::write(
            resource_dir.join("html").join("fxmanifest.lua"),
            "fx_version 'cerulean'\n",
        )
        .unwrap();

        let resources = discover_resources(dir.path()).unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].name, "my-resource");
    }
}
