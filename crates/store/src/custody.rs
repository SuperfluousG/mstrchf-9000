//! Private Windows file custody. Paths never substitute for the retained handles.
use crate::records::storage;
use mstrchf_contracts::{DomainError, ErrorCode};
use sha2::{Digest as _, Sha256};
use std::os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::AsRawHandle};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, Instant},
};
use windows_sys::Win32::Storage::FileSystem::*;

const DB_LIMIT: u64 = 1_073_741_824;
const SHM_LIMIT: u64 = 33_554_432;
const MANIFEST_LIMIT: u64 = 2_097_152;
const BUFFER: usize = 65_536;
const BUDGET: Duration = Duration::from_secs(120);

#[cfg(test)]
mod injection {
    use std::{
        cell::{Cell, RefCell},
        collections::VecDeque,
    };
    type IdScript = VecDeque<Result<[u8; 16], ()>>;
    thread_local! {
        static IO_FAULT: Cell<&'static str> = const { Cell::new("") };
        static IDS: RefCell<Option<IdScript>> = const { RefCell::new(None) };
        static CALLS: Cell<usize> = const { Cell::new(0) };
    }
    pub(super) struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            IO_FAULT.with(|f| f.set(""));
            IDS.with(|ids| *ids.borrow_mut() = None);
        }
    }
    pub(super) fn arm(name: &'static str) -> Reset {
        IO_FAULT.with(|f| f.set(name));
        Reset
    }
    pub(super) fn ids(ids: Vec<Result<[u8; 16], ()>>) -> Reset {
        IDS.with(|q| *q.borrow_mut() = Some(ids.into()));
        CALLS.with(|c| c.set(0));
        Reset
    }
    pub(super) fn calls() -> usize {
        CALLS.with(Cell::get)
    }
    pub(super) fn id() -> Option<Result<[u8; 16], ()>> {
        IDS.with(|q| {
            q.borrow_mut().as_mut().map(|q| {
                CALLS.with(|c| c.set(c.get() + 1));
                q.pop_front().unwrap_or(Err(()))
            })
        })
    }
    pub(super) fn fault(name: &str) -> bool {
        let local = IO_FAULT.with(|f| {
            if f.get() == name {
                f.set("");
                true
            } else {
                false
            }
        });
        local || crate::backup_upgrade_tests::fault(name)
    }
}

fn random_attempt_id() -> Result<[u8; 16], DomainError> {
    #[cfg(test)]
    if crate::backup_upgrade_tests::fault("custody_rng_error") {
        return Err(storage("injected RNG failure"));
    }
    #[cfg(test)]
    if let Some(result) = injection::id() {
        return result.map_err(|_| storage("injected RNG failure"));
    }
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).map_err(storage)?;
    Ok(bytes)
}

fn write_manifest_bytes(file: &mut File, bytes: &[u8]) -> std::io::Result<()> {
    #[cfg(test)]
    {
        // Exercise write_all against a real partial file followed by a simulated
        // device failure. No test fills the volume or modifies sealed content.
        struct FaultyWriter<'a> {
            file: &'a mut File,
            first: bool,
            full: bool,
        }
        impl Write for FaultyWriter<'_> {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                if self.first {
                    self.first = false;
                    return self.file.write(&bytes[..bytes.len().min(3)]);
                }
                if self.full {
                    Err(std::io::Error::from_raw_os_error(112))
                } else {
                    Ok(0)
                }
            }
            fn flush(&mut self) -> std::io::Result<()> {
                self.file.flush()
            }
        }
        if injection::fault("custody_manifest_short_write") {
            return FaultyWriter {
                file,
                first: true,
                full: false,
            }
            .write_all(bytes);
        }
        if injection::fault("custody_manifest_enospc") {
            return FaultyWriter {
                file,
                first: true,
                full: true,
            }
            .write_all(bytes);
        }
    }
    file.write_all(bytes)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FileIdentity {
    pub(super) volume_serial: u64,
    pub(super) file_id: [u8; 16],
}

struct Directory {
    _parent: Option<Rc<Directory>>,
    file: File,
    path: PathBuf,
    identity: FileIdentity,
}

pub(super) struct TrustedStoreRoot {
    directory: Rc<Directory>,
}
pub(super) struct StoreLock {
    file: File,
    identity: FileIdentity,
    directory: Rc<Directory>,
}
pub(super) struct GuardedLive {
    path: PathBuf,
    identity: FileIdentity,
    file: File,
    directory: Rc<Directory>,
}
pub(super) struct PrivateBackupDir {
    directory: Rc<Directory>,
    id: String,
}
pub(super) struct ReservedFile {
    path: PathBuf,
    identity: FileIdentity,
    guard: File,
    directory: Rc<Directory>,
    limit: u64,
}
pub(super) struct SealedFile {
    path: PathBuf,
    identity: FileIdentity,
    file: File,
    directory: Rc<Directory>,
    length: u64,
    sha256: String,
    limit: u64,
}

fn limit_error() -> DomainError {
    DomainError::new(ErrorCode::LimitExceeded, "/storage/backup")
}
fn check_time(start: Instant) -> Result<(), DomainError> {
    if start.elapsed() > BUDGET {
        Err(storage("custody deadline"))
    } else {
        Ok(())
    }
}
fn component(name: &str) -> Result<(), DomainError> {
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.ends_with(['.', ' '])
        || name.chars().any(|c| c < ' ' || "<>:\"/\\|?*".contains(c))
        || matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        )
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
    {
        return Err(storage("path policy"));
    }
    Ok(())
}
fn lexical_root(path: &Path) -> Result<(PathBuf, Vec<String>), DomainError> {
    let text = path
        .to_str()
        .ok_or_else(|| storage("non Unicode root"))?
        .replace('/', "\\");
    let bytes = text.as_bytes();
    if bytes.len() < 3
        || !bytes[0].is_ascii_alphabetic()
        || bytes[1] != b':'
        || bytes[2] != b'\\'
        || text.contains('\0')
    {
        return Err(storage("path policy"));
    }
    let drive = PathBuf::from(&text[..3]);
    let rest = text[3..].strip_suffix('\\').unwrap_or(&text[3..]);
    let mut parts = Vec::new();
    if !rest.is_empty() {
        for p in rest.split('\\') {
            component(p)?;
            parts.push(p.to_owned());
        }
    }
    Ok((drive, parts))
}
fn options(write: bool, sealed: bool, directory: bool) -> OpenOptions {
    let mut o = OpenOptions::new();
    o.read(true)
        .write(write)
        .share_mode(if sealed {
            FILE_SHARE_READ
        } else {
            FILE_SHARE_READ | FILE_SHARE_WRITE
        })
        .custom_flags(
            FILE_FLAG_OPEN_REPARSE_POINT
                | if directory {
                    FILE_FLAG_BACKUP_SEMANTICS
                } else {
                    0
                },
        );
    if directory {
        o.access_mode(FILE_READ_ATTRIBUTES);
    }
    o
}
fn identity(file: &File) -> Result<FileIdentity, DomainError> {
    let mut info = std::mem::MaybeUninit::<FILE_ID_INFO>::uninit();
    // The retained handle is valid and the buffer has the exact queried native size.
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileIdInfo,
            info.as_mut_ptr().cast(),
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    } == 0
    {
        return Err(storage(format!(
            "FileIdInfo: {:?}",
            std::io::Error::last_os_error().raw_os_error()
        )));
    }
    let info = unsafe { info.assume_init() };
    Ok(FileIdentity {
        volume_serial: info.VolumeSerialNumber,
        file_id: info.FileId.Identifier,
    })
}

/// Diagnostic identity only: never authorises reuse or creates custody.
#[cfg(test)]
pub(super) fn identity_for_test(path: &Path, directory: bool) -> FileIdentity {
    identity(&options(false, false, directory).open(path).unwrap()).unwrap()
}
fn final_path(file: &File) -> Result<PathBuf, DomainError> {
    let mut buffer = vec![0u16; 512];
    loop {
        let count = unsafe {
            GetFinalPathNameByHandleW(
                file.as_raw_handle(),
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                0,
            )
        };
        if count == 0 {
            return Err(storage(format!(
                "final path: {:?}",
                std::io::Error::last_os_error().raw_os_error()
            )));
        }
        if count as usize >= buffer.len() {
            let next = (count as usize).checked_add(1).ok_or_else(limit_error)?;
            if next > 32768 {
                return Err(limit_error());
            }
            buffer.resize(next, 0);
            continue;
        }
        let text = String::from_utf16(&buffer[..count as usize]).map_err(storage)?;
        let text = text
            .strip_prefix("\\\\?\\")
            .ok_or_else(|| storage("unexpected final path"))?;
        return Ok(PathBuf::from(text));
    }
}
fn same_path(a: &Path, b: &Path) -> bool {
    // Conservative for non-ASCII case aliases: exact Unicode codepoints are required.
    a.as_os_str()
        .encode_wide()
        .map(|c| if (65..=90).contains(&c) { c + 32 } else { c })
        .eq(b
            .as_os_str()
            .encode_wide()
            .map(|c| if (65..=90).contains(&c) { c + 32 } else { c }))
}
fn checked(
    file: &File,
    expected: &Path,
    directory: bool,
    volume: Option<u64>,
) -> Result<FileIdentity, DomainError> {
    let mut tag = std::mem::MaybeUninit::<FILE_ATTRIBUTE_TAG_INFO>::uninit();
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileAttributeTagInfo,
            tag.as_mut_ptr().cast(),
            std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    } == 0
    {
        return Err(storage(format!(
            "FileAttributeTagInfo: {:?}",
            std::io::Error::last_os_error().raw_os_error()
        )));
    }
    let tag = unsafe { tag.assume_init() };
    if tag.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || (tag.FileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0) != directory
    {
        return Err(storage("reparse or wrong file kind"));
    }
    let id = identity(file)?;
    if volume.is_some_and(|v| v != id.volume_serial) {
        return Err(storage("volume changed"));
    }
    if !directory {
        let mut links = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), links.as_mut_ptr()) } == 0 {
            return Err(storage(format!(
                "link information: {:?}",
                std::io::Error::last_os_error().raw_os_error()
            )));
        }
        if unsafe { links.assume_init() }.nNumberOfLinks != 1
            || !file.metadata().map_err(storage)?.is_file()
        {
            return Err(storage("unexpected leaf links or kind"));
        }
    }
    if !same_path(&final_path(file)?, expected) || identity(file)? != id {
        return Err(storage("file containment or identity changed"));
    }
    Ok(id)
}
fn child(parent: &Rc<Directory>, name: &str, fresh: bool) -> Result<Rc<Directory>, DomainError> {
    component(name)?;
    if checked(
        &parent.file,
        &parent.path,
        true,
        Some(parent.identity.volume_serial),
    )? != parent.identity
    {
        return Err(storage("parent directory identity changed"));
    }
    let path = parent.path.join(name);
    if fresh {
        std::fs::create_dir(&path).map_err(storage)?;
    }
    let file = match options(false, false, true).open(&path) {
        Ok(f) => f,
        Err(e) if !fresh && e.raw_os_error() == Some(2) => {
            std::fs::create_dir(&path).map_err(storage)?;
            options(false, false, true).open(&path).map_err(storage)?
        }
        Err(e) => {
            return Err(storage(format!(
                "directory component open: {:?}",
                e.raw_os_error()
            )));
        }
    };
    let id = checked(&file, &path, true, Some(parent.identity.volume_serial))?;
    let path = final_path(&file)?;
    Ok(Rc::new(Directory {
        _parent: Some(Rc::clone(parent)),
        file,
        path,
        identity: id,
    }))
}

fn existing_child(parent: &Rc<Directory>, name: &str) -> Result<Rc<Directory>, DomainError> {
    component(name)?;
    if checked(
        &parent.file,
        &parent.path,
        true,
        Some(parent.identity.volume_serial),
    )? != parent.identity
    {
        return Err(storage("parent directory identity changed"));
    }
    let path = parent.path.join(name);
    let file = options(false, false, true).open(&path).map_err(storage)?;
    let identity = checked(&file, &path, true, Some(parent.identity.volume_serial))?;
    Ok(Rc::new(Directory {
        _parent: Some(Rc::clone(parent)),
        file,
        path,
        identity,
    }))
}

fn asset_name(digest: &str, extension: &str) -> Result<String, DomainError> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || !matches!(extension, "jpg" | "png" | "webp")
    {
        return Err(storage("asset name policy"));
    }
    Ok(format!("{digest}.{extension}"))
}

impl TrustedStoreRoot {
    pub(super) fn open_or_create(root: &Path) -> Result<Self, DomainError> {
        // StoreConfig is host-owned. The OS may redirect a known-folder child;
        // retain its resolved handle rather than asserting every ancestor from C:\.
        // Untrusted children and restore files still pass checked() below.
        lexical_root(root)?;
        std::fs::create_dir_all(root).map_err(storage)?;
        let file = options(false, false, true)
            .open(root)
            .map_err(|e| storage(format!("host root open: {:?}", e.raw_os_error())))?;
        let path = final_path(&file)?;
        let id = checked(&file, &path, true, None)?;
        let directory = Rc::new(Directory {
            _parent: None,
            path,
            file,
            identity: id,
        });
        Ok(Self { directory })
    }
    pub(super) fn path(&self) -> &Path {
        &self.directory.path
    }
    pub(super) fn identity(&self) -> FileIdentity {
        self.directory.identity
    }
    pub(super) fn acquire_store_lock(&self) -> Result<StoreLock, DomainError> {
        let path = self.path().join("store.lock");
        let file = match options(true, false, false).open(&path) {
            Ok(file) => file,
            Err(e) if e.raw_os_error() == Some(2) => options(true, false, false)
                .create_new(true)
                .open(&path)
                .map_err(storage)?,
            Err(e) => return Err(storage(e)),
        };
        let identity = checked(&file, &path, false, Some(self.identity().volume_serial))?;
        file.try_lock().map_err(storage)?;
        Ok(StoreLock {
            file,
            identity,
            directory: Rc::clone(&self.directory),
        })
    }
    pub(super) fn live(&self) -> Result<GuardedLive, DomainError> {
        guard(&self.directory, "planner.sqlite")
    }
    pub(super) fn seal_live(&self) -> Result<SealedFile, DomainError> {
        self.seal_leaf("planner.sqlite")?
            .ok_or_else(|| storage("missing live"))
    }
    pub(super) fn seal_leaf(&self, name: &str) -> Result<Option<SealedFile>, DomainError> {
        let limit = match name {
            "planner.sqlite" | "planner.sqlite-wal" | "planner.sqlite-journal" => DB_LIMIT,
            "planner.sqlite-shm" => SHM_LIMIT,
            _ => return Err(storage("unsupported source leaf")),
        };
        let path = self.path().join(name);
        let file = match options(false, true, false).open(&path) {
            Ok(file) => file,
            Err(e) if e.raw_os_error() == Some(2) => return Ok(None),
            Err(e) => return Err(storage(e)),
        };
        seal(file, path, Rc::clone(&self.directory), limit).map(Some)
    }
    pub(super) fn create_backup_dir(&self) -> Result<PrivateBackupDir, DomainError> {
        self.attempt("private-backups")
    }
    pub(super) fn seal_asset(
        &self,
        digest: &str,
        extension: &str,
    ) -> Result<SealedFile, DomainError> {
        let name = asset_name(digest, extension)?;
        let directory = existing_child(&self.directory, "assets")?;
        let path = directory.path.join(name);
        let file = options(false, true, false).open(&path).map_err(storage)?;
        let sealed = seal(file, path, directory, SHM_LIMIT)?;
        if sealed.sha256() != digest {
            return Err(storage("asset digest mismatch"));
        }
        Ok(sealed)
    }
    pub(super) fn create_preflight_dir(&self) -> Result<PrivateBackupDir, DomainError> {
        self.attempt("private-preflight")
    }
    fn attempt(&self, name: &str) -> Result<PrivateBackupDir, DomainError> {
        let parent = child(&self.directory, name, false)?;
        for _ in 0..3 {
            let bytes = random_attempt_id()?;
            if bytes.iter().all(|b| *b == 0) {
                continue;
            }
            let id: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
            // Create-new directory: collision does not adopt the prior attempt.
            match std::fs::create_dir(parent.path.join(&id)) {
                Ok(()) => {
                    return Ok(PrivateBackupDir {
                        directory: child(&parent, &id, false)?,
                        id,
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(storage(e)),
            }
        }
        Err(storage("attempt ID exhausted"))
    }
}

impl StoreLock {
    #[cfg(test)]
    pub(super) fn invalidate_identity_for_test(&mut self) {
        self.identity.file_id[0] ^= 1;
    }
    pub(super) fn verify_for(&self, root: &TrustedStoreRoot) -> Result<(), DomainError> {
        self.reverify()?;
        if self.directory.identity != root.identity() {
            return Err(storage("root lock mismatch"));
        }
        Ok(())
    }
    pub(super) fn reverify(&self) -> Result<(), DomainError> {
        if checked(
            &self.file,
            &self.directory.path.join("store.lock"),
            false,
            Some(self.identity.volume_serial),
        )? != self.identity
        {
            return Err(storage("lock identity changed"));
        }
        Ok(())
    }
}
fn guard(directory: &Rc<Directory>, name: &str) -> Result<GuardedLive, DomainError> {
    guard_matching(directory, name, None)
}
fn guard_matching(
    directory: &Rc<Directory>,
    name: &str,
    expected: Option<FileIdentity>,
) -> Result<GuardedLive, DomainError> {
    component(name)?;
    let path = directory.path.join(name);
    let file = options(false, false, false).open(&path).map_err(storage)?;
    let identity = checked(&file, &path, false, Some(directory.identity.volume_serial))?;
    if expected.is_some_and(|expected| expected != identity) {
        return Err(storage("guard identity changed"));
    }
    Ok(GuardedLive {
        path,
        identity,
        file,
        directory: Rc::clone(directory),
    })
}
impl GuardedLive {
    #[cfg(test)]
    pub(super) fn invalidate_identity_for_test(&mut self) {
        self.identity.file_id[0] ^= 1;
    }
    pub(super) fn path(&self) -> &Path {
        &self.path
    }
    pub(super) fn identity(&self) -> FileIdentity {
        self.identity
    }
    pub(super) fn reverify(&self) -> Result<(), DomainError> {
        if checked(
            &self.file,
            &self.path,
            false,
            Some(self.directory.identity.volume_serial),
        )? != self.identity
        {
            return Err(storage("live identity changed"));
        }
        Ok(())
    }
}
impl PrivateBackupDir {
    pub(super) fn path(&self) -> &Path {
        &self.directory.path
    }
    pub(super) fn identity(&self) -> FileIdentity {
        self.directory.identity
    }
    pub(super) fn id(&self) -> &str {
        &self.id
    }
    fn reserve(&self, name: &str, limit: u64) -> Result<ReservedFile, DomainError> {
        let path = self.path().join(name);
        let writer = options(true, false, false)
            .create_new(true)
            .open(&path)
            .map_err(storage)?;
        let identity = checked(&writer, &path, false, Some(self.identity().volume_serial))?;
        let guard = options(false, false, false).open(&path).map_err(storage)?;
        if checked(&guard, &path, false, Some(self.identity().volume_serial))? != identity {
            return Err(storage("reservation identity changed"));
        }
        drop(writer);
        Ok(ReservedFile {
            path,
            identity,
            guard,
            directory: Rc::clone(&self.directory),
            limit,
        })
    }
    pub(super) fn reserve_database(&self) -> Result<ReservedFile, DomainError> {
        self.reserve("database.sqlite", DB_LIMIT)
    }
    pub(super) fn reserve_inspection(&self) -> Result<ReservedFile, DomainError> {
        self.reserve("inspection.sqlite", DB_LIMIT)
    }
    pub(super) fn reserve_asset(
        &self,
        digest: &str,
        extension: &str,
    ) -> Result<ReservedFile, DomainError> {
        let name = asset_name(digest, extension)?;
        let directory = child(&self.directory, "assets", false)?;
        let assets = PrivateBackupDir {
            directory,
            id: self.id.clone(),
        };
        assets.reserve(&name, SHM_LIMIT)
    }
    pub(super) fn write_manifest(&self, bytes: &[u8]) -> Result<SealedFile, DomainError> {
        if bytes.len() as u64 > MANIFEST_LIMIT {
            return Err(limit_error());
        }
        let reserved = self.reserve("verified.json", MANIFEST_LIMIT)?;
        let mut writer = reserved.writer()?;
        write_manifest_bytes(&mut writer, bytes).map_err(storage)?;
        #[cfg(test)]
        {
            crate::backup_upgrade_tests::barrier("bu_after_manifest_write");
            if crate::backup_upgrade_tests::fault("bu_after_manifest_write") {
                return Err(storage("injected manifest IO failure"));
            }
        }
        #[cfg(test)]
        if injection::fault("custody_manifest_flush") {
            return Err(storage(std::io::Error::from_raw_os_error(1117)));
        }
        writer.sync_all().map_err(storage)?;
        drop(writer);
        reserved.seal_after_close()
    }
}
impl ReservedFile {
    pub(super) fn sqlite_path(&self) -> &Path {
        &self.path
    }
    fn writer(&self) -> Result<File, DomainError> {
        let file = options(true, false, false)
            .open(&self.path)
            .map_err(storage)?;
        if checked(&file, &self.path, false, Some(self.identity.volume_serial))? != self.identity {
            return Err(storage("reservation identity changed"));
        }
        Ok(file)
    }
    pub(super) fn seal_after_close(self) -> Result<SealedFile, DomainError> {
        // The write handle closes before deny-write acquisition; the overlapping
        // reservation handle continuously prevents deletion and replacement.
        let writer = self.writer()?;
        #[cfg(test)]
        if injection::fault("custody_reserved_flush") {
            return Err(storage(std::io::Error::from_raw_os_error(1117)));
        }
        writer.sync_all().map_err(storage)?;
        drop(writer);
        #[cfg(test)]
        if self
            .path
            .file_name()
            .is_some_and(|name| name == "database.sqlite")
        {
            crate::backup_upgrade_tests::barrier("bu_after_database_flush");
            if crate::backup_upgrade_tests::fault("bu_after_database_flush") {
                return Err(storage("injected database seal IO failure"));
            }
        }
        for suffix in ["-wal", "-shm", "-journal"] {
            let mut name = self.path.as_os_str().to_os_string();
            name.push(suffix);
            match options(false, true, false).open(PathBuf::from(name)) {
                Err(e) if e.raw_os_error() == Some(2) => (),
                _ => return Err(storage("destination sidecar present or inaccessible")),
            }
        }
        let file = options(false, true, false)
            .open(&self.path)
            .map_err(storage)?;
        if checked(&file, &self.path, false, Some(self.identity.volume_serial))? != self.identity {
            return Err(storage("seal identity changed"));
        }
        let sealed = seal(file, self.path, self.directory, self.limit)?;
        drop(self.guard);
        Ok(sealed)
    }
}
fn hash(file: &mut File, limit: u64) -> Result<(u64, String), DomainError> {
    let start = Instant::now();
    if file.metadata().map_err(storage)?.len() > limit {
        return Err(limit_error());
    }
    file.seek(SeekFrom::Start(0)).map_err(storage)?;
    let mut bytes = [0u8; BUFFER];
    let mut length = 0u64;
    let mut hash = Sha256::new();
    loop {
        check_time(start)?;
        let n = file.read(&mut bytes).map_err(storage)?;
        if n == 0 {
            break;
        }
        length = length.checked_add(n as u64).ok_or_else(limit_error)?;
        if length > limit {
            return Err(limit_error());
        }
        hash.update(&bytes[..n]);
    }
    Ok((
        length,
        hash.finalize().iter().map(|b| format!("{b:02x}")).collect(),
    ))
}
fn seal(
    mut file: File,
    path: PathBuf,
    directory: Rc<Directory>,
    limit: u64,
) -> Result<SealedFile, DomainError> {
    let identity = checked(&file, &path, false, Some(directory.identity.volume_serial))?;
    let (length, sha256) = hash(&mut file, limit)?;
    if file.metadata().map_err(storage)?.len() != length
        || checked(&file, &path, false, Some(directory.identity.volume_serial))? != identity
    {
        return Err(storage("seal changed"));
    }
    Ok(SealedFile {
        path,
        identity,
        file,
        directory,
        length,
        sha256,
        limit,
    })
}
impl SealedFile {
    #[cfg(test)]
    pub(super) fn invalidate_saved_for_test(&mut self, field: &str) {
        match field {
            "identity" => self.identity.file_id[0] ^= 1,
            "length" => self.length += 1,
            "hash" => self.sha256 = "0".repeat(64),
            _ => panic!("unknown saved field"),
        }
    }
    pub(super) fn path(&self) -> &Path {
        &self.path
    }
    pub(super) fn identity(&self) -> FileIdentity {
        self.identity
    }
    pub(super) fn length(&self) -> u64 {
        self.length
    }
    pub(super) fn sha256(&self) -> &str {
        &self.sha256
    }
    pub(super) fn header(&mut self) -> Result<[u8; 100], DomainError> {
        self.file.seek(SeekFrom::Start(0)).map_err(storage)?;
        let mut header = [0u8; 100];
        self.file.read_exact(&mut header).map_err(storage)?;
        Ok(header)
    }
    pub(super) fn reverify(&mut self) -> Result<(), DomainError> {
        if checked(
            &self.file,
            &self.path,
            false,
            Some(self.directory.identity.volume_serial),
        )? != self.identity
        {
            return Err(storage("sealed identity changed"));
        }
        let (length, sha256) = hash(&mut self.file, self.limit)?;
        if length != self.length
            || sha256 != self.sha256
            || self.file.metadata().map_err(storage)?.len() != self.length
        {
            return Err(storage("sealed contents changed"));
        }
        Ok(())
    }
    pub(super) fn permissive_guard(&self) -> Result<GuardedLive, DomainError> {
        let name = self
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| storage("leaf name"))?;
        guard_matching(&self.directory, name, Some(self.identity))
    }
    pub(super) fn copy_to(&mut self, destination: ReservedFile) -> Result<SealedFile, DomainError> {
        let start = Instant::now();
        self.reverify()?;
        if self.length > destination.limit {
            return Err(limit_error());
        }
        let mut writer = destination.writer()?;
        if writer.metadata().map_err(storage)?.len() != 0 {
            return Err(storage("copy destination not empty"));
        }
        self.file.seek(SeekFrom::Start(0)).map_err(storage)?;
        let mut bytes = [0u8; BUFFER];
        let mut length = 0u64;
        let mut digest = Sha256::new();
        loop {
            check_time(start)?;
            let n = self.file.read(&mut bytes).map_err(storage)?;
            if n == 0 {
                break;
            }
            length = length.checked_add(n as u64).ok_or_else(limit_error)?;
            if length > destination.limit {
                return Err(limit_error());
            }
            writer.write_all(&bytes[..n]).map_err(storage)?;
            digest.update(&bytes[..n]);
        }
        writer.sync_all().map_err(storage)?;
        drop(writer);
        let digest: String = digest
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if length != self.length || digest != self.sha256 {
            return Err(storage("copy mismatch"));
        }
        let mut sealed = destination.seal_after_close()?;
        if sealed.length != self.length || sealed.sha256 != self.sha256 {
            return Err(storage("sealed copy mismatch"));
        }
        self.reverify()?;
        sealed.reverify()?;
        check_time(start)?;
        Ok(sealed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn root() -> PathBuf {
        let mut id = [0u8; 16];
        getrandom::fill(&mut id).unwrap();
        std::env::temp_dir().join(format!(
            "mstrchf-custody-{}",
            id.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ))
    }
    #[test]
    fn lexical_refusals_before_any_open() {
        for p in [
            "relative",
            "C:relative",
            "\\\\server\\share",
            "\\\\?\\C:\\x",
            "C:\\x\\..\\y",
            "C:\\x:stream",
            "C:\\NUL",
            "C:\\x.\\y",
            "C:\\x\\\\y",
        ] {
            assert!(lexical_root(Path::new(p)).is_err(), "{p}");
        }
    }
    #[test]
    fn trusted_host_root_resolves_an_ancestor_alias_and_keeps_leaf_scope() {
        let base = root();
        std::fs::create_dir(&base).unwrap();
        let target = base.join("host-data");
        std::fs::create_dir(&target).unwrap();
        let alias = base.join("host-alias");
        let result = std::process::Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&alias)
            .arg(&target)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "junction setup: {} {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        let root = TrustedStoreRoot::open_or_create(&alias.join("app-root")).unwrap();
        assert_eq!(root.path(), target.join("app-root"));
        let lock = root.acquire_store_lock().unwrap();
        lock.reverify().unwrap();
        assert!(root.acquire_store_lock().is_err());
        assert!(std::fs::rename(root.path(), target.join("moved")).is_err());
        assert!(root.seal_leaf("../outside").is_err());
    }
    #[test]
    fn nested_host_root_lock_and_parent_move_are_guarded() {
        let base = root();
        let path = base.join("one").join("two");
        let root = TrustedStoreRoot::open_or_create(&path).unwrap();
        let lock = root.acquire_store_lock().unwrap();
        lock.reverify().unwrap();
        assert!(root.acquire_store_lock().is_err());
        assert!(std::fs::rename(root.path(), base.join("changed-root")).is_err());
        if std::fs::rename(&base, base.with_extension("moved")).is_ok() {
            assert!(
                lock.reverify().is_err(),
                "an OS-owned parent move must invalidate the cached scoped path"
            );
        } else {
            lock.reverify().unwrap();
        }
        eprintln!(
            "native proof: scoped root rename denial, exclusive owner lock, parent move refused or detected"
        );
    }
    #[test]
    fn sealed_copy_denies_competing_writes_and_delete_access() {
        let root = TrustedStoreRoot::open_or_create(&root()).unwrap();
        let bytes = vec![7u8; BUFFER * 2 + 19];
        std::fs::write(root.path().join("planner.sqlite"), &bytes).unwrap();
        let competing = options(true, false, false)
            .open(root.path().join("planner.sqlite"))
            .unwrap();
        assert!(root.seal_live().is_err());
        drop(competing);
        let mut source = root.seal_live().unwrap();
        let directory = root.create_preflight_dir().unwrap();
        let mut copied = source
            .copy_to(directory.reserve_inspection().unwrap())
            .unwrap();
        assert_eq!(source.length(), bytes.len() as u64);
        assert_eq!(source.sha256(), copied.sha256());
        for path in [source.path(), copied.path()] {
            assert!(options(true, false, false).open(path).is_err());
            assert!(
                OpenOptions::new()
                    .access_mode(0x00010000)
                    .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
                    .open(path)
                    .is_err()
            );
            assert!(options(false, false, false).open(path).is_ok());
        }
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "custody::tests::cross_process_seal_probe",
                "--nocapture",
            ])
            .env("MSTRCHF_CUSTODY_PROBE", copied.path())
            .output()
            .unwrap();
        assert!(
            child.status.success(),
            "{}",
            String::from_utf8_lossy(&child.stderr)
        );
        assert!(String::from_utf8_lossy(&child.stdout).contains("1 passed"));
        source.reverify().unwrap();
        copied.reverify().unwrap();
        assert!(directory.reserve_inspection().is_err());
        let g = source.permissive_guard().unwrap();
        drop(source);
        g.reverify().unwrap();
        assert!(options(true, false, false).open(g.path()).is_ok());
        eprintln!(
            "native proof: competing writer prevents seal; sealed source and copy deny WRITE and DELETE, allow READ; overlap transition keeps identity"
        );
    }
    #[test]
    fn cross_process_seal_probe() {
        let Some(path) = std::env::var_os("MSTRCHF_CUSTODY_PROBE") else {
            return;
        };
        let path = PathBuf::from(path);
        assert!(options(false, false, false).open(&path).is_ok());
        assert!(options(true, false, false).open(&path).is_err());
        assert!(
            OpenOptions::new()
                .access_mode(0x00010000)
                .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
                .open(&path)
                .is_err()
        );
    }
    #[test]
    fn junction_root_and_lock_refuse_before_target_writes() {
        let base = root();
        std::fs::create_dir(&base).unwrap();
        let target = base.join("sentinel-target");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("sentinel"), b"unchanged").unwrap();
        let link = base.join("junction");
        let output = std::process::Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "junction setup failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(TrustedStoreRoot::open_or_create(&link).is_err());
        let guarded = TrustedStoreRoot::open_or_create(&base.join("normal")).unwrap();
        let output = std::process::Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(guarded.path().join("store.lock"))
            .arg(&target)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(guarded.acquire_store_lock().is_err());
        assert_eq!(std::fs::read_dir(&target).unwrap().count(), 1);
        assert_eq!(
            std::fs::read(target.join("sentinel")).unwrap(),
            b"unchanged"
        );
        eprintln!(
            "native proof: junction root/ancestor and junction lock refused with sentinel bytes and inventory unchanged"
        );
    }
    #[test]
    fn hardlinked_lock_refuses_without_target_write() {
        let root = TrustedStoreRoot::open_or_create(&root()).unwrap();
        let sentinel = root.path().join("sentinel");
        std::fs::write(&sentinel, b"unchanged").unwrap();
        std::fs::hard_link(&sentinel, root.path().join("store.lock")).unwrap();
        assert!(root.acquire_store_lock().is_err());
        assert_eq!(std::fs::read(&sentinel).unwrap(), b"unchanged");
    }
    #[test]
    fn sealed_hash_limit_and_sidecar_absence_are_enforced() {
        let root = TrustedStoreRoot::open_or_create(&root()).unwrap();
        assert!(root.seal_leaf("planner.sqlite-wal").unwrap().is_none());
        let directory = root.create_backup_dir().unwrap();
        let reserved = directory.reserve_database().unwrap();
        std::fs::write(directory.path().join("database.sqlite-wal"), []).unwrap();
        assert!(reserved.seal_after_close().is_err());
        let tiny = directory.reserve("small.bin", 3).unwrap();
        let mut w = tiny.writer().unwrap();
        w.write_all(b"four").unwrap();
        drop(w);
        assert!(tiny.seal_after_close().is_err());
        let mut manifest = directory.write_manifest(b"{}").unwrap();
        manifest.reverify().unwrap();
        assert!(directory.write_manifest(b"{}").is_err());
    }
    #[test]
    fn asset_paths_existing_source_and_sealed_copy_are_bounded() {
        let root = TrustedStoreRoot::open_or_create(&root()).unwrap();
        let bytes = b"ordinary synthetic asset";
        let digest: String = Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert!(root.seal_asset(&digest, "png").is_err());
        assert!(!root.path().join("assets").exists());
        std::fs::create_dir(root.path().join("assets")).unwrap();
        std::fs::write(
            root.path().join("assets").join(format!("{digest}.png")),
            bytes,
        )
        .unwrap();
        let mut source = root.seal_asset(&digest, "png").unwrap();
        let directory = root.create_backup_dir().unwrap();
        let mut copy = source
            .copy_to(directory.reserve_asset(&digest, "png").unwrap())
            .unwrap();
        assert_eq!(copy.sha256(), digest);
        copy.reverify().unwrap();
        assert!(directory.reserve_asset(&digest, "png").is_err());
        for (d, extension) in [("../escape", "png"), (&digest, "../png"), (&digest, "svg")] {
            assert!(directory.reserve_asset(d, extension).is_err());
            assert!(root.seal_asset(d, extension).is_err());
        }
        let wrong = "0".repeat(64);
        std::fs::write(
            root.path().join("assets").join(format!("{wrong}.jpg")),
            bytes,
        )
        .unwrap();
        assert!(root.seal_asset(&wrong, "jpg").is_err());
    }

    #[test]
    fn attempt_actual_three_retry_path_collision_zero_and_rng_failure() {
        let root = TrustedStoreRoot::open_or_create(&root()).unwrap();
        let parent = child(&root.directory, "private-backups", false).unwrap();
        let collision = [0x11u8; 16];
        let fresh = [0x22u8; 16];
        let collision_name = "11".repeat(16);
        std::fs::create_dir(parent.path.join(&collision_name)).unwrap();
        let sentinel = parent.path.join(&collision_name).join("sentinel");
        std::fs::write(&sentinel, b"never adopted").unwrap();
        {
            let _reset = injection::ids(vec![Ok(collision), Ok(collision), Ok(fresh)]);
            let attempt = root.create_backup_dir().unwrap();
            assert_eq!(attempt.id(), "22".repeat(16));
            assert_eq!(injection::calls(), 3);
        }
        for script in [vec![Ok(collision); 4], vec![Ok([0; 16]); 4]] {
            let _reset = injection::ids(script);
            assert!(root.create_backup_dir().is_err());
            assert_eq!(
                injection::calls(),
                3,
                "fourth candidate must never be requested"
            );
        }
        {
            let _reset = injection::ids(vec![Err(()), Ok([0x33; 16])]);
            assert!(root.create_backup_dir().is_err());
            assert_eq!(
                injection::calls(),
                1,
                "RNG failure must not retry or fall back"
            );
        }
        assert_eq!(std::fs::read(&sentinel).unwrap(), b"never adopted");
        assert_eq!(std::fs::read_dir(&parent.path).unwrap().count(), 2);
    }

    #[test]
    fn saved_seal_identity_hash_length_and_live_identity_mismatches_refuse() {
        let root = TrustedStoreRoot::open_or_create(&root()).unwrap();
        let bytes = b"ordinary preserved source";
        std::fs::write(root.path().join("planner.sqlite"), bytes).unwrap();
        let mut seal = root.seal_live().unwrap();
        seal.identity.file_id[0] ^= 1;
        assert!(seal.reverify().is_err());
        seal.identity.file_id[0] ^= 1;
        let digest = seal.sha256.clone();
        seal.sha256 = "0".repeat(64);
        assert!(seal.reverify().is_err());
        seal.sha256 = digest;
        seal.length += 1;
        assert!(seal.reverify().is_err());
        seal.length -= 1;
        seal.reverify().unwrap();
        let mut live = seal.permissive_guard().unwrap();
        live.invalidate_identity_for_test();
        assert!(live.reverify().is_err());
        assert_eq!(std::fs::read(seal.path()).unwrap(), bytes);
    }

    #[test]
    fn substitution_before_first_guard_refuses_saved_identity() {
        let root = TrustedStoreRoot::open_or_create(&root()).unwrap();
        let path = root.path().join("planner.sqlite");
        std::fs::write(&path, b"original ordinary fixture").unwrap();
        let observed = File::open(&path).unwrap();
        let expected = identity(&observed).unwrap();
        drop(observed);
        // There is deliberately no leaf custody yet. Preserve the original and
        // introduce an ordinary replacement before the first protective guard.
        let preserved = root.path().join("preserved-original.bin");
        std::fs::rename(&path, &preserved).unwrap();
        std::fs::write(&path, b"replacement ordinary fixture").unwrap();
        assert!(guard_matching(&root.directory, "planner.sqlite", Some(expected)).is_err());
        let current = guard(&root.directory, "planner.sqlite").unwrap();
        assert_ne!(current.identity(), expected);
        assert_eq!(
            std::fs::read(preserved).unwrap(),
            b"original ordinary fixture"
        );
        assert_eq!(
            std::fs::read(path).unwrap(),
            b"replacement ordinary fixture"
        );
    }

    #[test]
    fn manifest_exact_limit_and_plus_one_use_real_chain() {
        let root = TrustedStoreRoot::open_or_create(&root()).unwrap();
        let mut bytes = vec![b'x'; MANIFEST_LIMIT as usize];
        bytes[0] = b'"';
        bytes[MANIFEST_LIMIT as usize - 1] = b'"';
        let exact = root.create_backup_dir().unwrap();
        let mut sealed = exact.write_manifest(&bytes).unwrap();
        assert_eq!(sealed.length(), MANIFEST_LIMIT);
        sealed.reverify().unwrap();
        bytes.push(b' ');
        let too_large = root.create_backup_dir().unwrap();
        assert!(too_large.write_manifest(&bytes).is_err());
        assert_eq!(std::fs::read_dir(too_large.path()).unwrap().count(), 0);
    }

    #[test]
    fn manifest_short_write_flush_and_disk_full_injections_preserve_partial_files() {
        let root = TrustedStoreRoot::open_or_create(&root()).unwrap();
        let bytes = br#"{"ordinary":"synthetic manifest"}"#;
        for (name, expected_length) in [
            ("custody_manifest_short_write", 3),
            ("custody_manifest_enospc", 3),
            ("custody_manifest_flush", bytes.len()),
            ("custody_reserved_flush", bytes.len()),
        ] {
            let attempt = root.create_backup_dir().unwrap();
            let _reset = injection::arm(name);
            assert!(attempt.write_manifest(bytes).is_err(), "{name}");
            let retained = std::fs::read(attempt.path().join("verified.json")).unwrap();
            assert_eq!(retained.len(), expected_length, "{name}");
            assert_eq!(retained, bytes[..expected_length], "{name}");
            assert_eq!(std::fs::read_dir(attempt.path()).unwrap().count(), 1);
        }
    }

    #[test]
    fn missing_and_reparse_asset_sources_and_destinations_preserve_sentinel() {
        let base = root();
        std::fs::create_dir(&base).unwrap();
        let target = base.join("synthetic-sentinel");
        std::fs::create_dir(&target).unwrap();
        let bytes = b"ordinary protected asset";
        let digest: String = Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let filename = format!("{digest}.png");
        std::fs::write(target.join(&filename), bytes).unwrap();
        let root = TrustedStoreRoot::open_or_create(&base.join("store")).unwrap();
        assert!(root.seal_asset(&digest, "png").is_err());
        assert!(!root.path().join("assets").exists());
        let make_junction = |link: &Path| {
            let output = std::process::Command::new("cmd.exe")
                .args(["/d", "/c", "mklink", "/J"])
                .arg(link)
                .arg(&target)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        };
        make_junction(&root.path().join("assets"));
        assert!(root.seal_asset(&digest, "png").is_err());
        let attempt = root.create_backup_dir().unwrap();
        make_junction(&attempt.path().join("assets"));
        assert!(attempt.reserve_asset(&digest, "png").is_err());
        assert_eq!(std::fs::read_dir(&target).unwrap().count(), 1);
        assert_eq!(std::fs::read(target.join(&filename)).unwrap(), bytes);
    }
}
