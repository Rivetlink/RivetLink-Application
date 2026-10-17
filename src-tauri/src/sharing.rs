//! Clipboard and file sharing with a host during a live LAN stream.
//!
//! Only active when the host listed the sharing features for this client and
//! answered `EnableSharing`. Everything travels over the stream's existing
//! sealed channel: requests through its input sender, host responses through
//! [`Sharing::handle`].
//!
//! - Clipboard: a dedicated thread owns the OS clipboard, polls it for local
//!   copies and applies text from the host; it remembers the last text in
//!   either direction so nothing echoes back and forth.
//! - Files: uploads read local files/folders in chunks; downloads land in the
//!   OS Downloads folder under `RivetLink/`, one at a time, never overwriting.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine;
use rivetlink_sdk::lan::{LanRequest, LanResponse};
use rivetlink_sdk::share::{self, FILE_CHUNK_BYTES, MAX_CLIPBOARD_BYTES};
use tauri::{Emitter, Manager};
use tokio::sync::mpsc::Sender;

const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;
/// How often the local clipboard is checked for a new copy.
const CLIPBOARD_POLL: Duration = Duration::from_millis(500);
/// Keep this many input-queue slots free while uploading, so pointer and key
/// events still get through during a large transfer.
const INPUT_HEADROOM: usize = 128;
const PART_SUFFIX: &str = ".rivetlink-part";

/// Progress of one transfer, emitted as `lan://transfer`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TransferEvent {
    id: u32,
    name: String,
    direction: &'static str,
    bytes: u64,
    total: u64,
    /// "active", "done" or "error".
    state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Debug)]
struct Download {
    name: String,
    part: PathBuf,
    dir: PathBuf,
    file_name: String,
    file: std::fs::File,
    received: u64,
}

#[derive(Debug, Default)]
struct Inner {
    tx: Mutex<Option<Sender<LanRequest>>>,
    clipboard: Mutex<Option<ClipboardSync>>,
    /// Uploads the host rejected; their remaining chunks are not sent.
    failed_uploads: Mutex<BTreeSet<u32>>,
    upload_names: Mutex<BTreeMap<u32, (String, u64)>>,
    downloads: Mutex<BTreeMap<u32, Download>>,
    /// Downloads waiting for the active one to finish (host path, local root).
    queued_downloads: Mutex<VecDeque<(u32, String, PathBuf)>>,
    next_id: AtomicU32,
}

/// Per-app sharing state; cheap to clone into the stream task.
#[derive(Debug, Clone, Default)]
pub struct Sharing {
    inner: Arc<Inner>,
}

impl Sharing {
    /// Remember the stream's request sender for a new session.
    pub fn attach(&self, tx: Sender<LanRequest>) {
        self.reset();
        if let Ok(mut guard) = self.inner.tx.lock() {
            *guard = Some(tx);
        }
    }

    /// Stop clipboard sync and drop unfinished downloads (stream ended).
    pub fn reset(&self) {
        if let Ok(mut guard) = self.inner.tx.lock() {
            *guard = None;
        }
        if let Ok(mut guard) = self.inner.clipboard.lock() {
            *guard = None;
        }
        if let Ok(mut downloads) = self.inner.downloads.lock() {
            for (_, download) in std::mem::take(&mut *downloads) {
                let _ = std::fs::remove_file(&download.part);
            }
        }
        if let Ok(mut queue) = self.inner.queued_downloads.lock() {
            queue.clear();
        }
        if let Ok(mut failed) = self.inner.failed_uploads.lock() {
            failed.clear();
        }
        if let Ok(mut names) = self.inner.upload_names.lock() {
            names.clear();
        }
    }

    fn sender(&self) -> Option<Sender<LanRequest>> {
        self.inner.tx.lock().ok().and_then(|guard| guard.clone())
    }

    fn next_id(&self) -> u32 {
        self.inner
            .next_id
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_add(1)
    }

    /// Handle one sharing response from the host. Runs on the stream task.
    pub fn handle(&self, app: &tauri::AppHandle, response: LanResponse) {
        match response {
            LanResponse::SharingEnabled { clipboard, files } => {
                if clipboard {
                    if let Some(tx) = self.sender() {
                        if let Ok(mut guard) = self.inner.clipboard.lock() {
                            *guard = ClipboardSync::start(tx);
                        }
                    }
                }
                let _ = app.emit(
                    "lan://sharing",
                    serde_json::json!({ "clipboard": clipboard, "files": files }),
                );
            }
            LanResponse::ClipboardText { text } => {
                if let Ok(guard) = self.inner.clipboard.lock() {
                    if let Some(sync) = guard.as_ref() {
                        sync.apply(text);
                        let _ = app.emit("lan://clipboard", ());
                    }
                }
            }
            LanResponse::FileUploaded { id, path } => {
                let (_, total) = self.upload_name(id);
                let _ = app.emit(
                    "lan://transfer",
                    TransferEvent {
                        id,
                        name: path,
                        direction: "up",
                        bytes: total,
                        total,
                        state: "done",
                        message: None,
                    },
                );
            }
            LanResponse::SharedFiles { entries } => {
                let _ = app.emit("lan://shared-files", entries);
            }
            LanResponse::FileChunk {
                id,
                size,
                data_b64,
                eof,
            } => self.download_chunk(app, id, size, &data_b64, eof),
            LanResponse::FileError { id, message } => {
                if let Ok(mut failed) = self.inner.failed_uploads.lock() {
                    failed.insert(id);
                }
                if let Some(download) = self.take_download(id) {
                    let _ = std::fs::remove_file(&download.part);
                    emit_error(app, id, download.name, "down", &message);
                    self.start_next_download(app);
                } else {
                    emit_error(app, id, self.upload_name(id).0, "up", &message);
                }
            }
            _ => {}
        }
    }

    fn upload_name(&self, id: u32) -> (String, u64) {
        self.inner
            .upload_names
            .lock()
            .ok()
            .and_then(|names| names.get(&id).cloned())
            .unwrap_or_default()
    }

    fn take_download(&self, id: u32) -> Option<Download> {
        self.inner.downloads.lock().ok()?.remove(&id)
    }

    fn download_chunk(
        &self,
        app: &tauri::AppHandle,
        id: u32,
        size: u64,
        data_b64: &str,
        eof: bool,
    ) {
        let Some(mut download) = self.take_download(id) else {
            return;
        };
        let written = B64
            .decode(data_b64)
            .map_err(|_| "the host sent invalid data".to_string())
            .and_then(|data| {
                download
                    .file
                    .write_all(&data)
                    .map(|()| data.len() as u64)
                    .map_err(|e| e.to_string())
            });
        match written {
            Ok(length) => {
                download.received += length;
                if eof {
                    let result = finish_download(&download);
                    match result {
                        Ok(()) => {
                            let _ = app.emit(
                                "lan://transfer",
                                TransferEvent {
                                    id,
                                    name: download.name.clone(),
                                    direction: "down",
                                    bytes: download.received,
                                    total: size,
                                    state: "done",
                                    message: None,
                                },
                            );
                        }
                        Err(message) => {
                            let _ = std::fs::remove_file(&download.part);
                            emit_error(app, id, download.name.clone(), "down", &message);
                        }
                    }
                    self.start_next_download(app);
                } else {
                    let _ = app.emit(
                        "lan://transfer",
                        TransferEvent {
                            id,
                            name: download.name.clone(),
                            direction: "down",
                            bytes: download.received,
                            total: size,
                            state: "active",
                            message: None,
                        },
                    );
                    if let Ok(mut downloads) = self.inner.downloads.lock() {
                        downloads.insert(id, download);
                    }
                }
            }
            Err(message) => {
                let _ = std::fs::remove_file(&download.part);
                emit_error(app, id, download.name.clone(), "down", &message);
                self.start_next_download(app);
            }
        }
    }

    /// Ask the host for its shared-folder listing (answered as `lan://shared-files`).
    pub async fn list(&self) -> Result<(), String> {
        let tx = self.sender().ok_or("not connected")?;
        tx.send(LanRequest::ListSharedFiles)
            .await
            .map_err(|_| "the connection closed".to_string())
    }

    /// Queue downloads of host files (relative paths from the listing) into
    /// `<Downloads>/RivetLink`, keeping their folder structure.
    pub fn download(&self, app: &tauri::AppHandle, paths: Vec<String>) -> Result<(), String> {
        let root = app
            .path()
            .download_dir()
            .map_err(|e| e.to_string())?
            .join("RivetLink");
        if let Ok(mut queue) = self.inner.queued_downloads.lock() {
            for path in paths {
                if share::relative_components(&path).is_some() {
                    queue.push_back((self.next_id(), path, root.clone()));
                }
            }
        }
        let idle = self
            .inner
            .downloads
            .lock()
            .map(|d| d.is_empty())
            .unwrap_or(false);
        if idle {
            self.start_next_download(app);
        }
        Ok(())
    }

    fn start_next_download(&self, app: &tauri::AppHandle) {
        loop {
            let Some((id, path, root)) = self
                .inner
                .queued_downloads
                .lock()
                .ok()
                .and_then(|mut queue| queue.pop_front())
            else {
                return;
            };
            match open_download(&root, &path) {
                Ok(download) => {
                    let Some(tx) = self.sender() else { return };
                    if let Ok(mut downloads) = self.inner.downloads.lock() {
                        downloads.insert(id, download);
                    }
                    tauri::async_runtime::spawn(async move {
                        let _ = tx.send(LanRequest::FileDownload { id, path }).await;
                    });
                    return;
                }
                Err(message) => emit_error(app, id, path, "down", &message),
            }
        }
    }

    /// Upload local files and folders (recursively) into the host's shared
    /// folder. Runs until every file is sent; progress arrives as events.
    pub async fn upload(&self, app: tauri::AppHandle, paths: Vec<PathBuf>) -> Result<(), String> {
        let tx = self.sender().ok_or("not connected")?;
        let mut files = Vec::new();
        for path in paths {
            collect_upload(&path, None, &mut files);
        }
        for (local, relative) in files {
            let id = self.next_id();
            if let Err(message) = self.upload_one(&app, &tx, id, &local, &relative).await {
                emit_error(&app, id, relative, "up", &message);
            }
        }
        Ok(())
    }

    async fn upload_one(
        &self,
        app: &tauri::AppHandle,
        tx: &Sender<LanRequest>,
        id: u32,
        local: &Path,
        relative: &str,
    ) -> Result<(), String> {
        if share::relative_components(relative).is_none() {
            return Err("this file name cannot be sent".to_string());
        }
        let mut file = std::fs::File::open(local).map_err(|e| e.to_string())?;
        let size = file.metadata().map_err(|e| e.to_string())?.len();
        if size > share::MAX_FILE_BYTES {
            return Err("this file is too large".to_string());
        }
        if let Ok(mut names) = self.inner.upload_names.lock() {
            names.insert(id, (relative.to_string(), size));
        }
        let closed = || "the connection closed".to_string();
        tx.send(LanRequest::FileUploadStart {
            id,
            path: relative.to_string(),
            size,
        })
        .await
        .map_err(|_| closed())?;
        let mut sent = 0_u64;
        let mut buffer = vec![0_u8; FILE_CHUNK_BYTES];
        loop {
            if self.upload_failed(id) {
                // The host already reported why; it shows as a transfer error.
                return Ok(());
            }
            let read = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if read == 0 {
                break;
            }
            while tx.capacity() < INPUT_HEADROOM {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            tx.send(LanRequest::FileUploadChunk {
                id,
                data_b64: B64.encode(&buffer[..read]),
            })
            .await
            .map_err(|_| closed())?;
            sent += read as u64;
            let _ = app.emit(
                "lan://transfer",
                TransferEvent {
                    id,
                    name: relative.to_string(),
                    direction: "up",
                    bytes: sent,
                    total: size,
                    state: "active",
                    message: None,
                },
            );
        }
        tx.send(LanRequest::FileUploadEnd { id })
            .await
            .map_err(|_| closed())
    }

    fn upload_failed(&self, id: u32) -> bool {
        self.inner
            .failed_uploads
            .lock()
            .map(|failed| failed.contains(&id))
            .unwrap_or(false)
    }
}

fn emit_error(
    app: &tauri::AppHandle,
    id: u32,
    name: String,
    direction: &'static str,
    message: &str,
) {
    let _ = app.emit(
        "lan://transfer",
        TransferEvent {
            id,
            name,
            direction,
            bytes: 0,
            total: 0,
            state: "error",
            message: Some(message.to_string()),
        },
    );
}

/// Expand `path` into (local file, relative upload path) pairs. Folders keep
/// their own name as the top-level folder; symlinks are skipped.
fn collect_upload(path: &Path, prefix: Option<&str>, out: &mut Vec<(PathBuf, String)>) {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return;
    };
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return;
    };
    let relative = match prefix {
        Some(prefix) => format!("{prefix}/{name}"),
        None => name.to_string(),
    };
    if relative.split('/').count() > share::MAX_PATH_DEPTH {
        return;
    }
    if meta.is_file() {
        out.push((path.to_path_buf(), relative));
    } else if meta.is_dir() {
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        let mut children: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        children.sort();
        for child in children {
            collect_upload(&child, Some(&relative), out);
        }
    }
}

/// Create the folders for `relative` under `root` and open a `.part` file.
fn open_download(root: &Path, relative: &str) -> Result<Download, String> {
    let components = share::relative_components(relative).ok_or("invalid file name")?;
    let (file_name, folders) = components.split_last().ok_or("invalid file name")?;
    let mut dir = root.to_path_buf();
    for folder in folders {
        dir.push(folder);
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let part = dir.join(format!(".{file_name}{PART_SUFFIX}"));
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&part)
        .map_err(|e| e.to_string())?;
    Ok(Download {
        name: relative.to_string(),
        part,
        dir,
        file_name: (*file_name).to_string(),
        file,
        received: 0,
    })
}

/// Move a completed `.part` file to the first unused name.
fn finish_download(download: &Download) -> Result<(), String> {
    download.file.sync_all().map_err(|e| e.to_string())?;
    let (stem, extension) = match download.file_name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => (stem, Some(ext)),
        _ => (download.file_name.as_str(), None),
    };
    let target = (0..10_000)
        .map(|n| match (n, extension) {
            (0, _) => download.file_name.clone(),
            (_, Some(ext)) => format!("{stem} ({n}).{ext}"),
            (_, None) => format!("{stem} ({n})"),
        })
        .map(|name| download.dir.join(name))
        .find(|path| std::fs::symlink_metadata(path).is_err())
        .ok_or("no free file name")?;
    std::fs::rename(&download.part, target).map_err(|e| e.to_string())
}

/// Owns the OS clipboard on its own thread (it is not `Send` everywhere).
#[derive(Debug)]
struct ClipboardSync {
    incoming: std::sync::mpsc::Sender<String>,
    stop: Arc<AtomicBool>,
}

impl ClipboardSync {
    fn start(tx: Sender<LanRequest>) -> Option<Self> {
        let (incoming, from_host) = std::sync::mpsc::channel::<String>();
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        std::thread::Builder::new()
            .name("rivetlink-clipboard".to_string())
            .spawn(move || clipboard_loop(&tx, &from_host, &thread_stop))
            .ok()?;
        Some(Self { incoming, stop })
    }

    fn apply(&self, text: String) {
        let _ = self.incoming.send(text);
    }
}

impl Drop for ClipboardSync {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn clipboard_loop(
    tx: &Sender<LanRequest>,
    from_host: &std::sync::mpsc::Receiver<String>,
    stop: &AtomicBool,
) {
    let Ok(mut clipboard) = arboard::Clipboard::new() else {
        tracing::warn!("clipboard sharing: the local clipboard is unavailable");
        return;
    };
    // Whatever was copied before connecting stays local; only new copies go.
    let mut last = clipboard.get_text().ok();
    while !stop.load(Ordering::Relaxed) {
        while let Ok(text) = from_host.try_recv() {
            if clipboard.set_text(text.clone()).is_ok() {
                last = Some(text);
            }
        }
        if let Ok(text) = clipboard.get_text() {
            if !text.is_empty() && last.as_deref() != Some(text.as_str()) {
                last = Some(text.clone());
                if text.len() <= MAX_CLIPBOARD_BYTES
                    && tx.blocking_send(LanRequest::ClipboardSet { text }).is_err()
                {
                    return;
                }
            }
        }
        std::thread::sleep(CLIPBOARD_POLL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_folders_recursively_with_their_own_name() {
        let root = std::env::temp_dir().join(format!("rivet-upload-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("Photos/2026")).unwrap();
        std::fs::write(root.join("Photos/a.jpg"), b"a").unwrap();
        std::fs::write(root.join("Photos/2026/b.jpg"), b"b").unwrap();
        let mut out = Vec::new();
        collect_upload(&root.join("Photos"), None, &mut out);
        let relative: Vec<_> = out.iter().map(|(_, r)| r.as_str()).collect();
        assert_eq!(relative, ["Photos/2026/b.jpg", "Photos/a.jpg"]);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn downloads_never_overwrite_existing_files() {
        let root = std::env::temp_dir().join(format!("rivet-download-{}", uuid::Uuid::new_v4()));
        for expected in ["Docs/report.pdf", "Docs/report (1).pdf"] {
            let mut download = open_download(&root, "Docs/report.pdf").unwrap();
            download.file.write_all(b"pdf").unwrap();
            finish_download(&download).unwrap();
            assert!(root.join(expected).is_file(), "{expected} should exist");
        }
        assert!(open_download(&root, "../escape").is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}
