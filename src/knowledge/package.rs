// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reproducible, data-only knowledge package archives.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tar::{Archive, Builder, Header};
use thiserror::Error;

const FORMAT_VERSION: u32 = 1;
const MANIFEST_PATH: &str = "knowledge-package.toml";
const CHECKSUMS_PATH: &str = "checksums.sha256";
const ARCHIVE_PATH: &str = "package.tar.zst";
const MAX_FILES: usize = 10_000;
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_EXPANDED_BYTES: u64 = 64 * 1024 * 1024;
const MAX_COMPRESSED_BYTES: u64 = 32 * 1024 * 1024;

/// The declared identity of a data-only package archive.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageManifest {
    #[serde(default = "format_version")]
    pub format_version: u32,
    pub name: String,
    pub version: String,
}

/// One verified package content file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PackageFile {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

/// Package identity plus its exact archive content inventory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PackageArchive {
    pub manifest: PackageManifest,
    pub archive_sha256: String,
    pub files: Vec<PackageFile>,
}

/// A content-addressed package materialised in one deployment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InstalledPackage {
    pub package: PackageArchive,
    pub path: String,
    pub already_installed: bool,
}

/// An installed package with its content-addressed directory name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StoredPackage {
    pub digest: String,
    pub package: PackageArchive,
}

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("reading {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("package source `{0}` is not a directory")]
    NotDirectory(PathBuf),
    #[error("package archive `{archive}` must not be inside source directory `{package_root}`")]
    ArchiveInsideSource {
        package_root: PathBuf,
        archive: PathBuf,
    },
    #[error("package is missing {MANIFEST_PATH}")]
    MissingManifest,
    #[error("package archive is missing {CHECKSUMS_PATH}")]
    MissingChecksums,
    #[error("parsing package manifest: {0}")]
    Manifest(#[from] toml::de::Error),
    #[error("serializing package manifest: {0}")]
    SerializeManifest(#[from] toml::ser::Error),
    #[error("unsupported package format version {0}; expected {FORMAT_VERSION}")]
    UnsupportedFormatVersion(u32),
    #[error("package name and version must not be empty")]
    EmptyIdentity,
    #[error("unsafe package path `{0}`")]
    UnsafePath(String),
    #[error("package file `{0}` is outside allowed data directories")]
    DisallowedPath(String),
    #[error("package archive contains unsupported entry `{0}`")]
    UnsupportedEntry(String),
    #[error("duplicate package path `{0}`")]
    DuplicatePath(String),
    #[error("package has {0} files, exceeding limit {MAX_FILES}")]
    TooManyFiles(usize),
    #[error("compressed package is {bytes} bytes, exceeding limit {MAX_COMPRESSED_BYTES}")]
    CompressedTooLarge { bytes: u64 },
    #[error("package file `{path}` is {bytes} bytes, exceeding limit {MAX_FILE_BYTES}")]
    FileTooLarge { path: String, bytes: u64 },
    #[error("package expanded size exceeds limit {MAX_EXPANDED_BYTES}")]
    ExpandedTooLarge,
    #[error("invalid checksum line `{0}`")]
    InvalidChecksum(String),
    #[error("missing checksum for `{0}`")]
    MissingChecksum(String),
    #[error("checksum mismatch for `{path}`: expected {expected}, got {actual}")]
    ChecksumMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    #[error("checksum declares missing package file `{0}`")]
    ChecksumForMissingFile(String),
    #[error("archive must have a .tar.zst extension: `{0}`")]
    WrongExtension(PathBuf),
    #[error("invalid package store entry `{0}`")]
    InvalidStoreEntry(String),
    #[error("installed package digest mismatch: expected {expected}, got {actual}")]
    StoreDigestMismatch { expected: String, actual: String },
}

impl PackageArchive {
    /// Build a deterministic `.tar.zst` archive from a declared package directory.
    pub fn build(source: &Path, archive: &Path) -> Result<Self, PackageError> {
        if !source.is_dir() {
            return Err(PackageError::NotDirectory(source.to_path_buf()));
        }
        check_extension(archive)?;
        if archive.starts_with(source) {
            return Err(PackageError::ArchiveInsideSource {
                package_root: source.to_path_buf(),
                archive: archive.to_path_buf(),
            });
        }
        let manifest = read_manifest(&source.join(MANIFEST_PATH))?;
        let files = source_files(source)?;
        let package_files = files
            .iter()
            .map(|(path, bytes)| PackageFile {
                path: path.clone(),
                sha256: digest(bytes),
                bytes: bytes.len() as u64,
            })
            .collect::<Vec<_>>();
        let checksums = render_checksums(&package_files);
        let manifest_text = render_manifest(&manifest)?;

        let temporary = temporary_path(archive);
        let result = write_archive_file(&temporary, &manifest_text, &files, &checksums)
            .and_then(|()| Self::verify(&temporary));
        let package = match result {
            Ok(package) => package,
            Err(error) => {
                let _ = fs::remove_file(&temporary);
                return Err(error);
            }
        };
        if let Err(source) = replace_file(&temporary, archive) {
            let _ = fs::remove_file(&temporary);
            return Err(PackageError::Io {
                path: archive.to_path_buf(),
                source,
            });
        }
        Ok(package)
    }

    /// Read package metadata and validate archive structure without checksum comparison.
    pub fn inspect(archive: &Path) -> Result<Self, PackageError> {
        read_archive(archive, false).map(|(package, _)| package)
    }

    /// Read package metadata and verify every declared package file checksum.
    pub fn verify(archive: &Path) -> Result<Self, PackageError> {
        read_archive(archive, true).map(|(package, _)| package)
    }

    /// Verify then atomically materialise package data under a digest-named directory.
    pub fn install(
        archive: &Path,
        deployment_root: &Path,
    ) -> Result<InstalledPackage, PackageError> {
        let (package, files) = read_archive(archive, true)?;
        let packages = deployment_root.join("knowledge").join("packages");
        fs::create_dir_all(&packages).map_err(|source| PackageError::Io {
            path: packages.clone(),
            source,
        })?;
        let destination = packages.join(format!("sha256-{}", package.archive_sha256));
        if destination.is_dir() {
            return Ok(InstalledPackage {
                package,
                path: path_to_string(&destination)?,
                already_installed: true,
            });
        }
        let staging = packages.join(format!(
            ".sha256-{}.{}.tmp",
            package.archive_sha256,
            std::process::id()
        ));
        fs::create_dir(&staging).map_err(|source| PackageError::Io {
            path: staging.clone(),
            source,
        })?;
        let result = write_installed_package(&staging, archive, &package, &files);
        if let Err(error) = result {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
        if let Err(source) = fs::rename(&staging, &destination) {
            let _ = fs::remove_dir_all(&staging);
            return Err(PackageError::Io {
                path: destination,
                source,
            });
        }
        Ok(InstalledPackage {
            package,
            path: path_to_string(&destination)?,
            already_installed: false,
        })
    }

    /// List every well-formed package materialised in a deployment.
    pub fn list_installed(deployment_root: &Path) -> Result<Vec<StoredPackage>, PackageError> {
        let packages = deployment_root.join("knowledge").join("packages");
        if !packages.exists() {
            return Ok(Vec::new());
        }
        let mut directories = fs::read_dir(&packages)
            .map_err(|source| PackageError::Io {
                path: packages.clone(),
                source,
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| PackageError::Io {
                path: packages.clone(),
                source,
            })?;
        directories.sort_by_key(|entry| entry.file_name());
        let mut stored = Vec::new();
        for directory in directories {
            let path = directory.path();
            if !directory
                .file_type()
                .map_err(|source| PackageError::Io {
                    path: path.clone(),
                    source,
                })?
                .is_dir()
            {
                continue;
            }
            let name = directory.file_name().to_string_lossy().to_string();
            let digest = store_digest(&name)?;
            let package = read_installed_package(&path, &digest, false)?;
            stored.push(StoredPackage { digest, package });
        }
        Ok(stored)
    }

    /// Audit every installed package's materialised files and checksums.
    pub fn audit_installed(deployment_root: &Path) -> Result<Vec<StoredPackage>, PackageError> {
        let packages = deployment_root.join("knowledge").join("packages");
        if !packages.exists() {
            return Ok(Vec::new());
        }
        let mut directories = fs::read_dir(&packages)
            .map_err(|source| PackageError::Io {
                path: packages.clone(),
                source,
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| PackageError::Io {
                path: packages.clone(),
                source,
            })?;
        directories.sort_by_key(|entry| entry.file_name());
        let mut stored = Vec::new();
        for directory in directories {
            let path = directory.path();
            if !directory
                .file_type()
                .map_err(|source| PackageError::Io {
                    path: path.clone(),
                    source,
                })?
                .is_dir()
            {
                continue;
            }
            {
                let name = directory.file_name().to_string_lossy().to_string();
                let digest = store_digest(&name)?;
                let package = read_installed_package(&directory.path(), &digest, true)?;
                stored.push(StoredPackage { digest, package });
            }
        }
        Ok(stored)
    }
}

fn store_digest(name: &str) -> Result<String, PackageError> {
    name.strip_prefix("sha256-")
        .filter(|digest| digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .map(str::to_string)
        .ok_or_else(|| PackageError::InvalidStoreEntry(name.to_string()))
}

fn write_archive_file(
    archive: &Path,
    manifest: &str,
    files: &[(String, Vec<u8>)],
    checksums: &str,
) -> Result<(), PackageError> {
    let output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(archive)
        .map_err(|source| PackageError::Io {
            path: archive.to_path_buf(),
            source,
        })?;
    let encoder = zstd::stream::Encoder::new(output, 19).map_err(|source| PackageError::Io {
        path: archive.to_path_buf(),
        source,
    })?;
    let mut builder = Builder::new(encoder);
    append_file(&mut builder, MANIFEST_PATH, manifest.as_bytes())?;
    for (path, bytes) in files {
        append_file(&mut builder, path, bytes)?;
    }
    append_file(&mut builder, CHECKSUMS_PATH, checksums.as_bytes())?;
    builder.finish().map_err(|source| PackageError::Io {
        path: archive.to_path_buf(),
        source,
    })?;
    builder
        .into_inner()
        .map_err(|source| PackageError::Io {
            path: archive.to_path_buf(),
            source,
        })?
        .finish()
        .map_err(|source| PackageError::Io {
            path: archive.to_path_buf(),
            source,
        })?;
    Ok(())
}

fn read_archive(
    archive_path: &Path,
    verify: bool,
) -> Result<(PackageArchive, BTreeMap<String, Vec<u8>>), PackageError> {
    check_extension(archive_path)?;
    let metadata = fs::metadata(archive_path).map_err(|source| PackageError::Io {
        path: archive_path.to_path_buf(),
        source,
    })?;
    if metadata.len() > MAX_COMPRESSED_BYTES {
        return Err(PackageError::CompressedTooLarge {
            bytes: metadata.len(),
        });
    }
    let archive_bytes = fs::read(archive_path).map_err(|source| PackageError::Io {
        path: archive_path.to_path_buf(),
        source,
    })?;
    let decoder =
        zstd::stream::Decoder::new(&archive_bytes[..]).map_err(|source| PackageError::Io {
            path: archive_path.to_path_buf(),
            source,
        })?;
    let mut archive = Archive::new(decoder);
    let mut entries: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut expanded = 0_u64;

    for entry in archive.entries().map_err(|source| PackageError::Io {
        path: archive_path.to_path_buf(),
        source,
    })? {
        let mut entry = entry.map_err(|source| PackageError::Io {
            path: archive_path.to_path_buf(),
            source,
        })?;
        let path = entry.path().map_err(|source| PackageError::Io {
            path: archive_path.to_path_buf(),
            source,
        })?;
        let path = path_to_string(&path)?;
        safe_path(&path)?;
        if !entry.header().entry_type().is_file() {
            return Err(PackageError::UnsupportedEntry(path));
        }
        let bytes = entry.size();
        if bytes > MAX_FILE_BYTES {
            return Err(PackageError::FileTooLarge { path, bytes });
        }
        expanded = expanded
            .checked_add(bytes)
            .ok_or(PackageError::ExpandedTooLarge)?;
        if expanded > MAX_EXPANDED_BYTES {
            return Err(PackageError::ExpandedTooLarge);
        }
        let data_entries = entries
            .keys()
            .filter(|existing| {
                existing.as_str() != MANIFEST_PATH && existing.as_str() != CHECKSUMS_PATH
            })
            .count();
        if path != MANIFEST_PATH && path != CHECKSUMS_PATH && data_entries >= MAX_FILES {
            return Err(PackageError::TooManyFiles(data_entries + 1));
        }
        let mut content = Vec::with_capacity(bytes as usize);
        entry
            .read_to_end(&mut content)
            .map_err(|source| PackageError::Io {
                path: archive_path.to_path_buf(),
                source,
            })?;
        if entries.insert(path.clone(), content).is_some() {
            return Err(PackageError::DuplicatePath(path));
        }
    }

    let manifest_text = entries
        .remove(MANIFEST_PATH)
        .ok_or(PackageError::MissingManifest)?;
    let manifest = parse_manifest(&manifest_text)?;
    let checksums_text = entries
        .remove(CHECKSUMS_PATH)
        .ok_or(PackageError::MissingChecksums)?;
    let declared = parse_checksums(&checksums_text)?;
    let mut files = Vec::new();
    for (path, content) in &entries {
        data_path(path)?;
        let actual = digest(content);
        if verify {
            let expected = declared
                .get(path)
                .ok_or_else(|| PackageError::MissingChecksum(path.clone()))?;
            if expected != &actual {
                return Err(PackageError::ChecksumMismatch {
                    path: path.clone(),
                    expected: expected.clone(),
                    actual,
                });
            }
        }
        files.push(PackageFile {
            path: path.clone(),
            sha256: actual,
            bytes: content.len() as u64,
        });
    }
    if verify {
        for path in declared.keys() {
            if !entries.contains_key(path) {
                return Err(PackageError::ChecksumForMissingFile(path.clone()));
            }
        }
    }
    Ok((
        PackageArchive {
            manifest,
            archive_sha256: digest(&archive_bytes),
            files,
        },
        entries,
    ))
}

fn write_installed_package(
    staging: &Path,
    archive: &Path,
    package: &PackageArchive,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<(), PackageError> {
    let stored_archive = staging.join(ARCHIVE_PATH);
    fs::copy(archive, &stored_archive).map_err(|source| PackageError::Io {
        path: stored_archive.clone(),
        source,
    })?;
    let stored = PackageArchive::verify(&stored_archive)?;
    if stored.archive_sha256 != package.archive_sha256 {
        return Err(PackageError::StoreDigestMismatch {
            expected: package.archive_sha256.clone(),
            actual: stored.archive_sha256,
        });
    }
    let manifest = render_manifest(&package.manifest)?;
    write_package_file(staging, MANIFEST_PATH, manifest.as_bytes())?;
    for (path, content) in files {
        write_package_file(staging, path, content)?;
    }
    write_package_file(
        staging,
        CHECKSUMS_PATH,
        render_checksums(&package.files).as_bytes(),
    )
}

fn read_installed_package(
    path: &Path,
    digest_name: &str,
    audit: bool,
) -> Result<PackageArchive, PackageError> {
    let (package, expected) = read_archive(&path.join(ARCHIVE_PATH), true)?;
    if package.archive_sha256 != digest_name {
        return Err(PackageError::StoreDigestMismatch {
            expected: digest_name.to_string(),
            actual: package.archive_sha256,
        });
    }
    if !audit {
        return Ok(package);
    }
    let declared = expected
        .iter()
        .map(|(path, content)| (path.clone(), digest(content)))
        .collect::<BTreeMap<_, _>>();
    for (relative, _) in installed_data_files(path)? {
        if !declared.contains_key(&relative) {
            return Err(PackageError::MissingChecksum(relative));
        }
    }
    let mut files = Vec::new();
    for (relative, expected) in &declared {
        let file = path.join(relative);
        let metadata = fs::symlink_metadata(&file).map_err(|source| PackageError::Io {
            path: file.clone(),
            source,
        })?;
        if !metadata.file_type().is_file() {
            return Err(PackageError::UnsupportedEntry(relative.clone()));
        }
        if metadata.len() > MAX_FILE_BYTES {
            return Err(PackageError::FileTooLarge {
                path: relative.clone(),
                bytes: metadata.len(),
            });
        }
        let content = fs::read(&file).map_err(|source| PackageError::Io { path: file, source })?;
        let actual = digest(&content);
        if &actual != expected {
            return Err(PackageError::ChecksumMismatch {
                path: relative.clone(),
                expected: expected.clone(),
                actual,
            });
        }
        files.push(PackageFile {
            path: relative.clone(),
            sha256: expected.clone(),
            bytes: content.len() as u64,
        });
    }
    Ok(PackageArchive { files, ..package })
}

fn installed_data_files(root: &Path) -> Result<Vec<(String, Vec<u8>)>, PackageError> {
    let mut paths = Vec::new();
    collect_files(root, root, &mut paths)?;
    let mut files = Vec::new();
    for path in paths {
        let relative = path_to_string(path.strip_prefix(root).expect("descendant path"))?;
        if matches!(
            relative.as_str(),
            MANIFEST_PATH | CHECKSUMS_PATH | ARCHIVE_PATH
        ) {
            continue;
        }
        data_path(&relative)?;
        let content = fs::read(&path).map_err(|source| PackageError::Io {
            path: path.clone(),
            source,
        })?;
        files.push((relative, content));
    }
    Ok(files)
}

fn write_package_file(root: &Path, path: &str, content: &[u8]) -> Result<(), PackageError> {
    safe_path(path)?;
    if path != MANIFEST_PATH && path != CHECKSUMS_PATH {
        data_path(path)?;
    }
    let destination = root.join(path);
    let parent = destination.parent().expect("package path has parent");
    fs::create_dir_all(parent).map_err(|source| PackageError::Io {
        path: parent.to_path_buf(),
        source,
    })?;
    fs::write(&destination, content).map_err(|source| PackageError::Io {
        path: destination,
        source,
    })
}

fn read_manifest(path: &Path) -> Result<PackageManifest, PackageError> {
    let text = fs::read(path).map_err(|source| PackageError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    parse_manifest(&text)
}

fn parse_manifest(text: &[u8]) -> Result<PackageManifest, PackageError> {
    let text = std::str::from_utf8(text).map_err(|_| PackageError::MissingManifest)?;
    let manifest: PackageManifest = toml::from_str(text)?;
    if manifest.format_version != FORMAT_VERSION {
        return Err(PackageError::UnsupportedFormatVersion(
            manifest.format_version,
        ));
    }
    if manifest.name.trim().is_empty() || manifest.version.trim().is_empty() {
        return Err(PackageError::EmptyIdentity);
    }
    Ok(manifest)
}

fn render_manifest(manifest: &PackageManifest) -> Result<String, PackageError> {
    let mut text = toml::to_string_pretty(manifest)?;
    if !text.ends_with('\n') {
        text.push('\n');
    }
    Ok(text)
}

fn source_files(source: &Path) -> Result<Vec<(String, Vec<u8>)>, PackageError> {
    let mut paths = Vec::new();
    collect_files(source, source, &mut paths)?;
    let mut result = Vec::new();
    for path in paths {
        let relative = path_to_string(path.strip_prefix(source).expect("descendant path"))?;
        if relative == MANIFEST_PATH || relative == CHECKSUMS_PATH {
            continue;
        }
        data_path(&relative)?;
        let content = fs::read(&path).map_err(|source_error| PackageError::Io {
            path: path.clone(),
            source: source_error,
        })?;
        if content.len() as u64 > MAX_FILE_BYTES {
            return Err(PackageError::FileTooLarge {
                path: relative,
                bytes: content.len() as u64,
            });
        }
        result.push((relative, content));
    }
    if result.len() > MAX_FILES {
        return Err(PackageError::TooManyFiles(result.len()));
    }
    let expanded: u64 = result.iter().map(|(_, content)| content.len() as u64).sum();
    if expanded > MAX_EXPANDED_BYTES {
        return Err(PackageError::ExpandedTooLarge);
    }
    result.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(result)
}

fn collect_files(
    root: &Path,
    directory: &Path,
    output: &mut Vec<PathBuf>,
) -> Result<(), PackageError> {
    let mut entries = fs::read_dir(directory)
        .map_err(|source| PackageError::Io {
            path: directory.to_path_buf(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| PackageError::Io {
            path: directory.to_path_buf(),
            source,
        })?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let relative = path_to_string(path.strip_prefix(root).expect("descendant path"))?;
        let file_type = entry.file_type().map_err(|source| PackageError::Io {
            path: path.clone(),
            source,
        })?;
        if file_type.is_dir() {
            safe_path(&relative)?;
            collect_files(root, &path, output)?;
        } else if file_type.is_file() {
            output.push(path);
        } else {
            return Err(PackageError::UnsupportedEntry(relative));
        }
    }
    Ok(())
}

fn append_file(
    builder: &mut Builder<impl Write>,
    path: &str,
    content: &[u8],
) -> Result<(), PackageError> {
    let mut header = Header::new_gnu();
    header.set_size(content.len() as u64);
    header.set_mode(0o644);
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(0);
    header.set_cksum();
    builder
        .append_data(&mut header, path, content)
        .map_err(|source| PackageError::Io {
            path: PathBuf::from(path),
            source,
        })
}

fn render_checksums(files: &[PackageFile]) -> String {
    files
        .iter()
        .map(|file| format!("{}  {}\n", file.sha256, file.path))
        .collect()
}

fn parse_checksums(content: &[u8]) -> Result<BTreeMap<String, String>, PackageError> {
    let text = std::str::from_utf8(content)
        .map_err(|_| PackageError::InvalidChecksum("non-UTF-8".to_string()))?;
    let mut checksums = BTreeMap::new();
    for line in text.lines() {
        let (hash, path) = line
            .split_once("  ")
            .ok_or_else(|| PackageError::InvalidChecksum(line.to_string()))?;
        if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(PackageError::InvalidChecksum(line.to_string()));
        }
        safe_path(path)?;
        data_path(path)?;
        if checksums
            .insert(path.to_string(), hash.to_string())
            .is_some()
        {
            return Err(PackageError::DuplicatePath(path.to_string()));
        }
    }
    Ok(checksums)
}

fn data_path(path: &str) -> Result<(), PackageError> {
    if path.starts_with("artefacts/") || path.starts_with("provenance/") {
        Ok(())
    } else {
        Err(PackageError::DisallowedPath(path.to_string()))
    }
}

fn safe_path(path: &str) -> Result<(), PackageError> {
    let parsed = Path::new(path);
    if path.is_empty()
        || parsed.is_absolute()
        || path
            .split('/')
            .any(|segment| segment.is_empty() || segment == ".")
        || parsed.components().any(|component| {
            matches!(
                component,
                Component::CurDir
                    | Component::ParentDir
                    | Component::RootDir
                    | Component::Prefix(_)
            )
        })
    {
        return Err(PackageError::UnsafePath(path.to_string()));
    }
    Ok(())
}

fn path_to_string(path: &Path) -> Result<String, PackageError> {
    path.to_str()
        .map(|value| value.replace('\\', "/"))
        .ok_or_else(|| PackageError::UnsafePath(path.display().to_string()))
}

fn check_extension(path: &Path) -> Result<(), PackageError> {
    if path.to_string_lossy().ends_with(".tar.zst") {
        Ok(())
    } else {
        Err(PackageError::WrongExtension(path.to_path_buf()))
    }
}

fn temporary_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("package.tar.zst");
    path.with_file_name(format!(".{name}.{}.tmp.tar.zst", std::process::id()))
}

#[cfg(not(windows))]
fn replace_file(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

#[cfg(windows)]
fn replace_file(from: &Path, to: &Path) -> io::Result<()> {
    if to.exists() {
        fs::remove_file(to)?;
    }
    fs::rename(from, to)
}

fn digest(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

const fn format_version() -> u32 {
    FORMAT_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> tempfile::TempDir {
        let directory = tempfile::tempdir().expect("temporary package source");
        fs::create_dir_all(directory.path().join("artefacts/archetypes")).expect("create content");
        fs::write(
            directory.path().join(MANIFEST_PATH),
            "format_version = 1\nname = \"test-package\"\nversion = \"1.0.0\"\n",
        )
        .expect("write manifest");
        fs::write(
            directory
                .path()
                .join("artefacts/archetypes/openEHR-EHR-CLUSTER.example.v1.adl"),
            "archetype example\n",
        )
        .expect("write artefact");
        directory
    }

    #[test]
    fn builds_byte_identical_verified_archives() {
        let source = source();
        let output = tempfile::tempdir().expect("temporary package output");
        let first = output.path().join("first.tar.zst");
        let second = output.path().join("second.tar.zst");
        let package = PackageArchive::build(source.path(), &first).expect("build package");
        PackageArchive::build(source.path(), &second).expect("rebuild package");

        assert_eq!(fs::read(&first).unwrap(), fs::read(&second).unwrap());
        assert_eq!(package.manifest.name, "test-package");
        assert_eq!(package.files.len(), 1);
        assert_eq!(PackageArchive::verify(&first).unwrap(), package);
    }

    #[test]
    fn rejects_checksum_mismatch_without_extraction() {
        let source = source();
        let output = tempfile::tempdir().expect("temporary package output");
        let archive = output.path().join("package.tar.zst");
        PackageArchive::build(source.path(), &archive).expect("build package");
        let tampered = output.path().join("tampered.tar.zst");
        write_archive(
            &tampered,
            &[
                (MANIFEST_PATH, b"format_version = 1\nname = \"test-package\"\nversion = \"1.0.0\"\n"),
                ("artefacts/example.txt", b"tampered"),
                (CHECKSUMS_PATH, b"0000000000000000000000000000000000000000000000000000000000000000  artefacts/example.txt\n"),
            ],
        );
        assert!(matches!(
            PackageArchive::verify(&tampered),
            Err(PackageError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn rejects_traversal_and_link_entries() {
        let output = tempfile::tempdir().expect("temporary package output");
        assert!(matches!(
            parse_checksums(
                b"0000000000000000000000000000000000000000000000000000000000000000  ../escape\n"
            ),
            Err(PackageError::UnsafePath(_))
        ));

        let link = output.path().join("link.tar.zst");
        let output = fs::File::create(&link).unwrap();
        let encoder = zstd::stream::Encoder::new(output, 19).unwrap();
        let mut builder = Builder::new(encoder);
        append_file(
            &mut builder,
            MANIFEST_PATH,
            b"format_version = 1\nname = \"test-package\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        let mut header = Header::new_gnu();
        header.set_entry_type(tar::EntryType::symlink());
        header.set_size(0);
        header.set_mode(0o644);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_link_name("target").unwrap();
        header.set_cksum();
        builder
            .append_data(&mut header, "artefacts/link", io::empty())
            .unwrap();
        append_file(&mut builder, CHECKSUMS_PATH, b"").unwrap();
        builder.finish().unwrap();
        builder.into_inner().unwrap().finish().unwrap();
        assert!(matches!(
            PackageArchive::inspect(&link),
            Err(PackageError::UnsupportedEntry(_))
        ));
    }

    #[test]
    fn rejects_archive_inside_its_source_directory() {
        let source = source();
        assert!(matches!(
            PackageArchive::build(source.path(), &source.path().join("package.tar.zst")),
            Err(PackageError::ArchiveInsideSource { .. })
        ));
    }

    #[test]
    fn rejects_non_canonical_checksum_paths_and_oversized_compressed_archives() {
        assert!(matches!(
            parse_checksums(b"0000000000000000000000000000000000000000000000000000000000000000  artefacts//alias\n"),
            Err(PackageError::UnsafePath(_))
        ));
        assert!(matches!(
            parse_checksums(b"0000000000000000000000000000000000000000000000000000000000000000  artefacts/./alias\n"),
            Err(PackageError::UnsafePath(_))
        ));

        let directory = tempfile::tempdir().expect("temporary package output");
        let oversized = directory.path().join("oversized.tar.zst");
        let file = fs::File::create(&oversized).expect("create oversized archive");
        file.set_len(MAX_COMPRESSED_BYTES + 1)
            .expect("set oversized archive length");
        assert!(matches!(
            PackageArchive::inspect(&oversized),
            Err(PackageError::CompressedTooLarge { .. })
        ));
    }

    #[test]
    fn failed_build_preserves_existing_archive() {
        let source = source();
        let output = tempfile::tempdir().expect("temporary package output");
        let archive = output.path().join("package.tar.zst");
        fs::write(&archive, b"existing archive").expect("write existing archive");
        fs::write(source.path().join("unexpected.txt"), b"not package data")
            .expect("write invalid source file");

        assert!(matches!(
            PackageArchive::build(source.path(), &archive),
            Err(PackageError::DisallowedPath(_))
        ));
        assert_eq!(
            fs::read(&archive).expect("read preserved archive"),
            b"existing archive"
        );
    }

    #[test]
    fn installs_verified_data_by_archive_digest_and_is_idempotent() {
        let source = source();
        let output = tempfile::tempdir().expect("temporary package output");
        let deployment = tempfile::tempdir().expect("temporary deployment");
        let archive = output.path().join("package.tar.zst");
        let package = PackageArchive::build(source.path(), &archive).expect("build package");

        let first = PackageArchive::install(&archive, deployment.path()).expect("install package");
        assert!(!first.already_installed);
        let path = deployment
            .path()
            .join("knowledge/packages")
            .join(format!("sha256-{}", package.archive_sha256));
        assert!(path.join(MANIFEST_PATH).is_file());
        assert_eq!(
            fs::read_to_string(
                path.join("artefacts/archetypes/openEHR-EHR-CLUSTER.example.v1.adl")
            )
            .expect("read installed content"),
            "archetype example\n"
        );
        assert!(path.join(CHECKSUMS_PATH).is_file());

        let second =
            PackageArchive::install(&archive, deployment.path()).expect("reinstall package");
        assert!(second.already_installed);
        assert_eq!(first.package, second.package);
    }

    #[test]
    fn failed_install_leaves_no_staging_or_package_content() {
        let output = tempfile::tempdir().expect("temporary package output");
        let deployment = tempfile::tempdir().expect("temporary deployment");
        let archive = output.path().join("tampered.tar.zst");
        write_archive(
            &archive,
            &[
                (
                    MANIFEST_PATH,
                    b"format_version = 1\nname = \"test-package\"\nversion = \"1.0.0\"\n",
                ),
                ("artefacts/example.txt", b"tampered"),
                (
                    CHECKSUMS_PATH,
                    b"0000000000000000000000000000000000000000000000000000000000000000  artefacts/example.txt\n",
                ),
            ],
        );
        assert!(matches!(
            PackageArchive::install(&archive, deployment.path()),
            Err(PackageError::ChecksumMismatch { .. })
        ));
        assert!(!deployment.path().join("knowledge/packages").exists());
    }

    #[test]
    fn audit_detects_tampered_installed_content() {
        let source = source();
        let output = tempfile::tempdir().expect("temporary package output");
        let deployment = tempfile::tempdir().expect("temporary deployment");
        let archive = output.path().join("package.tar.zst");
        let package = PackageArchive::build(source.path(), &archive).expect("build package");
        PackageArchive::install(&archive, deployment.path()).expect("install package");
        let content = deployment
            .path()
            .join("knowledge/packages")
            .join(format!("sha256-{}", package.archive_sha256))
            .join("artefacts/archetypes/openEHR-EHR-CLUSTER.example.v1.adl");
        fs::write(&content, "tampered\n").expect("tamper installed content");

        assert!(matches!(
            PackageArchive::audit_installed(deployment.path()),
            Err(PackageError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn audit_detects_tampered_archive_and_undeclared_material() {
        let source = source();
        let output = tempfile::tempdir().expect("temporary package output");
        let deployment = tempfile::tempdir().expect("temporary deployment");
        let archive = output.path().join("package.tar.zst");
        let package = PackageArchive::build(source.path(), &archive).expect("build package");
        PackageArchive::install(&archive, deployment.path()).expect("install package");
        let store = deployment
            .path()
            .join("knowledge/packages")
            .join(format!("sha256-{}", package.archive_sha256));
        fs::write(store.join("artefacts/extra.txt"), "unexpected\n").expect("add material");
        assert!(matches!(
            PackageArchive::audit_installed(deployment.path()),
            Err(PackageError::MissingChecksum(path)) if path == "artefacts/extra.txt"
        ));
        fs::remove_file(store.join("artefacts/extra.txt")).expect("remove material");
        fs::write(store.join(ARCHIVE_PATH), "tampered\n").expect("tamper stored archive");
        assert!(PackageArchive::audit_installed(deployment.path()).is_err());
    }

    fn write_archive(path: &Path, files: &[(&str, &[u8])]) {
        let output = fs::File::create(path).unwrap();
        let encoder = zstd::stream::Encoder::new(output, 19).unwrap();
        let mut builder = Builder::new(encoder);
        for (path, content) in files {
            append_file(&mut builder, path, content).unwrap();
        }
        builder.finish().unwrap();
        builder.into_inner().unwrap().finish().unwrap();
    }
}
