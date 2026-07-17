//! Check the machine-readable verification inventory against the Rust sources.

use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct Inventory {
    requirement: Vec<Requirement>,
}

#[derive(Deserialize)]
struct Requirement {
    test_name: String,
}

/// Recursively collects Rust source files beneath `path`.
fn rust_files(path: &Path, output: &mut Vec<PathBuf>) {
    if path.is_dir() {
        for entry in fs::read_dir(path).expect("read Rust source directory") {
            rust_files(&entry.expect("read directory entry").path(), output);
        }
    } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
        output.push(path.to_path_buf());
    }
}

/// Adds the fully qualified names of test functions in `items` to `output`.
fn collect_test_items(
    items: &[syn::Item],
    modules: &mut Vec<String>,
    output: &mut HashSet<String>,
) {
    for item in items {
        match item {
            syn::Item::Fn(function)
                if function
                    .attrs
                    .iter()
                    .any(|attribute| attribute.path().is_ident("test")) =>
            {
                let mut path = modules.clone();
                path.push(function.sig.ident.to_string());
                output.insert(path.join("::"));
            }
            syn::Item::Mod(module) => {
                if let Some((_, items)) = &module.content {
                    modules.push(module.ident.to_string());
                    collect_test_items(items, modules, output);
                    modules.pop();
                }
            }
            _ => {}
        }
    }
}

/// Returns the test functions declared in the model crate's source and test files.
fn source_test_inventory(manifest_dir: &Path) -> HashSet<String> {
    let src_dir = manifest_dir.join("src");
    let bin_dir = src_dir.join("bin");
    let mut files = Vec::new();
    rust_files(&src_dir, &mut files);
    rust_files(&manifest_dir.join("tests"), &mut files);
    let mut tests = HashSet::new();

    for file in files {
        let source = fs::read_to_string(&file).expect("read Rust source");
        let parsed = syn::parse_file(&source).expect("parse Rust source");
        let mut modules = Vec::new();
        if file.starts_with(&src_dir)
            && !file.starts_with(&bin_dir)
            && file.file_name().and_then(|name| name.to_str()) != Some("lib.rs")
        {
            modules.push(
                file.file_stem()
                    .and_then(|stem| stem.to_str())
                    .expect("UTF-8 Rust filename")
                    .to_string(),
            );
        }
        collect_test_items(&parsed.items, &mut modules, &mut tests);
    }
    tests
}

/// Checks that every verification inventory entry refers to an existing Rust test.
///
/// Duplicate references are allowed, but produce a warning because they may be
/// accidental.
fn check_verification_plan() -> (usize, usize, usize) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repository = manifest_dir.parent().expect("model has repository parent");
    let inventory_path = repository.join("model-verification.toml");
    let inventory: Inventory =
        toml::from_str(&fs::read_to_string(&inventory_path).expect("read model-verification.toml"))
            .expect("parse model-verification.toml");

    let source_tests = source_test_inventory(&manifest_dir);
    let mut referenced_tests = HashSet::new();
    let mut duplicate_tests = HashSet::new();
    for requirement in &inventory.requirement {
        let test_name = &requirement.test_name;
        assert!(
            source_tests.contains(test_name),
            "referenced test does not exist: {test_name}"
        );
        if !referenced_tests.insert(test_name.clone()) {
            duplicate_tests.insert(test_name.clone());
        }
    }

    let mut duplicate_tests: Vec<_> = duplicate_tests.into_iter().collect();
    duplicate_tests.sort();
    for test_name in duplicate_tests {
        eprintln!("warning: verification test is referenced more than once: {test_name}");
    }

    (
        inventory.requirement.len(),
        referenced_tests.len(),
        source_tests.len(),
    )
}

/// Runs the verification inventory check and reports its coverage counts.
fn main() {
    let (requirements, referenced, total) = check_verification_plan();
    println!(
        "verification plan OK: {requirements} requirements, {referenced} referenced tests, {total} Rust tests total"
    );
}

#[cfg(test)]
mod tests {
    /// Confirms that the checked-in verification inventory references real tests.
    #[test]
    fn verification_plan_references_existing_tests() {
        super::check_verification_plan();
    }
}
