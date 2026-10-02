//! Child-process-only observation of the pinned built-in win32 VFS.
//!
//! These hooks forward the exact arguments/result and preserve GetLastError.
//! They never deny an operation, alter SQLite behaviour, supply a fake result,
//! or substitute a VFS. Include this module only with cfg(all(test, windows)).
//! The caller runs one exact test in a dedicated process, installs before its
//! first source SQLite open, closes all connections before dropping the guard,
//! and retains source seals throughout. No unrelated SQLite work is permitted
//! in that child while hooks are installed.

use rusqlite::ffi;
use std::ffi::{CStr, c_void};
use std::marker::PhantomData;
use std::path::Path;
use std::rc::Rc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use windows_sys::Win32::Foundation::{GetLastError, SetLastError};

const GENERIC_READ: u32 = 0x8000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;
const OPEN_EXISTING: u32 = 3;
const OPEN_ALWAYS: u32 = 4;
const ERROR_SHARING_VIOLATION: u32 = 32;
const PAGE_READONLY: u32 = 2;
const FILE_MAP_READ: u32 = 4;
const SOURCE_ID: &str =
    "2026-06-03 19:12:13 d6e03d8c777cfa2d35e3b60d8ec3e0187f3e9f99d8e2ee9cac695fd6fcdf1a24";

type Handle = *mut c_void;
type CreateFile =
    unsafe extern "system" fn(*const u16, u32, u32, *const c_void, u32, u32, Handle) -> Handle;
type CreateMapping =
    unsafe extern "system" fn(Handle, *const c_void, u32, u32, u32, *const u16) -> Handle;
type MapView = unsafe extern "system" fn(Handle, u32, u32, u32, usize) -> *mut c_void;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static CREATE_FILE: AtomicUsize = AtomicUsize::new(0);
static CREATE_MAPPING: AtomicUsize = AtomicUsize::new(0);
static MAP_VIEW: AtomicUsize = AtomicUsize::new(0);
static EVENTS: Mutex<Vec<Event>> = Mutex::new(Vec::new());

#[derive(Clone, Debug)]
#[allow(dead_code)] // Complete native diagnostics are retained in Debug trace output.
pub(crate) enum Event {
    Open {
        path: String,
        access: u32,
        share: u32,
        disposition: u32,
        flags: u32,
        handle: usize,
        error: u32,
    },
    Mapping {
        file: usize,
        protect: u32,
        mapping: usize,
        error: u32,
    },
    View {
        mapping: usize,
        access: u32,
        address: usize,
        error: u32,
    },
}

fn events() -> std::sync::MutexGuard<'static, Vec<Event>> {
    EVENTS.lock().unwrap_or_else(|poison| poison.into_inner())
}

unsafe extern "system" fn trace_create_file(
    path: *const u16,
    access: u32,
    share: u32,
    security: *const c_void,
    disposition: u32,
    flags: u32,
    template: Handle,
) -> Handle {
    // Installation publishes original callbacks before registering any hook.
    let original: CreateFile = unsafe { std::mem::transmute(CREATE_FILE.load(Ordering::Acquire)) };
    let result = unsafe { original(path, access, share, security, disposition, flags, template) };
    let error = unsafe { GetLastError() };
    let mut length = 0;
    if !path.is_null() {
        while unsafe { *path.add(length) } != 0 {
            length += 1;
        }
    }
    let path = if path.is_null() {
        String::new()
    } else {
        String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(path, length) })
    };
    events().push(Event::Open {
        path,
        access,
        share,
        disposition,
        flags,
        handle: result as usize,
        error,
    });
    unsafe { SetLastError(error) };
    result
}

unsafe extern "system" fn trace_create_mapping(
    file: Handle,
    security: *const c_void,
    protect: u32,
    high: u32,
    low: u32,
    name: *const u16,
) -> Handle {
    let original: CreateMapping =
        unsafe { std::mem::transmute(CREATE_MAPPING.load(Ordering::Acquire)) };
    let result = unsafe { original(file, security, protect, high, low, name) };
    let error = unsafe { GetLastError() };
    events().push(Event::Mapping {
        file: file as usize,
        protect,
        mapping: result as usize,
        error,
    });
    unsafe { SetLastError(error) };
    result
}

unsafe extern "system" fn trace_map_view(
    mapping: Handle,
    access: u32,
    high: u32,
    low: u32,
    bytes: usize,
) -> *mut c_void {
    let original: MapView = unsafe { std::mem::transmute(MAP_VIEW.load(Ordering::Acquire)) };
    let result = unsafe { original(mapping, access, high, low, bytes) };
    let error = unsafe { GetLastError() };
    events().push(Event::View {
        mapping: mapping as usize,
        access,
        address: result as usize,
        error,
    });
    unsafe { SetLastError(error) };
    result
}

/// Not Send/Sync: installation and removal belong to one dedicated child.
pub(crate) struct TraceGuard {
    vfs: *mut ffi::sqlite3_vfs,
    originals: [ffi::sqlite3_syscall_ptr; 3],
    installed: usize,
    _not_send: PhantomData<Rc<()>>,
}

const NAMES: [&CStr; 3] = [c"CreateFileW", c"CreateFileMappingW", c"MapViewOfFile"];

impl TraceGuard {
    pub(crate) fn install_for_current_child() -> Result<Self, String> {
        if std::env::var("MSTRCHF_VFS_TRACE_CHILD").as_deref() != Ok("1") {
            return Err("VFS hooks require dedicated MSTRCHF_VFS_TRACE_CHILD=1 child".into());
        }
        if ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err("another VFS trace is already installed in this process".into());
        }
        let result = unsafe { Self::install() };
        if result.is_err() {
            ACTIVE.store(false, Ordering::Release);
        }
        result
    }

    unsafe fn install() -> Result<Self, String> {
        let version = unsafe { ffi::sqlite3_libversion_number() };
        let source = unsafe { CStr::from_ptr(ffi::sqlite3_sourceid()) }.to_string_lossy();
        if version != 3_053_002 || source != SOURCE_ID {
            return Err(format!(
                "pinned SQLite engine mismatch: {version}, {source}"
            ));
        }
        let vfs = unsafe { ffi::sqlite3_vfs_find(c"win32".as_ptr()) };
        if vfs.is_null() || unsafe { (*vfs).iVersion } < 3 {
            return Err("built-in win32 VFS version 3 unavailable".into());
        }
        let get = unsafe { (*vfs).xGetSystemCall }.ok_or("win32 xGetSystemCall unavailable")?;
        let set = unsafe { (*vfs).xSetSystemCall }.ok_or("win32 xSetSystemCall unavailable")?;
        let mut originals = [None; 3];
        for (index, name) in NAMES.iter().enumerate() {
            originals[index] = unsafe { get(vfs, name.as_ptr()) };
            if originals[index].is_none() {
                return Err(format!("win32 {} unavailable", name.to_string_lossy()));
            }
        }
        CREATE_FILE.store(originals[0].unwrap() as usize, Ordering::Release);
        CREATE_MAPPING.store(originals[1].unwrap() as usize, Ordering::Release);
        MAP_VIEW.store(originals[2].unwrap() as usize, Ordering::Release);
        events().clear();
        let callbacks: [ffi::sqlite3_syscall_ptr; 3] = unsafe {
            [
                Some(std::mem::transmute::<CreateFile, unsafe extern "C" fn()>(
                    trace_create_file,
                )),
                Some(
                    std::mem::transmute::<CreateMapping, unsafe extern "C" fn()>(
                        trace_create_mapping,
                    ),
                ),
                Some(std::mem::transmute::<MapView, unsafe extern "C" fn()>(
                    trace_map_view,
                )),
            ]
        };
        let mut guard = Self {
            vfs,
            originals,
            installed: 0,
            _not_send: PhantomData,
        };
        for (index, name) in NAMES.iter().enumerate() {
            let rc = unsafe { set(vfs, name.as_ptr(), callbacks[index]) };
            if rc != ffi::SQLITE_OK {
                return Err(format!("install {} returned {rc}", name.to_string_lossy()));
            }
            guard.installed += 1;
        }
        Ok(guard)
    }

    pub(crate) fn snapshot(&self) -> Vec<Event> {
        events().clone()
    }

    /// Exact SR04 open proof. Call before recognised writable transition.
    /// Mapping proof is separate because crash readers may use a heap index.
    pub(crate) fn assert_held_wal_readonly(&self, live: &Path) -> Result<(), String> {
        let wal = normalise(&format!("{}-wal", live.display()));
        let shm = normalise(&format!("{}-shm", live.display()));
        let trace = self.snapshot();
        let mut denied_rw = None;
        let mut readonly_wal = false;
        let mut shm_handles = Vec::new();
        for (index, event) in trace.iter().enumerate() {
            if let Event::Open {
                path,
                access,
                disposition,
                handle,
                error,
                ..
            } = event
            {
                let path = normalise(path);
                if path == wal {
                    if *access & GENERIC_WRITE != 0 {
                        if *disposition != OPEN_ALWAYS
                            || *handle != usize::MAX
                            || *error != ERROR_SHARING_VIOLATION
                        {
                            return Err(format!("WAL RW was not denied by native seal: {event:?}"));
                        }
                        denied_rw = Some(index);
                    } else {
                        if *access != GENERIC_READ
                            || *disposition != OPEN_EXISTING
                            || *handle == usize::MAX
                            || denied_rw.is_none_or(|prior| prior >= index)
                        {
                            return Err(format!(
                                "WAL RO was not successful OPEN_EXISTING after denied RW: {event:?}"
                            ));
                        }
                        readonly_wal = true;
                    }
                }
                if path == shm {
                    if *access != GENERIC_READ
                        || *disposition != OPEN_ALWAYS
                        || *handle == usize::MAX
                    {
                        return Err(format!(
                            "SHM open was not successful readonly OPEN_ALWAYS: {event:?}"
                        ));
                    }
                    shm_handles.push(*handle);
                }
            }
        }
        if denied_rw.is_none()
            || !readonly_wal
            || shm_handles.len() != 2
            || shm_handles[0] == shm_handles[1]
        {
            return Err(format!(
                "missing WAL fallback or two distinct readonly SHM handles: {trace:#?}"
            ));
        }
        Ok(())
    }

    /// Strict mapping obligation. Absence is an error, never vacuous success.
    /// A fresh crash fixture's heap reconstruction may correctly leave it unmet.
    pub(crate) fn assert_readonly_shm_mapping(&self, live: &Path) -> Result<(), String> {
        let shm = normalise(&format!("{}-shm", live.display()));
        let trace = self.snapshot();
        let mut handles = Vec::new();
        let mut mappings = Vec::new();
        let mut views = 0;
        for event in &trace {
            match event {
                Event::Open { path, handle, .. }
                    if normalise(path) == shm && *handle != usize::MAX =>
                {
                    handles.push(*handle)
                }
                Event::Mapping {
                    file,
                    protect,
                    mapping,
                    ..
                } if handles.contains(file) => {
                    if *protect != PAGE_READONLY || *mapping == 0 {
                        return Err(format!(
                            "SHM mapping not successful PAGE_READONLY: {event:?}"
                        ));
                    }
                    mappings.push(*mapping);
                }
                Event::View {
                    mapping,
                    access,
                    address,
                    ..
                } if mappings.contains(mapping) => {
                    if *access != FILE_MAP_READ || *address == 0 {
                        return Err(format!("SHM view not successful FILE_MAP_READ: {event:?}"));
                    }
                    views += 1;
                }
                _ => {}
            }
        }
        if mappings.is_empty() || views != mappings.len() {
            return Err(format!(
                "PAGE_READONLY/FILE_MAP_READ proof absent (heap WAL index is possible): {trace:#?}"
            ));
        }
        Ok(())
    }

    pub(crate) fn assert_no_sidecar_opens(&self, copy: &Path) -> Result<(), String> {
        let copy = normalise(&copy.to_string_lossy());
        let trace = self.snapshot();
        let mut main_seen = false;
        for event in &trace {
            if let Event::Open {
                path,
                access,
                disposition,
                handle,
                ..
            } = event
            {
                let path = normalise(path);
                if path == copy {
                    if *access != GENERIC_READ
                        || *disposition != OPEN_EXISTING
                        || *handle == usize::MAX
                    {
                        return Err(format!(
                            "inspection copy was not opened readonly existing: {event:?}"
                        ));
                    }
                    main_seen = true;
                }
                if ["-wal", "-shm", "-journal"]
                    .iter()
                    .any(|suffix| path == format!("{copy}{suffix}"))
                {
                    return Err(format!("inspection copy sidecar open observed: {event:?}"));
                }
            }
        }
        if !main_seen {
            return Err(format!("no inspection copy open observed: {trace:#?}"));
        }
        Ok(())
    }

    pub(crate) fn assert_no_opens(&self, path: &Path) -> Result<(), String> {
        let expected = normalise(&path.to_string_lossy());
        for event in self.snapshot() {
            if let Event::Open { ref path, .. } = event
                && normalise(path) == expected
            {
                return Err(format!("unexpected source SQLite open: {event:?}"));
            }
        }
        Ok(())
    }
}

impl Drop for TraceGuard {
    fn drop(&mut self) {
        // The caller must first close its connections; no other SQLite threads
        // exist in this dedicated child. Restore exactly the captured pointers.
        let set = unsafe { (*self.vfs).xSetSystemCall }.expect("checked at installation");
        for index in (0..self.installed).rev() {
            let rc = unsafe { set(self.vfs, NAMES[index].as_ptr(), self.originals[index]) };
            if rc != ffi::SQLITE_OK {
                std::process::abort();
            }
        }
        ACTIVE.store(false, Ordering::Release);
    }
}

fn normalise(path: &str) -> String {
    path.strip_prefix(r"\\?\")
        .unwrap_or(path)
        .replace('/', "\\")
        .to_lowercase()
}
