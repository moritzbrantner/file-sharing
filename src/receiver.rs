use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use anyhow::{Result, bail};

use crate::manifest::{MANIFEST_SCHEMA_VERSION, ManifestEntry, RootKind, ShareManifest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivePlan {
    pub root_path: PathBuf,
    pub entries: Vec<ReceivePlanEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceivePlanEntry {
    Directory {
        manifest_path: String,
        destination_path: PathBuf,
    },
    File {
        manifest_path: String,
        destination_path: PathBuf,
        size_bytes: u64,
        sha256: String,
    },
}

pub fn build_receive_plan(
    manifest: &ShareManifest,
    destination_parent: impl AsRef<Path>,
) -> Result<ReceivePlan> {
    validate_manifest_for_receive(manifest)?;

    let root_path = destination_parent.as_ref().join(&manifest.root_name);
    let entries = match manifest.root_kind {
        RootKind::File => {
            let ManifestEntry::File {
                path,
                size_bytes,
                sha256,
            } = &manifest.entries[0]
            else {
                unreachable!("validated file manifest must contain exactly one file entry");
            };

            vec![ReceivePlanEntry::File {
                manifest_path: path.clone(),
                destination_path: root_path.clone(),
                size_bytes: *size_bytes,
                sha256: sha256.clone(),
            }]
        }
        RootKind::Directory => manifest
            .entries
            .iter()
            .map(|entry| match entry {
                ManifestEntry::Directory { path } => ReceivePlanEntry::Directory {
                    manifest_path: path.clone(),
                    destination_path: append_portable_path(&root_path, path),
                },
                ManifestEntry::File {
                    path,
                    size_bytes,
                    sha256,
                } => ReceivePlanEntry::File {
                    manifest_path: path.clone(),
                    destination_path: append_portable_path(&root_path, path),
                    size_bytes: *size_bytes,
                    sha256: sha256.clone(),
                },
            })
            .collect(),
    };

    Ok(ReceivePlan { root_path, entries })
}

pub fn validate_manifest_for_receive(manifest: &ShareManifest) -> Result<()> {
    if manifest.schema_version != MANIFEST_SCHEMA_VERSION {
        bail!(
            "unsupported manifest schema version {}; expected {}",
            manifest.schema_version,
            MANIFEST_SCHEMA_VERSION
        );
    }

    validate_component(&manifest.root_name, "root name")?;

    match manifest.root_kind {
        RootKind::File => validate_file_root(manifest)?,
        RootKind::Directory => validate_directory_root(manifest)?,
    }

    Ok(())
}

fn validate_file_root(manifest: &ShareManifest) -> Result<()> {
    if manifest.entries.len() != 1 {
        bail!("file manifest must contain exactly one file entry");
    }

    match &manifest.entries[0] {
        ManifestEntry::File { path, sha256, .. } => {
            validate_component(path, "file root entry")?;
            if path != &manifest.root_name {
                bail!("file root entry must match root name");
            }
            validate_sha256(sha256)?;
        }
        ManifestEntry::Directory { .. } => {
            bail!("file manifest cannot contain a directory entry");
        }
    }

    Ok(())
}

fn validate_directory_root(manifest: &ShareManifest) -> Result<()> {
    let mut seen = BTreeSet::new();
    let mut kinds = BTreeMap::<String, bool>::new();

    for entry in &manifest.entries {
        let (path, is_directory) = match entry {
            ManifestEntry::Directory { path } => (path.as_str(), true),
            ManifestEntry::File { path, sha256, .. } => {
                validate_sha256(sha256)?;
                (path.as_str(), false)
            }
        };

        let components = validate_portable_path(path)?;

        if !seen.insert(path.to_owned()) {
            bail!("manifest contains duplicate path: {path}");
        }

        let mut prefix = String::new();
        for component in components.iter().take(components.len().saturating_sub(1)) {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(component);

            match kinds.get(&prefix) {
                Some(true) => {}
                Some(false) => bail!("manifest path descends through a file: {prefix}"),
                None => bail!("manifest path is missing parent directory entry: {prefix}"),
            }
        }

        kinds.insert(path.to_owned(), is_directory);
    }

    Ok(())
}

fn validate_portable_path(path: &str) -> Result<Vec<&str>> {
    if path.is_empty() {
        bail!("manifest path cannot be empty");
    }

    if path.starts_with('/') || path.ends_with('/') {
        bail!("manifest path must be relative and normalized: {path}");
    }

    let components = path.split('/').collect::<Vec<_>>();
    for component in &components {
        validate_component(component, "manifest path component")?;
    }

    Ok(components)
}

fn validate_component(component: &str, label: &str) -> Result<()> {
    if component.is_empty() || component == "." || component == ".." {
        bail!("{label} is not a safe portable path component: {component:?}");
    }

    if component
        .chars()
        .any(|character| matches!(character, '/' | '\\' | '\0' | ':'))
    {
        bail!("{label} contains a forbidden path character: {component:?}");
    }

    Ok(())
}

fn validate_sha256(sha256: &str) -> Result<()> {
    if sha256.len() != 64
        || !sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("file entry has an invalid lowercase SHA-256 digest");
    }

    Ok(())
}

fn append_portable_path(base: &Path, path: &str) -> PathBuf {
    let mut destination = base.to_path_buf();
    for component in path.split('/') {
        destination.push(component);
    }
    destination
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::manifest::{ManifestEntry, RootKind, ShareManifest};

    use super::{ReceivePlanEntry, build_receive_plan, validate_manifest_for_receive};

    const EMPTY_SHA256: &str =
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    fn directory_manifest(entries: Vec<ManifestEntry>) -> ShareManifest {
        ShareManifest {
            schema_version: 1,
            root_name: "shared".into(),
            root_kind: RootKind::Directory,
            entries,
        }
    }

    #[test]
    fn receive_plan_keeps_every_entry_under_the_selected_root() {
        let manifest = directory_manifest(vec![
            ManifestEntry::Directory {
                path: "docs".into(),
            },
            ManifestEntry::File {
                path: "docs/readme.txt".into(),
                size_bytes: 0,
                sha256: EMPTY_SHA256.into(),
            },
        ]);

        let plan = build_receive_plan(&manifest, Path::new("/receive"))
            .expect("valid manifest should produce a plan");

        assert_eq!(plan.root_path, Path::new("/receive").join("shared"));
        assert_eq!(
            plan.entries[1],
            ReceivePlanEntry::File {
                manifest_path: "docs/readme.txt".into(),
                destination_path: Path::new("/receive")
                    .join("shared")
                    .join("docs")
                    .join("readme.txt"),
                size_bytes: 0,
                sha256: EMPTY_SHA256.into(),
            }
        );
    }

    #[test]
    fn path_traversal_is_rejected() {
        let manifest = directory_manifest(vec![ManifestEntry::File {
            path: "../escape.txt".into(),
            size_bytes: 0,
            sha256: EMPTY_SHA256.into(),
        }]);

        let error =
            validate_manifest_for_receive(&manifest).expect_err("traversal must be rejected");

        assert!(error.to_string().contains("safe portable path component"));
    }

    #[test]
    fn windows_style_traversal_is_rejected_on_every_platform() {
        let manifest = directory_manifest(vec![ManifestEntry::File {
            path: "..\\escape.txt".into(),
            size_bytes: 0,
            sha256: EMPTY_SHA256.into(),
        }]);

        let error = validate_manifest_for_receive(&manifest)
            .expect_err("backslash traversal must be rejected");

        assert!(error.to_string().contains("forbidden path character"));
    }

    #[test]
    fn duplicate_paths_are_rejected() {
        let manifest = directory_manifest(vec![
            ManifestEntry::Directory { path: "docs".into() },
            ManifestEntry::Directory { path: "docs".into() },
        ]);

        let error =
            validate_manifest_for_receive(&manifest).expect_err("duplicate paths must be rejected");

        assert!(error.to_string().contains("duplicate path"));
    }

    #[test]
    fn nested_entries_require_declared_parent_directories() {
        let manifest = directory_manifest(vec![ManifestEntry::File {
            path: "docs/readme.txt".into(),
            size_bytes: 0,
            sha256: EMPTY_SHA256.into(),
        }]);

        let error = validate_manifest_for_receive(&manifest)
            .expect_err("undeclared parent directory must be rejected");

        assert!(error.to_string().contains("missing parent directory"));
    }

    #[test]
    fn paths_cannot_descend_through_files() {
        let manifest = directory_manifest(vec![
            ManifestEntry::File {
                path: "docs".into(),
                size_bytes: 0,
                sha256: EMPTY_SHA256.into(),
            },
            ManifestEntry::File {
                path: "docs/readme.txt".into(),
                size_bytes: 0,
                sha256: EMPTY_SHA256.into(),
            },
        ]);

        let error = validate_manifest_for_receive(&manifest)
            .expect_err("file cannot be used as a parent directory");

        assert!(error.to_string().contains("descends through a file"));
    }

    #[test]
    fn malformed_hashes_are_rejected() {
        let manifest = directory_manifest(vec![ManifestEntry::File {
            path: "bad.txt".into(),
            size_bytes: 1,
            sha256: "ABC".into(),
        }]);

        let error =
            validate_manifest_for_receive(&manifest).expect_err("invalid hash must be rejected");

        assert!(error.to_string().contains("invalid lowercase SHA-256"));
    }

    #[test]
    fn file_root_maps_directly_to_the_selected_destination_name() {
        let manifest = ShareManifest {
            schema_version: 1,
            root_name: "photo.jpg".into(),
            root_kind: RootKind::File,
            entries: vec![ManifestEntry::File {
                path: "photo.jpg".into(),
                size_bytes: 0,
                sha256: EMPTY_SHA256.into(),
            }],
        };

        let plan = build_receive_plan(&manifest, Path::new("/receive"))
            .expect("valid file manifest should produce a plan");

        assert_eq!(plan.root_path, Path::new("/receive").join("photo.jpg"));
        match &plan.entries[0] {
            ReceivePlanEntry::File {
                destination_path, ..
            } => assert_eq!(destination_path, &plan.root_path),
            ReceivePlanEntry::Directory { .. } => panic!("expected file entry"),
        }
    }
}
