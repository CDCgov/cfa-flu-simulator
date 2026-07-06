use anyhow::bail;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> anyhow::Result<()> {
    let mut results = Vec::new();
    for path in vec!["../../model/src", "../../model/tests"] {
        let this = extract_test_docstrings_from_dir(Path::new(path))?;
        results.extend(this);
    }

    println!("{}", serde_json::to_string(&results)?);
    Ok(())
}

fn extract_test_docstrings_from_dir(dir: &Path) -> anyhow::Result<Vec<HashMap<&str, String>>> {
    let files = find_rust_files(dir)?;
    let mut out = Vec::new();
    for file in files {
        for (module, function_name, docstring) in extract_test_docstrings_from_file(&file)? {
            out.push(HashMap::from([
                ("file", file.to_string_lossy().into_owned()),
                ("module", module.unwrap_or(String::from(""))),
                ("function_name", function_name),
                ("docstring", docstring),
            ]));
        }
    }
    Ok(out)
}

#[test]
fn test_extract() {
    let results = extract_test_docstrings_from_dir(Path::new("tests")).unwrap();
    assert!(results.len() == 4);
    assert!(results[0].len() == 4);
}

fn find_rust_files(path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    find_rust_files_recursive(path, &mut files)?;
    Ok(files)
}

fn find_rust_files_recursive(path: &Path, files: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let next_path: PathBuf = entry?.path();
            find_rust_files_recursive(&next_path, files)?;
        }
    } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
        files.push(path.to_path_buf());
    }

    Ok(())
}

fn extract_test_docstrings_from_file(
    file_path: &Path,
) -> anyhow::Result<Vec<(Option<String>, String, String)>> {
    let code = fs::read_to_string(file_path)?;
    let parsed_file: syn::File = syn::parse_file(&code)?;

    let mut out = Vec::new();
    collect_test_docstrings_from_items(parsed_file.items, None, &mut out)?;
    Ok(out)
}

fn collect_test_docstrings_from_items(
    items: Vec<syn::Item>,
    module: Option<String>,
    out: &mut Vec<(Option<String>, String, String)>,
) -> anyhow::Result<()> {
    for item in items {
        if let syn::Item::Fn(item_fn) = item {
            if is_test_function(&item_fn) {
                let name = item_fn.sig.ident.to_string();
                let docstring = extract_fn_docstring(&item_fn);
                out.push((module.clone(), name, docstring));
            }
        } else if let syn::Item::Mod(item_mod) = item {
            if let Some(parent_module) = &module {
                let nested_name = item_mod.ident.to_string();
                bail!(
                    "nested module is not supported: {}::{}",
                    parent_module,
                    nested_name
                );
            }

            if let Some((_, mod_items)) = item_mod.content {
                let module = Some(item_mod.ident.to_string());
                collect_test_docstrings_from_items(mod_items, module, out)?;
            }
        }
    }

    Ok(())
}

// Helper to identify if function has #[test]
fn is_test_function(item_fn: &syn::ItemFn) -> bool {
    item_fn
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("test"))
}

// Helper to parse `///` and `#[doc = "..."]` attributes
fn extract_fn_docstring(item_fn: &syn::ItemFn) -> String {
    let mut docs = Vec::new();
    for attr in &item_fn.attrs {
        if attr.path().is_ident("doc") {
            if let syn::Meta::NameValue(name_value) = &attr.meta {
                // `/// foo` is represented as `#[doc = " foo"]`.
                if let syn::Expr::Lit(expr_lit) = &name_value.value {
                    if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                        docs.push(lit_str.value().trim().to_string());
                    }
                }
            }
        }
    }
    docs.join("\n")
}
