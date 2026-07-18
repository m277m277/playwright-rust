//! Opt-in test fakes for browser APIs Playwright cannot drive natively.
//!
//! The File System Access API (`window.showSaveFilePicker` /
//! `showOpenFilePicker`) opens native OS dialogs with no DOM presence, so no
//! locator or dialog handler can reach them. The standard cross-binding
//! pattern is to install a deterministic fake before the app's JS runs;
//! [`FakeFileSystem`] packages that pattern so consumers don't hand-roll it.
//!
//! Nothing here is installed unless a test asks for it: a page that never
//! calls [`Page::fake_file_system`](crate::protocol::Page::fake_file_system)
//! keeps the browser's real (or absent) picker functions, so
//! feature-detection and fallback paths stay testable.
//!
//! # Example
//!
//! ```no_run
//! # use playwright_rs::Playwright;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let pw = Playwright::launch().await?;
//! # let page = pw.chromium().launch().await?.new_page().await?;
//! let fs = page.fake_file_system().await?;
//!
//! // Seed a file the app's Open dialog will "pick":
//! fs.set_open_file("plan.json", br#"{"rooms": []}"#).await?;
//!
//! // ... drive the app's Save As flow, then assert what it wrote:
//! page.goto("https://localhost:8080", None).await?;
//! let saved = fs.last_saved_bytes().await?;
//! assert!(saved.is_some());
//! # Ok(())
//! # }
//! ```

use crate::error::Result;
use crate::protocol::Page;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

/// The JS shim. Installed both as an init script (so it survives
/// navigations) and evaluated immediately (so it works on the current
/// document). All state lives on `window.__pwRsFakeFs`; the guard on the
/// first line makes double-installation a no-op.
const SHIM: &str = r#"(() => {
    if (window.__pwRsFakeFs) return;
    const state = {
        saves: [],            // { name, b64 }
        openFiles: new Map(), // name -> b64
        permission: 'granted',
    };
    const b64encode = (bytes) => {
        let s = '';
        bytes.forEach((b) => { s += String.fromCharCode(b); });
        return btoa(s);
    };
    const b64decode = (b64) => Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    const toBytes = async (data) => {
        if (typeof data === 'string') return new TextEncoder().encode(data);
        if (data instanceof Blob) return new Uint8Array(await data.arrayBuffer());
        if (data instanceof ArrayBuffer) return new Uint8Array(data);
        if (ArrayBuffer.isView(data)) {
            return new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
        }
        // FileSystemWriteChunkType object form: { type: 'write', data }
        if (data && data.type === 'write') return toBytes(data.data);
        throw new TypeError('fake fs: unsupported write payload');
    };
    const makeHandle = (name) => ({
        kind: 'file',
        name,
        isSameEntry: async (other) => !!other && other.name === name,
        queryPermission: async () => state.permission,
        requestPermission: async () => {
            if (state.permission === 'prompt') state.permission = 'granted';
            return state.permission;
        },
        getFile: async () => {
            const b64 = state.openFiles.get(name) ?? '';
            return new File([b64decode(b64)], name);
        },
        createWritable: async () => {
            const chunks = [];
            return {
                write: async (data) => { chunks.push(await toBytes(data)); },
                seek: async () => {},
                truncate: async () => {},
                abort: async () => {},
                close: async () => {
                    let total = 0;
                    chunks.forEach((c) => { total += c.length; });
                    const all = new Uint8Array(total);
                    let offset = 0;
                    chunks.forEach((c) => { all.set(c, offset); offset += c.length; });
                    const b64 = b64encode(all);
                    state.saves.push({ name, b64 });
                    state.openFiles.set(name, b64);
                },
            };
        },
    });
    window.__pwRsFakeFs = {
        lastSaved: () => (state.saves.length ? state.saves[state.saves.length - 1] : null),
        setOpenFile: (name, b64) => { state.openFiles.set(name, b64); },
        setPermission: (p) => { state.permission = p; },
    };
    window.showSaveFilePicker = async (options) => {
        if (state.permission === 'denied') {
            throw new DOMException('fake fs: permission denied', 'NotAllowedError');
        }
        return makeHandle((options && options.suggestedName) || 'untitled');
    };
    window.showOpenFilePicker = async () => {
        if (state.permission === 'denied') {
            throw new DOMException('fake fs: permission denied', 'NotAllowedError');
        }
        const names = [...state.openFiles.keys()];
        if (names.length === 0) {
            throw new DOMException('fake fs: no open file seeded', 'AbortError');
        }
        return [makeHandle(names[names.length - 1])];
    };
})()"#;

/// Handle to the fake File System Access API installed on a [`Page`] by
/// [`Page::fake_file_system`](crate::protocol::Page::fake_file_system).
///
/// See the [module docs](self) for the pattern and a usage example. Scope of
/// the fake: `showSaveFilePicker`, `showOpenFilePicker`, per-handle
/// `getFile`/`createWritable`/`queryPermission`/`requestPermission`. The fake
/// lives in page JS, so its state does not survive a full page reload; seed
/// again after navigating. `showDirectoryPicker` is not faked.
#[derive(Debug, Clone)]
pub struct FakeFileSystem {
    page: Page,
}

impl FakeFileSystem {
    /// Install the fake on `page` (init script + current document).
    pub(crate) async fn install(page: &Page) -> Result<Self> {
        page.add_init_script(SHIM).await?;
        page.evaluate_expression(SHIM).await?;
        Ok(Self { page: page.clone() })
    }

    /// The file name passed to the most recent completed save, or `None` if
    /// nothing has been saved.
    ///
    /// # Errors
    ///
    /// Returns an error if the page is closed or the evaluation fails.
    pub async fn last_saved_name(&self) -> Result<Option<String>> {
        self.page
            .evaluate(
                "() => { const s = window.__pwRsFakeFs.lastSaved(); return s ? s.name : null; }",
                None::<&()>,
            )
            .await
    }

    /// The bytes written by the most recent completed save (everything
    /// written between `createWritable()` and `close()`), or `None` if
    /// nothing has been saved.
    ///
    /// # Errors
    ///
    /// Returns an error if the page is closed, the evaluation fails, or the
    /// shim returns malformed base64 (which indicates a bug in the shim, not
    /// the caller).
    pub async fn last_saved_bytes(&self) -> Result<Option<Vec<u8>>> {
        let b64: Option<String> = self
            .page
            .evaluate(
                "() => { const s = window.__pwRsFakeFs.lastSaved(); return s ? s.b64 : null; }",
                None::<&()>,
            )
            .await?;
        b64.map(|s| {
            BASE64
                .decode(s)
                .map_err(|e| crate::error::Error::ProtocolError(format!("fake fs base64: {e}")))
        })
        .transpose()
    }

    /// Seed a file that the app's next `showOpenFilePicker()` call will
    /// "pick". Seeding the same name again replaces the content.
    ///
    /// # Errors
    ///
    /// Returns an error if the page is closed or the evaluation fails.
    pub async fn set_open_file(&self, name: &str, bytes: &[u8]) -> Result<()> {
        let arg = (name, BASE64.encode(bytes));
        let _: Option<()> = self
            .page
            .evaluate(
                "([name, b64]) => { window.__pwRsFakeFs.setOpenFile(name, b64); }",
                Some(&arg),
            )
            .await?;
        Ok(())
    }

    /// Set the permission state reported by the fake handles'
    /// `queryPermission` / `requestPermission`: `"granted"`, `"prompt"`, or
    /// `"denied"`. Defaults to `"granted"`. In the `"prompt"` state,
    /// `requestPermission` upgrades to `"granted"`, mirroring the real API's
    /// user-approval path; in `"denied"`, the pickers throw
    /// `NotAllowedError`.
    ///
    /// # Errors
    ///
    /// Returns an error if the page is closed or the evaluation fails.
    pub async fn set_permission(&self, state: &str) -> Result<()> {
        let _: Option<()> = self
            .page
            .evaluate(
                "(state) => { window.__pwRsFakeFs.setPermission(state); }",
                Some(&state),
            )
            .await?;
        Ok(())
    }

    /// Shorthand for [`set_permission("granted")`](Self::set_permission).
    ///
    /// # Errors
    ///
    /// Returns an error if the page is closed or the evaluation fails.
    pub async fn grant_permission(&self) -> Result<()> {
        self.set_permission("granted").await
    }
}
