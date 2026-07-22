//! Nori-native bundler (Phase 5 scaffold → usable graph walker).
//!
//! Relative resolve, static import extraction via `nori-parser` (regex fallback),
//! recursive [`ModuleGraph`] construction, concatenated ESM emit, and a minimal
//! Module Federation `create_remote_entry` helper.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use nori_allocator::Allocator;
use nori_ast::Program;
use nori_diagnostic::NoriError;
use nori_lexer::lex;
use nori_parser::parse_in;
use thiserror::Error;

/// A module in the dependency graph.
#[derive(Debug, Clone)]
pub struct Module {
    pub id: ModuleId,
    pub path: PathBuf,
    pub source: String,
    pub dependencies: Vec<ModuleId>,
    /// Specifiers as written in the source (parallel to `dependencies` where resolved).
    pub import_specifiers: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(pub u32);

/// Directed module graph keyed by [`ModuleId`].
#[derive(Debug, Default, Clone)]
pub struct ModuleGraph {
    pub modules: BTreeMap<ModuleId, Module>,
    pub path_to_id: BTreeMap<PathBuf, ModuleId>,
    pub entry: Option<ModuleId>,
}

impl ModuleGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, id: ModuleId) -> Option<&Module> {
        self.modules.get(&id)
    }

    pub fn get_by_path(&self, path: &Path) -> Option<&Module> {
        self.path_to_id.get(path).and_then(|id| self.modules.get(id))
    }

    /// Topological order from entry (dependencies before dependents). Cycles are
    /// broken by emitting the first-seen node order from BFS.
    pub fn topological_ids(&self) -> Vec<ModuleId> {
        let Some(entry) = self.entry else {
            return self.modules.keys().copied().collect();
        };

        let mut visiting = BTreeSet::new();
        let mut done = BTreeSet::new();
        let mut order = Vec::new();

        fn dfs(
            graph: &ModuleGraph,
            id: ModuleId,
            visiting: &mut BTreeSet<ModuleId>,
            done: &mut BTreeSet<ModuleId>,
            order: &mut Vec<ModuleId>,
        ) {
            if done.contains(&id) || visiting.contains(&id) {
                return;
            }
            visiting.insert(id);
            if let Some(module) = graph.get(id) {
                for dep in &module.dependencies {
                    dfs(graph, *dep, visiting, done, order);
                }
            }
            visiting.remove(&id);
            done.insert(id);
            order.push(id);
        }

        dfs(self, entry, &mut visiting, &mut done, &mut order);
        // Include any unreachable modules last.
        for id in self.modules.keys().copied() {
            if !done.contains(&id) {
                order.push(id);
            }
        }
        order
    }
}

/// Result of a bundle operation.
#[derive(Debug, Clone)]
pub struct BundleResult {
    /// Concatenated ESM (or multi-file listing when `multi_file` is used).
    pub code: String,
    /// Ordered module graph listing (ESM import edges).
    pub graph: Vec<GraphListing>,
    /// Full module graph.
    pub module_graph: ModuleGraph,
}

#[derive(Debug, Clone)]
pub struct GraphListing {
    pub path: PathBuf,
    pub imports: Vec<String>,
}

/// Bundle emit mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmitMode {
    /// Single concatenated ESM string with module banners.
    #[default]
    Concatenated,
    /// Multi-file style emit: each module wrapped with a path comment separator.
    MultiFile,
}

#[derive(Debug, Clone)]
pub struct BundleOptions {
    pub emit: EmitMode,
}

impl Default for BundleOptions {
    fn default() -> Self {
        Self {
            emit: EmitMode::Concatenated,
        }
    }
}

/// Module Federation remote entry contract (runtime shape).
#[derive(Debug, Clone)]
pub struct RemoteEntry {
    /// Initialize the shared scope (called by host before `get`).
    pub init: SharedScopeInit,
    /// Load an exposed module factory by id (e.g. `"./App"`).
    pub get: RemoteGet,
}

pub type SharedScopeInit = fn(&mut SharedScope) -> Result<(), BundleError>;
pub type RemoteGet = fn(&str) -> Result<ModuleFactory, BundleError>;
pub type ModuleFactory = Box<dyn Fn() -> String>;

/// Shared dependency scope for Module Federation.
#[derive(Debug, Default, Clone)]
pub struct SharedScope {
    pub versions: BTreeMap<String, String>,
    pub loaded: BTreeSet<String>,
}

impl SharedScope {
    pub fn share(&mut self, name: impl Into<String>, version: impl Into<String>) {
        self.versions.insert(name.into(), version.into());
    }

    pub fn has(&self, name: &str) -> bool {
        self.versions.contains_key(name) || self.loaded.contains(name)
    }
}

/// Map of exposed module id → factory source (or prebundled code).
pub type ExposeMap = BTreeMap<String, String>;

/// Build a minimal MF-compatible remote entry from an expose map.
///
/// `init` records shared package names into the scope; `get` returns a factory
/// that yields the exposed module source string.
pub fn create_remote_entry(exposes: ExposeMap) -> RemoteEntry {
    // Function pointers cannot capture the map; store via once_cell-like static
    // is awkward in lib code. Instead we return closures boxed through a
    // thin adapter that looks up from a leaked map for the scaffold lifetime.
    let leaked: &'static ExposeMap = Box::leak(Box::new(exposes));

    fn init_impl(scope: &mut SharedScope) -> Result<(), BundleError> {
        if !scope.has("@nori/core") {
            scope.share("@nori/core", "*");
        }
        Ok(())
    }

    // `get` needs the leaked map — use a thread-local / static registry keyed by
    // pointer. For the scaffold, register on creation.
    REMOTE_EXPOSES.with(|cell| {
        *cell.borrow_mut() = Some(leaked);
    });

    RemoteEntry {
        init: init_impl,
        get: get_impl,
    }
}

thread_local! {
    static REMOTE_EXPOSES: std::cell::RefCell<Option<&'static ExposeMap>> =
        const { std::cell::RefCell::new(None) };
}

fn get_impl(id: &str) -> Result<ModuleFactory, BundleError> {
    let source = REMOTE_EXPOSES.with(|cell| {
        cell.borrow()
            .and_then(|map| map.get(id).cloned())
            .ok_or_else(|| BundleError::NotFound(id.to_string()))
    })?;
    Ok(Box::new(move || source.clone()))
}

#[derive(Debug, Error)]
pub enum BundleError {
    #[error(transparent)]
    Nori(#[from] NoriError),
    #[error("module not found: {0}")]
    NotFound(String),
    #[error("resolve failed for `{specifier}` from `{from}`")]
    Resolve { specifier: String, from: String },
    #[error("{0}")]
    Message(String),
}

const RELATIVE_EXTENSIONS: &[&str] = &[".nori", ".js", ".ts", ".mjs", ".tsx", ".jsx"];

/// Node-style relative resolve.
///
/// Handles `./` and `../` against `from`, trying extensions and `/index.*`.
/// Bare package names are returned unchanged (no `node_modules` walk yet).
pub fn resolve(specifier: &str, from: &Path) -> Result<PathBuf, BundleError> {
    if !(specifier.starts_with("./") || specifier.starts_with("../")) {
        return Ok(PathBuf::from(specifier));
    }

    let base = from.parent().unwrap_or_else(|| Path::new("."));
    let candidate = base.join(specifier);

    if let Some(path) = try_file(&candidate) {
        return Ok(canonicalize_best_effort(path));
    }

    // Directory index
    if candidate.is_dir() {
        for ext in RELATIVE_EXTENSIONS {
            let index = candidate.join(format!("index{ext}"));
            if index.is_file() {
                return Ok(canonicalize_best_effort(index));
            }
        }
    }

    Err(BundleError::Resolve {
        specifier: specifier.to_string(),
        from: from.display().to_string(),
    })
}

fn try_file(candidate: &Path) -> Option<PathBuf> {
    if candidate.is_file() {
        return Some(candidate.to_path_buf());
    }
    if candidate.extension().is_some() {
        return None;
    }
    for ext in RELATIVE_EXTENSIONS {
        let with_ext = PathBuf::from(format!("{}{ext}", candidate.display()));
        if with_ext.is_file() {
            return Some(with_ext);
        }
    }
    None
}

fn canonicalize_best_effort(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

/// Bundle an entry file with default options.
pub fn bundle(entry: impl AsRef<Path>) -> Result<BundleResult, BundleError> {
    bundle_with_options(entry, BundleOptions::default())
}

/// Bundle an entry file, walking the relative import graph.
pub fn bundle_with_options(
    entry: impl AsRef<Path>,
    options: BundleOptions,
) -> Result<BundleResult, BundleError> {
    let entry = entry.as_ref();
    let entry = if entry.exists() {
        canonicalize_best_effort(entry.to_path_buf())
    } else {
        return Err(BundleError::NotFound(entry.display().to_string()));
    };

    let mut graph = ModuleGraph::new();
    let mut queue = VecDeque::new();
    queue.push_back(entry.clone());

    while let Some(path) = queue.pop_front() {
        if graph.path_to_id.contains_key(&path) {
            continue;
        }

        let source = std::fs::read_to_string(&path).map_err(|err| {
            BundleError::Message(format!("failed to read {}: {err}", path.display()))
        })?;
        let imports = collect_imports(&source, &path)?;

        // Reserve id first so cycles can reference it.
        let id = ModuleId(graph.modules.len() as u32);
        graph.path_to_id.insert(path.clone(), id);
        graph.modules.insert(
            id,
            Module {
                id,
                path: path.clone(),
                source,
                dependencies: Vec::new(),
                import_specifiers: imports.clone(),
            },
        );

        for import in &imports {
            if !(import.starts_with("./") || import.starts_with("../")) {
                continue;
            }
            if let Ok(resolved) = resolve(import, &path) {
                if resolved.exists() {
                    let resolved = canonicalize_best_effort(resolved);
                    if !graph.path_to_id.contains_key(&resolved) {
                        queue.push_back(resolved);
                    }
                }
            }
        }
    }

    // Second pass: rebuild dependency ids from import_specifiers now that all
    // modules are registered.
    let paths: Vec<(ModuleId, PathBuf, Vec<String>)> = graph
        .modules
        .values()
        .map(|m| (m.id, m.path.clone(), m.import_specifiers.clone()))
        .collect();

    for (id, path, imports) in paths {
        let mut dep_ids = Vec::new();
        for import in &imports {
            if !(import.starts_with("./") || import.starts_with("../")) {
                continue;
            }
            if let Ok(resolved) = resolve(import, &path) {
                let resolved = canonicalize_best_effort(resolved);
                if let Some(dep_id) = graph.path_to_id.get(&resolved) {
                    dep_ids.push(*dep_id);
                }
            }
        }
        if let Some(module) = graph.modules.get_mut(&id) {
            module.dependencies = dep_ids;
        }
    }

    let entry_id = *graph
        .path_to_id
        .get(&entry)
        .ok_or_else(|| BundleError::NotFound(entry.display().to_string()))?;
    graph.entry = Some(entry_id);

    let order = graph.topological_ids();
    let listings: Vec<GraphListing> = order
        .iter()
        .filter_map(|id| graph.get(*id))
        .map(|m| GraphListing {
            path: m.path.clone(),
            imports: m.import_specifiers.clone(),
        })
        .collect();

    let code = emit_bundle(&graph, &order, options.emit);

    Ok(BundleResult {
        code,
        graph: listings,
        module_graph: graph,
    })
}

fn emit_bundle(graph: &ModuleGraph, order: &[ModuleId], mode: EmitMode) -> String {
    let mut code = String::new();
    let entry_path = graph
        .entry
        .and_then(|id| graph.get(id))
        .map(|m| m.path.display().to_string())
        .unwrap_or_else(|| "<unknown>".into());

    code.push_str(&format!("// nori-bundler — entry: {entry_path}\n"));
    code.push_str(&format!("// modules: {}\n", order.len()));

    match mode {
        EmitMode::Concatenated => {
            code.push_str("// emit: concatenated\n\n");
            for id in order {
                let Some(module) = graph.get(*id) else {
                    continue;
                };
                code.push_str(&format!("// ----- module: {} -----\n", module.path.display()));
                code.push_str(module.source.trim_end());
                code.push_str("\n\n");
            }
        }
        EmitMode::MultiFile => {
            code.push_str("// emit: multi-file\n\n");
            for id in order {
                let Some(module) = graph.get(*id) else {
                    continue;
                };
                code.push_str(&format!(
                    "// ===== FILE: {} =====\n",
                    module.path.display()
                ));
                code.push_str(module.source.trim_end());
                code.push_str("\n// ===== END FILE =====\n\n");
            }
        }
    }

    code
}

/// Collect static `import` / `export ... from` sources.
///
/// Prefers `nori-parser`; falls back to a line-oriented regex if parse fails
/// (e.g. incomplete syntax in a dependency).
pub fn collect_imports(source: &str, path: &Path) -> Result<Vec<String>, BundleError> {
    match collect_imports_parsed(source, path) {
        Ok(imports) => Ok(imports),
        Err(_) => Ok(collect_imports_regex(source)),
    }
}

fn collect_imports_parsed(source: &str, path: &Path) -> Result<Vec<String>, BundleError> {
    let allocator = Allocator::new();
    let tokens = lex(source)?;
    let filename = path.display().to_string();
    let program: Program<'_> = parse_in(&allocator, source, filename, tokens)?;

    let mut imports = Vec::new();
    for stmt in &program.body {
        match stmt {
            nori_ast::Stmt::Import(import) => {
                imports.push(strip_quotes(import.source.as_str()));
            }
            nori_ast::Stmt::Export(export) => {
                if let Some(source) = export_source(export) {
                    imports.push(source);
                }
            }
            _ => {}
        }
    }
    Ok(imports)
}

fn export_source(export: &nori_ast::ExportDecl<'_>) -> Option<String> {
    use nori_ast::ExportDecl;
    match export {
        ExportDecl::Named { source: Some(source), .. } => Some(strip_quotes(source.as_str())),
        ExportDecl::All { source, .. } => Some(strip_quotes(source.as_str())),
        _ => None,
    }
}

fn strip_quotes(value: &str) -> String {
    value
        .trim()
        .trim_matches(|c| c == '"' || c == '\'' || c == '`')
        .to_string()
}

fn collect_imports_regex(source: &str) -> Vec<String> {
    let mut imports = Vec::new();
    // import ... from "x"  |  import "x"  |  export ... from "x"
    let patterns = [
        regex_lite_from(r#"(?:import|export)\s+(?:[^'";]+?\s+from\s+)?['"]([^'"]+)['"]"#),
    ];
    for (line_idx, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        let _ = line_idx;
        for pat in &patterns {
            if let Some(spec) = pat(trimmed) {
                imports.push(spec);
            }
        }
    }
    imports
}

/// Tiny matcher so we avoid adding the `regex` crate; good enough for import lines.
fn regex_lite_from(
    _pattern: &str,
) -> impl Fn(&str) -> Option<String> {
    |line: &str| {
        let lower = line;
        let is_import = lower.starts_with("import ");
        let is_export_from = lower.starts_with("export ") && lower.contains(" from ");
        if !is_import && !is_export_from {
            return None;
        }
        // Find last quoted string on the line (the module specifier).
        let bytes = line.as_bytes();
        let mut end = None;
        let mut quote = b'"';
        for i in (0..bytes.len()).rev() {
            if bytes[i] == b'"' || bytes[i] == b'\'' {
                end = Some(i);
                quote = bytes[i];
                break;
            }
        }
        let end = end?;
        let mut start = None;
        for i in (0..end).rev() {
            if bytes[i] == quote {
                start = Some(i + 1);
                break;
            }
        }
        let start = start?;
        let spec = &line[start..end];
        if spec.is_empty() {
            return None;
        }
        Some(spec.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nori-bundler-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn resolve_relative_with_extension() {
        let dir = temp_dir("resolve");
        let entry = dir.join("entry.js");
        let dep = dir.join("dep.nori");
        std::fs::write(&dep, "export default function A() {}").unwrap();
        std::fs::write(&entry, "import A from './dep.nori';\n").unwrap();

        let resolved = resolve("./dep.nori", &entry).unwrap();
        assert_eq!(
            canonicalize_best_effort(resolved),
            canonicalize_best_effort(dep)
        );
    }

    #[test]
    fn resolve_extensionless_and_index() {
        let dir = temp_dir("resolve-index");
        let entry = dir.join("entry.js");
        let nested = dir.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        let index = nested.join("index.ts");
        std::fs::write(&index, "export const x = 1;\n").unwrap();
        std::fs::write(&entry, "import { x } from './nested';\n").unwrap();

        let resolved = resolve("./nested", &entry).unwrap();
        assert_eq!(
            canonicalize_best_effort(resolved),
            canonicalize_best_effort(index)
        );
    }

    #[test]
    fn bundle_walks_relative_graph_and_concatenates() {
        let dir = temp_dir("bundle-graph");
        let dep = dir.join("dep.js");
        let entry = dir.join("entry.js");
        std::fs::write(&dep, "export const answer = 42;\n").unwrap();
        let mut file = std::fs::File::create(&entry).unwrap();
        writeln!(file, "import {{ answer }} from './dep.js';").unwrap();
        writeln!(file, "export default answer;").unwrap();

        let result = bundle(&entry).unwrap();
        assert_eq!(result.module_graph.modules.len(), 2);
        assert!(result.code.contains("answer = 42"));
        assert!(result.code.contains("export default answer"));
        assert!(
            result
                .graph
                .iter()
                .any(|g| g.imports.iter().any(|i| i.contains("dep")))
        );
    }

    #[test]
    fn bundle_multi_file_emit() {
        let dir = temp_dir("bundle-multi");
        let entry = dir.join("entry.nori");
        std::fs::write(&entry, "export default function App() { return null; }\n").unwrap();

        let result = bundle_with_options(
            &entry,
            BundleOptions {
                emit: EmitMode::MultiFile,
            },
        )
        .unwrap();
        assert!(result.code.contains("===== FILE:"));
        assert!(result.code.contains("emit: multi-file"));
    }

    #[test]
    fn collect_imports_regex_fallback() {
        let imports = collect_imports_regex(
            "import A from \"./a.js\";\nexport { B } from '../b.ts';\nimport \"./side.js\";\n",
        );
        assert!(imports.contains(&"./a.js".to_string()));
        assert!(imports.contains(&"../b.ts".to_string()));
        assert!(imports.contains(&"./side.js".to_string()));
    }

    #[test]
    fn shared_scope_and_remote_entry() {
        let mut exposes = ExposeMap::new();
        exposes.insert("./App".into(), "export default 1".into());
        let remote = create_remote_entry(exposes);

        let mut scope = SharedScope::default();
        (remote.init)(&mut scope).unwrap();
        assert!(scope.has("@nori/core"));

        let factory = (remote.get)("./App").unwrap();
        assert!(factory().contains("export default 1"));
        assert!((remote.get)("./Missing").is_err());
    }
}
