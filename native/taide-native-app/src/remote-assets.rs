use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use taide_infra::range_file::extension_mime;
use taide_model::error::{AppError, AppResult};
use taide_runtime::TaskSupervisor;

use crate::preview_web_file::{Anchor, Stamp};
use crate::remote_serving::{Asset, AssetResolver};

const INDEX_DOCUMENT: &str = "index.html";
const BINARY_MIME: &str = "application/octet-stream";
const BUNDLE_DIRECTORY: &str = "remote-public";
const BUNDLE_MANIFEST: &str = "bundle-manifest.json";
const BUNDLE_FORMAT: &str = "taide-rust-remote-v1";
const MANIFEST_BYTES: usize = 64 * 1024;
const PACKAGED_PAYLOAD_BYTES: usize = 64 * 1024 * 1024;
const PACKAGED_ASSET_COUNT: usize = 1024;
const WASM_HEADER: &[u8] = b"\0asm\x01\0\0\0";

#[derive(Clone, Copy)]
pub struct Limits {
    pub payload_bytes: usize,
    pub count: usize,
}

pub struct Catalog {
    root: PathBuf,
    manifest: Source,
    limits: Limits,
    loading: tokio::sync::Mutex<()>,
    loaded: Arc<RwLock<Option<AssetResolver>>>,
}

enum Source {
    Explicit(Vec<String>),
    Bundle,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BundleManifest {
    format: String,
    entry_wasm: String,
    files: Vec<String>,
}

impl Catalog {
    pub fn new(root: PathBuf, manifest: Vec<String>, limits: Limits) -> Self {
        Self {
            root,
            manifest: Source::Explicit(manifest),
            limits,
            loading: tokio::sync::Mutex::new(()),
            loaded: Arc::new(RwLock::new(None)),
        }
    }

    pub fn bundle(root: PathBuf, limits: Limits) -> Self {
        Self {
            root,
            manifest: Source::Bundle,
            limits,
            loading: tokio::sync::Mutex::new(()),
            loaded: Arc::new(RwLock::new(None)),
        }
    }

    pub fn packaged(executable: &Path) -> AppResult<Self> {
        validate_root(executable)?;
        let binary_directory = executable
            .parent()
            .filter(|path| path.parent().is_some())
            .ok_or_else(|| invalid("native executable directory is invalid"))?;
        let contents = binary_directory.parent().filter(|path| {
            binary_directory
                .file_name()
                .is_some_and(|name| name == "MacOS")
                && path.file_name().is_some_and(|name| name == "Contents")
                && path.parent().is_some_and(|bundle| {
                    bundle
                        .extension()
                        .is_some_and(|extension| extension == "app")
                })
        });
        let root = match contents {
            Some(contents) => contents.join("Resources").join(BUNDLE_DIRECTORY),
            None => binary_directory.join(BUNDLE_DIRECTORY),
        };
        Ok(Self::bundle(
            root,
            Limits {
                payload_bytes: PACKAGED_PAYLOAD_BYTES,
                count: PACKAGED_ASSET_COUNT,
            },
        ))
    }

    pub fn resolver(&self) -> AssetResolver {
        let loaded = self.loaded.clone();
        Arc::new(move |name| {
            let resolver = loaded.read().ok()?.clone()?;
            resolver(name)
        })
    }

    pub async fn prepare(self: Arc<Self>, tasks: TaskSupervisor) -> AppResult<()> {
        let worker_tasks = tasks.clone();
        tasks
            .run_nonabortable_result("native-remote-assets-prepare", async move {
                let _loading = self.loading.lock().await;
                if self.loaded.read().map_err(|_| poisoned())?.is_some() {
                    return Ok(());
                }
                let resolver = match &self.manifest {
                    Source::Explicit(manifest) => {
                        load(
                            &worker_tasks,
                            self.root.clone(),
                            manifest.clone(),
                            self.limits,
                        )
                        .await?
                    }
                    Source::Bundle => {
                        load_bundle(&worker_tasks, self.root.clone(), self.limits).await?
                    }
                };
                *self.loaded.write().map_err(|_| poisoned())? = Some(resolver);
                Ok(())
            })
            .await
    }
}

fn poisoned() -> AppError {
    AppError::Internal("native remote asset catalog lock poisoned".into())
}

pub async fn load(
    tasks: &TaskSupervisor,
    root: PathBuf,
    manifest: Vec<String>,
    limits: Limits,
) -> AppResult<AssetResolver> {
    tasks
        .run_blocking_result("native-remote-assets", move || {
            let entries = catalog(root, manifest, limits)?;
            let resolver: AssetResolver = Arc::new(move |name| {
                entries.get(name).map(|asset| Asset {
                    mime: asset.mime.clone(),
                    bytes: asset.bytes.clone(),
                })
            });
            Ok(resolver)
        })
        .await
}

fn catalog(
    root: PathBuf,
    manifest: Vec<String>,
    limits: Limits,
) -> AppResult<BTreeMap<String, Asset>> {
    validate_manifest(&manifest, limits)?;
    validate_root(&root)?;
    let anchor = Anchor::open(root)?;
    read_entries(&anchor, manifest, limits)
}

fn validate_manifest(manifest: &[String], limits: Limits) -> AppResult<()> {
    if limits.payload_bytes == 0 || limits.count == 0 || manifest.len() > limits.count {
        return Err(invalid("remote public asset quota is invalid or exceeded"));
    }
    let mut names = BTreeSet::new();
    for name in manifest {
        validate(name)?;
        mime(name)?;
        if !names.insert(name.as_str()) {
            return Err(invalid(
                "remote public asset manifest has duplicate entries",
            ));
        }
    }
    if !names.contains(INDEX_DOCUMENT) {
        return Err(invalid("remote public asset manifest requires index.html"));
    }
    Ok(())
}

fn validate_root(root: &Path) -> AppResult<()> {
    if !root.is_absolute()
        || root.parent().is_none()
        || root
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(invalid("remote public asset root is invalid"));
    }
    Ok(())
}

fn read_entries(
    anchor: &Anchor,
    manifest: Vec<String>,
    limits: Limits,
) -> AppResult<BTreeMap<String, Asset>> {
    let mut entries = BTreeMap::new();
    let mut total_bytes: usize = 0;
    for name in manifest {
        let bytes = read_public_file(anchor, &name, limits.payload_bytes - total_bytes)?;
        total_bytes = total_bytes
            .checked_add(bytes.len())
            .filter(|total| *total <= limits.payload_bytes)
            .ok_or_else(|| invalid("remote public asset payload quota exceeded"))?;
        entries.insert(
            name.clone(),
            Asset {
                mime: mime(&name)?.into(),
                bytes,
            },
        );
    }
    anchor.check()?;
    Ok(entries)
}

fn read_public_file(anchor: &Anchor, name: &str, limit: usize) -> AppResult<Vec<u8>> {
    let mut file = anchor.file(&anchor.path.join(name))?;
    let before = Stamp::read(&file)?;
    let length = usize::try_from(file.metadata()?.len())
        .map_err(|_| invalid("remote public asset length exceeds addressable memory"))?;
    if length > limit {
        return Err(invalid("remote public asset payload quota exceeded"));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| invalid("remote public asset allocation failed"))?;
    bytes.resize(length, 0);
    file.read_exact(&mut bytes)?;
    let mut extra = [0u8; 1];
    if file.read(&mut extra)? != 0 || Stamp::read(&file)? != before {
        return Err(invalid("remote public asset changed while loading"));
    }
    anchor.check()?;
    Ok(bytes)
}

async fn load_bundle(
    tasks: &TaskSupervisor,
    root: PathBuf,
    limits: Limits,
) -> AppResult<AssetResolver> {
    tasks
        .run_blocking_result("native-remote-bundle", move || {
            if limits.count == 0 || limits.payload_bytes == 0 {
                return Err(invalid("remote public asset quota is invalid or exceeded"));
            }
            validate_root(&root)?;
            let anchor = Anchor::open(root)?;
            let manifest: BundleManifest = serde_json::from_slice(&read_public_file(
                &anchor,
                BUNDLE_MANIFEST,
                MANIFEST_BYTES,
            )?)
            .map_err(|_| invalid("native remote bundle manifest is invalid"))?;
            validate_manifest(&manifest.files, limits)?;
            if manifest.format != BUNDLE_FORMAT
                || !manifest.entry_wasm.ends_with(".wasm")
                || !manifest.files.contains(&manifest.entry_wasm)
                || manifest.files.iter().any(|name| name == BUNDLE_MANIFEST)
            {
                return Err(invalid("native remote bundle contract is invalid"));
            }
            let entries = read_entries(&anchor, manifest.files, limits)?;
            if !entries
                .get(&manifest.entry_wasm)
                .is_some_and(|asset| asset.bytes.starts_with(WASM_HEADER))
            {
                return Err(invalid("native remote entry is not a WebAssembly module"));
            }
            let resolver: AssetResolver = Arc::new(move |name| {
                entries.get(name).map(|asset| Asset {
                    mime: asset.mime.clone(),
                    bytes: asset.bytes.clone(),
                })
            });
            Ok(resolver)
        })
        .await
}

fn validate(name: &str) -> AppResult<()> {
    if name.contains(['\0', '\\', ':', '?', '#']) {
        return Err(invalid("remote public asset name is invalid"));
    }
    for part in name.split('/') {
        let lowered = part.to_ascii_lowercase();
        if part.is_empty()
            || part.starts_with('.')
            || lowered == "secrets"
            || lowered.starts_with("id_rsa")
            || lowered.starts_with("id_ed25519")
        {
            return Err(invalid("remote public asset name is not public"));
        }
    }
    Ok(())
}

fn mime(name: &str) -> AppResult<&'static str> {
    let path = Path::new(name);
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("html" | "htm") => Ok("text/html"),
        Some("js" | "mjs") => Ok("text/javascript"),
        Some("json") => Ok("application/json"),
        Some("wasm") => Ok("application/wasm"),
        _ => {
            let mime = extension_mime(path);
            if mime == BINARY_MIME {
                return Err(invalid("remote public asset type is not supported"));
            }
            Ok(mime)
        }
    }
}

fn invalid(message: &str) -> AppError {
    AppError::InvalidArgument(message.into())
}

#[cfg(all(test, unix))]
#[path = "remote-assets-tests.rs"]
mod tests;

#[cfg(all(test, unix))]
#[path = "remote-assets-prepare-tests.rs"]
mod prepare_tests;
