//! Turns a presentation file into PNG slides on disk.
//!
//! * PDF is rasterized with PDFium (the engine behind Chrome's PDF viewer).
//! * PPTX/PPT/ODP/Key are converted to PDF by LibreOffice first.
//!
//! Results are cached under a key derived from the file's path, size, mtime
//! and render width, so reopening a deck is instant. A deck is rendered into a
//! temporary directory and renamed into place only when complete: an
//! interrupted render never leaves a half-filled cache entry behind.

use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock, mpsc};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use pdfium_render::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::i18n::tr;

/// Bumped whenever the on-disk layout or rendering changes, invalidating caches.
const CACHE_FORMAT: u32 = 2;
/// How many rendered decks to keep in the cache.
const CACHE_KEEP: usize = 12;
/// Width of the overview thumbnails, px.
const THUMB_WIDTH: u32 = 480;
const MANIFEST: &str = "manifest.json";
const CONVERSION_TIMEOUT: Duration = Duration::from_secs(180);

/// File extensions LibreOffice converts to PDF for us.
pub const OFFICE_EXTENSIONS: [&str; 4] = ["pptx", "ppt", "odp", "key"];

/// Rendering progress, streamed to the start screen.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(tag = "stage", rename_all = "camelCase")]
pub enum Progress {
    Converting,
    Rendering { done: u32, total: u32 },
}

/// A deck rendered to the cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rendered {
    /// Cache key; also part of slide URLs, so a new deck never shows stale images.
    pub id: String,
    pub count: u32,
    /// Width / height of the first page.
    pub aspect: f32,
}

/// Which image of a slide to fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    Full,
    Thumb,
}

pub struct Renderer {
    cache_root: PathBuf,
    /// Directories searched for the PDFium shared library, in order.
    pdfium_dirs: Vec<PathBuf>,
}

impl Renderer {
    pub fn new(cache_root: PathBuf, pdfium_dirs: Vec<PathBuf>) -> Self {
        Self {
            cache_root,
            pdfium_dirs,
        }
    }

    /// Path of a rendered slide image; `None` for malformed ids.
    pub fn slide_path(&self, id: &str, slide: u32, size: Size) -> Option<PathBuf> {
        let valid_id = id.len() == 16 && id.bytes().all(|b| b.is_ascii_hexdigit());
        valid_id.then(|| self.cache_root.join(id).join(slide_file(slide, size)))
    }

    /// Renders `deck` at `width` px, reusing the cache when possible.
    pub fn render(
        &self,
        deck: &Path,
        width: u32,
        progress: &(dyn Fn(Progress) + Sync),
    ) -> Result<Rendered> {
        let id = cache_key(deck, width)?;
        let dir = self.cache_root.join(&id);
        if let Some(rendered) = read_manifest(&dir) {
            // Mark as recently used so pruning keeps it.
            let _ =
                fs::File::open(dir.join(MANIFEST)).and_then(|f| f.set_modified(SystemTime::now()));
            return Ok(rendered);
        }

        let partial = self.cache_root.join(format!("{id}.partial"));
        let _ = fs::remove_dir_all(&partial);
        fs::create_dir_all(&partial).map_err(|e| Error::io(&partial, e))?;

        let result = self.render_into(deck, width, &id, &partial, progress);
        let rendered = match result {
            Ok(rendered) => rendered,
            Err(e) => {
                let _ = fs::remove_dir_all(&partial);
                return Err(e);
            }
        };

        let manifest = serde_json::to_vec(&rendered).expect("manifest is serializable");
        fs::write(partial.join(MANIFEST), manifest).map_err(|e| Error::io(&partial, e))?;
        let _ = fs::remove_dir_all(&dir);
        fs::rename(&partial, &dir).map_err(|e| Error::io(&dir, e))?;
        self.prune();
        Ok(rendered)
    }

    fn render_into(
        &self,
        deck: &Path,
        width: u32,
        id: &str,
        out: &Path,
        progress: &(dyn Fn(Progress) + Sync),
    ) -> Result<Rendered> {
        let is_office = deck
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| OFFICE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()));
        let pdf = if is_office {
            progress(Progress::Converting);
            self.convert_to_pdf(deck, out)?
        } else {
            deck.to_path_buf()
        };

        let pdfium = self.pdfium()?;
        let document = pdfium
            .load_pdf_from_file(&pdf, None)
            .map_err(|e| Error::Pdf(e.to_string()))?;
        let pages = document.pages();
        let count = u32::try_from(pages.len()).unwrap_or_default();
        if count == 0 {
            return Err(Error::EmptyDeck);
        }
        let aspect = pages
            .first()
            .map(|p| p.width().value / p.height().value)
            .ok()
            .filter(|a| a.is_finite() && *a > 0.0)
            .unwrap_or(16.0 / 9.0);

        // PDFium renders on this thread; PNG encoding (the slower part) runs on
        // a small worker pool fed through a bounded channel to cap memory use.
        let workers = thread::available_parallelism().map_or(2, |n| n.get().clamp(2, 6));
        let (tx, rx) = mpsc::sync_channel::<Frame>(workers * 2);
        let rx = Mutex::new(rx);
        let done = AtomicU32::new(0);
        progress(Progress::Rendering {
            done: 0,
            total: count,
        });

        thread::scope(|scope| -> Result<()> {
            let handles: Vec<_> = (0..workers)
                .map(|_| {
                    scope.spawn(|| -> Result<()> {
                        loop {
                            // The lock is held only while waiting for the next frame.
                            let frame = rx.lock().expect("receiver lock").recv();
                            let Ok(frame) = frame else { return Ok(()) };
                            let path = out.join(slide_file(frame.slide, frame.size));
                            write_png(&path, &frame)?;
                            if frame.size == Size::Full {
                                let done = done.fetch_add(1, Ordering::Relaxed) + 1;
                                progress(Progress::Rendering { done, total: count });
                            }
                        }
                    })
                })
                .collect();

            let rendered = (|| -> Result<()> {
                for (index, page) in pages.iter().enumerate() {
                    let slide = u32::try_from(index).expect("page count fits u32") + 1;
                    for (size, w) in [(Size::Full, width), (Size::Thumb, THUMB_WIDTH)] {
                        let frame = rasterize(&page, slide, size, w)?;
                        if tx.send(frame).is_err() {
                            return Ok(()); // a worker failed; its error is reported below
                        }
                    }
                }
                Ok(())
            })();
            drop(tx);

            let mut result = rendered;
            for handle in handles {
                let worker = handle.join().expect("encoder thread panicked");
                result = result.and(worker);
            }
            result
        })?;

        if is_office {
            let _ = fs::remove_file(&pdf);
        }
        Ok(Rendered {
            id: id.to_owned(),
            count,
            aspect,
        })
    }

    /// Binds PDFium once per process (the bindings are global in pdfium-render).
    fn pdfium(&self) -> Result<&'static Pdfium> {
        static PDFIUM: OnceLock<Pdfium> = OnceLock::new();
        static INIT: Mutex<()> = Mutex::new(());

        let _guard = INIT.lock().expect("pdfium init lock");
        if let Some(pdfium) = PDFIUM.get() {
            return Ok(pdfium);
        }
        let bindings = self
            .pdfium_dirs
            .iter()
            .find_map(|dir| {
                Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(dir)).ok()
            })
            .map_or_else(Pdfium::bind_to_system_library, Ok)
            .map_err(|e| {
                log::error!("PDFium not found in {:?}: {e}", self.pdfium_dirs);
                Error::NoPdfium
            })?;
        Ok(PDFIUM.get_or_init(|| Pdfium::new(bindings)))
    }

    /// Converts an office document to PDF with a private LibreOffice profile,
    /// so it works even while the user has LibreOffice open.
    fn convert_to_pdf(&self, deck: &Path, out: &Path) -> Result<PathBuf> {
        let soffice = find_soffice().ok_or(Error::NoLibreOffice)?;
        let profile = self.cache_root.join("libreoffice-profile");
        let profile_url = tauri::Url::from_directory_path(&profile).map_err(|()| {
            Error::Conversion(tr!("некорректный путь к кэшу", "invalid cache path"))
        })?;

        log::info!("converting {} with {}", deck.display(), soffice.display());
        let mut child = Command::new(&soffice)
            .arg(format!("-env:UserInstallation={profile_url}"))
            .args([
                "--headless",
                "--norestore",
                "--convert-to",
                "pdf",
                "--outdir",
            ])
            .arg(out)
            .arg(deck)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::Conversion(e.to_string()))?;

        let started = Instant::now();
        let status = loop {
            if let Some(status) = child
                .try_wait()
                .map_err(|e| Error::Conversion(e.to_string()))?
            {
                break status;
            }
            if started.elapsed() > CONVERSION_TIMEOUT {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Error::Conversion(tr!(
                    "превышено время ожидания",
                    "timed out"
                )));
            }
            thread::sleep(Duration::from_millis(100));
        };

        let pdf = out
            .join(deck.file_stem().unwrap_or_default())
            .with_extension("pdf");
        if status.success() && pdf.is_file() {
            return Ok(pdf);
        }
        let stderr = child.stderr.take().and_then(|mut s| {
            let mut text = String::new();
            std::io::Read::read_to_string(&mut s, &mut text)
                .ok()
                .map(|_| text)
        });
        let detail = stderr
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty());
        Err(Error::Conversion(detail.unwrap_or_else(|| {
            tr!("код выхода {status}", "exit status {status}")
        })))
    }

    /// Removes all but the most recently used cache entries.
    fn prune(&self) {
        let Ok(entries) = fs::read_dir(&self.cache_root) else {
            return;
        };
        let mut decks: Vec<(SystemTime, PathBuf)> = entries
            .filter_map(|e| {
                let path = e.ok()?.path();
                let used = fs::metadata(path.join(MANIFEST))
                    .and_then(|m| m.modified())
                    .ok()?;
                Some((used, path))
            })
            .collect();
        decks.sort_by_key(|(used, _)| std::cmp::Reverse(*used));
        for (_, path) in decks.into_iter().skip(CACHE_KEEP) {
            log::info!("pruning cached deck {}", path.display());
            let _ = fs::remove_dir_all(path);
        }
    }
}

/// A rasterized page waiting to be encoded.
struct Frame {
    slide: u32,
    size: Size,
    width: u32,
    height: u32,
    rgb: Vec<u8>,
}

fn rasterize(page: &PdfPage, slide: u32, size: Size, width: u32) -> Result<Frame> {
    let config = PdfRenderConfig::new()
        .set_target_width(i32::try_from(width).unwrap_or(i32::MAX))
        .render_form_data(true)
        .render_annotations(true);
    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| Error::Pdf(e.to_string()))?;
    let (w, h) = (bitmap.width(), bitmap.height());
    // Slides are opaque: drop alpha, PNGs get a quarter smaller and faster to encode.
    let rgb = bitmap
        .as_rgba_bytes()
        .chunks_exact(4)
        .flat_map(|px| [px[0], px[1], px[2]])
        .collect();
    Ok(Frame {
        slide,
        size,
        width: u32::try_from(w).unwrap_or_default(),
        height: u32::try_from(h).unwrap_or_default(),
        rgb,
    })
}

fn write_png(path: &Path, frame: &Frame) -> Result<()> {
    let file = fs::File::create(path).map_err(|e| Error::io(path, e))?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), frame.width, frame.height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    let to_io = |e: png::EncodingError| Error::io(path, std::io::Error::other(e));
    let mut writer = encoder.write_header().map_err(to_io)?;
    writer.write_image_data(&frame.rgb).map_err(to_io)?;
    writer.finish().map_err(to_io)
}

fn slide_file(slide: u32, size: Size) -> String {
    match size {
        Size::Full => format!("{slide:03}.png"),
        Size::Thumb => format!("thumb-{slide:03}.png"),
    }
}

fn cache_key(deck: &Path, width: u32) -> Result<String> {
    let path = deck.canonicalize().map_err(|e| Error::io(deck, e))?;
    let meta = fs::metadata(&path).map_err(|e| Error::io(deck, e))?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok());

    let mut hasher = blake3::Hasher::new();
    hasher.update(path.as_os_str().as_encoded_bytes());
    hasher.update(&meta.len().to_le_bytes());
    hasher.update(&mtime.unwrap_or_default().as_nanos().to_le_bytes());
    hasher.update(&width.to_le_bytes());
    hasher.update(&CACHE_FORMAT.to_le_bytes());
    Ok(hasher.finalize().to_hex()[..16].to_owned())
}

fn read_manifest(dir: &Path) -> Option<Rendered> {
    serde_json::from_slice(&fs::read(dir.join(MANIFEST)).ok()?).ok()
}

fn find_soffice() -> Option<PathBuf> {
    let in_path = std::env::var_os("PATH")
        .into_iter()
        .flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>());
    let names = if cfg!(windows) {
        &["soffice.exe"][..]
    } else {
        &["soffice", "libreoffice"][..]
    };
    let from_path = in_path.flat_map(|dir| names.iter().map(move |n| dir.join(n)));
    let well_known = [
        "/Applications/LibreOffice.app/Contents/MacOS/soffice",
        r"C:\Program Files\LibreOffice\program\soffice.exe",
    ]
    .map(PathBuf::from);
    from_path.chain(well_known).find(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slide_paths_reject_traversal() {
        let r = Renderer::new(PathBuf::from("/cache"), vec![]);
        assert_eq!(
            r.slide_path("0123456789abcdef", 7, Size::Thumb),
            Some(PathBuf::from("/cache/0123456789abcdef/thumb-007.png"))
        );
        assert_eq!(r.slide_path("../../etc/passwd", 1, Size::Full), None);
        assert_eq!(r.slide_path("0123456789abcdeg", 1, Size::Full), None);
    }
}
