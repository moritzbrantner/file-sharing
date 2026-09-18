use std::{
    fs::{self, File},
    io::{BufReader, Read},
    path::Path,
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootKind {
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShareManifest {
    pub schema_version: u32,
    pub root_name: String,
    pub root_kind: RootKind,
    pub entries: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ManifestEntry {
    Directory {
        path: String,
    },
    File {
        path: String,
        size_bytes: u64,
        sha256: String,
    },
}

impl ManifestEntry {
    fn path(&self) -> &str {
        match self {
            Self::Directory { path } | Self::File { path, .. } => path,
        }
    }
}

pub fn build_manifest(path: impl AsRef<Path>) -> Result<ShareManifest> {
    let path = path.as_ref();
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("failed to inspect {}", path.display()))?;

    if metadata.file_type().is_symlink() {
        bail!(
            "symbolic-link roots are not supported yet: {}",
            path.display()
        );
    }

    let root_name = portable_name(path)?;
    let (root_kind, mut entries) = if metadata.is_file() {
        (RootKind::File, vec![file_entry(path, root_name.clone())?])
    } else if metadata.is_dir() {
        let mut entries = Vec::new();
        collect_directory(path, path, &mut entries)?;
        (RootKind::Directory, entries)
    } else {
        bail!(
            "only regular files and directories can be shared: {}",
            path.display()
        );
    };

    entries.sort_by(|left, right| left.path().cmp(right.path()));

    Ok(ShareManifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        root_name,
        root_kind,
        entries,
    })
}

pub fn manifest_json_pretty(manifest: &ShareManifest) -> Result<String> {
    serde_json::to_string_pretty(manifest).context("failed to serialize share manifest")
}

fn collect_directory(
    base: &Path,
    directory: &Path,
    entries: &mut Vec<ManifestEntry>,
) -> Result<()> {
    let mut children = fs::read_dir(directory)
        .with_context(|| format!("failed to read directory {}", directory.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("failed to enumerate directory {}", directory.display()))?;

    children.sort_by_key(|entry| entry.file_name());

    for child in children {
        let child_path = child.path();
        let metadata = fs::symlink_metadata(&child_path)
            .with_context(|| format!("failed to inspect {}", child_path.display()))?;

        if metadata.file_type().is_symlink() {
            bail!(
                "symbolic links are not supported yet: {}",
                child_path.display()
            );
        }

        let relative = portable_relative_path(base, &child_path)?;

        if metadata.is_dir() {
            entries.push(ManifestEntry::Directory { path: relative });
            collect_directory(base, &child_path, entries)?;
        } else if metadata.is_file() {
            entries.push(file_entry(&child_path, relative)?);
        } else {
            bail!(
                "only regular files and directories can be shared: {}",
                child_path.display()
            );
        }
    }

    Ok(())
}

fn file_entry(path: &Path, relative_path: String) -> Result<ManifestEntry> {
    let metadata =
        fs::metadata(path).with_context(|| format!("failed to inspect {}", path.display()))?;
    let sha256 = sha256_file(path)?;

    Ok(ManifestEntry::File {
        path: relative_path,
        size_bytes: metadata.len(),
        sha256,
    })
}

fn sha256_file(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let read = reader
            .read(&mut buffer)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn portable_name(path: &Path) -> Result<String> {
    let resolved = fs::canonicalize(path)
        .with_context(|| format!("failed to resolve share root {}", path.display()))?;
    let name = resolved
        .file_name()
        .context("share path has no portable name")?;

    name.to_str()
        .map(ToOwned::to_owned)
        .context("share path name is not valid UTF-8")
}

fn portable_relative_path(base: &Path, path: &Path) -> Result<String> {
    let relative = path
        .strip_prefix(base)
        .with_context(|| format!("{} is outside {}", path.display(), base.display()))?;

    let mut components = Vec::new();
    for component in relative.components() {
        let value = component
            .as_os_str()
            .to_str()
            .context("share path contains a non-UTF-8 component")?;
        components.push(value);
    }

    if components.is_empty() {
        bail!("share entry cannot use an empty relative path");
    }

    Ok(components.join("/"))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{ManifestEntry, RootKind, build_manifest, manifest_json_pretty};

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after Unix epoch")
                .as_nanos();
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "file-sharing-test-{}-{nonce}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("test directory should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn single_file_manifest_uses_sha256_content_identity() {
        let temp = TestDir::new();
        let file = temp.path().join("hello.txt");
        fs::write(&file, b"abc").expect("fixture should be written");

        let manifest = build_manifest(&file).expect("manifest should build");

        assert_eq!(manifest.root_kind, RootKind::File);
        assert_eq!(manifest.root_name, "hello.txt");
        assert_eq!(
            manifest.entries,
            vec![ManifestEntry::File {
                path: "hello.txt".into(),
                size_bytes: 3,
                sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
            }]
        );
    }

    #[test]
    fn folder_manifest_is_sorted_and_preserves_empty_directories() {
        let temp = TestDir::new();
        fs::create_dir_all(temp.path().join("nested")).expect("nested directory should be created");
        fs::create_dir_all(temp.path().join("empty")).expect("empty directory should be created");
        fs::write(temp.path().join("z.txt"), b"z").expect("fixture should be written");
        fs::write(temp.path().join("nested").join("a.txt"), b"a")
            .expect("fixture should be written");

        let manifest = build_manifest(temp.path()).expect("manifest should build");

        let paths = manifest
            .entries
            .iter()
            .map(|entry| match entry {
                ManifestEntry::Directory { path } | ManifestEntry::File { path, .. } => {
                    path.as_str()
                }
            })
            .collect::<Vec<_>>();

        assert_eq!(paths, vec!["empty", "nested", "nested/a.txt", "z.txt"]);
    }

    #[test]
    fn manifest_json_is_stable_for_unchanged_content() {
        let temp = TestDir::new();
        fs::write(temp.path().join("b.txt"), b"two").expect("fixture should be written");
        fs::write(temp.path().join("a.txt"), b"one").expect("fixture should be written");

        let first = build_manifest(temp.path()).expect("first manifest should build");
        let second = build_manifest(temp.path()).expect("second manifest should build");

        assert_eq!(
            manifest_json_pretty(&first).expect("first JSON should serialize"),
            manifest_json_pretty(&second).expect("second JSON should serialize")
        );
    }

    #[test]
    fn dot_path_uses_the_actual_directory_name() {
        let temp = TestDir::new();
        fs::write(temp.path().join("hello.txt"), b"hello").expect("fixture should be written");

        let current = std::env::current_dir().expect("current directory should be available");
        std::env::set_current_dir(temp.path()).expect("test should enter fixture directory");
        let manifest = build_manifest(".").expect("dot path should build");
        std::env::set_current_dir(current).expect("test should restore current directory");

        assert_eq!(
            manifest.root_name,
            temp.path()
                .file_name()
                .and_then(|name| name.to_str())
                .expect("fixture directory should have a UTF-8 name")
        );
        assert_ne!(manifest.root_name, ".");
        assert_ne!(manifest.root_name, "..");
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_links_are_rejected() {
        use std::os::unix::fs::symlink;

        let temp = TestDir::new();
        let target = temp.path().join("target.txt");
        fs::write(&target, b"target").expect("fixture should be written");
        symlink(&target, temp.path().join("link.txt")).expect("symlink should be created");

        let error = build_manifest(temp.path()).expect_err("symlink should be rejected");
        assert!(
            error
                .to_string()
                .contains("symbolic links are not supported")
        );
    }
}
