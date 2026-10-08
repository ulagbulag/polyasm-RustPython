//! Files and directories of the embedder host.

use alloc_crate::{string::String, vec, vec::Vec};
use core::fmt;

use crate::{
    consts::{
        DEFAULT_DIR_MODE, DEFAULT_FILE_MODE, ENOSYS, MAX_RW, O_APPEND, O_CLOEXEC, O_CREAT, O_EXCL,
        O_RDONLY, O_RDWR, O_TRUNC, O_WRONLY, S_IFBLK, S_IFCHR, S_IFDIR, S_IFIFO, S_IFLNK, S_IFMT,
        S_IFREG, S_IFSOCK, S_IWALL, SEEK_CUR, SEEK_END, SEEK_SET,
    },
    ffi::OsString,
    host::{Stat, host},
    io::{self, Error, ErrorKind, Read, Seek, SeekFrom, Write},
    os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd},
    path::{self, Component, Path, PathBuf},
    time::SystemTime,
};

const INVALID_ACCESS: Error = Error::const_message(
    ErrorKind::InvalidInput,
    "creating or truncating a file takes write or append access",
);
const ACCESS_REQUIRED: Error = Error::const_message(
    ErrorKind::InvalidInput,
    "opening a file takes read, write or append access",
);
const TREE_INCOMPLETE: Error =
    Error::const_message(ErrorKind::Uncategorized, "failed to create whole tree");

/// An open file of the host.
pub struct File {
    fd: OwnedFd,
}

/// Options and flags for opening a [`File`].
#[derive(Clone, Debug)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
    create_new: bool,
    mode: u32,
    custom_flags: i32,
}

/// The status of a file.
#[derive(Clone)]
pub struct Metadata {
    stat: Stat,
}

/// The type of a file.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct FileType {
    mode: u32,
}

/// The permission bits of a file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Permissions {
    mode: u32,
}

/// One entry of a [`ReadDir`].
pub struct DirEntry {
    dir: PathBuf,
    name: Vec<u8>,
}

/// Iterator over the entries of a directory, made by [`read_dir`].
pub struct ReadDir {
    dir: PathBuf,
    names: vec::IntoIter<Vec<u8>>,
}

impl File {
    /// Opens `path` for reading.
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        OpenOptions::new().read(true).open(path)
    }

    /// Opens `path` for writing, creating it or truncating it.
    pub fn create<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
    }

    /// Creates `path` for reading and writing; an existing file fails the call.
    pub fn create_new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
    }

    #[must_use]
    pub fn options() -> OpenOptions {
        OpenOptions::new()
    }

    pub fn sync_all(&self) -> io::Result<()> {
        host().fsync(self.fd.as_raw_fd())
    }

    pub fn sync_data(&self) -> io::Result<()> {
        host().fsync(self.fd.as_raw_fd())
    }

    pub fn set_len(&self, size: u64) -> io::Result<()> {
        host().ftruncate(self.fd.as_raw_fd(), size)
    }

    pub fn metadata(&self) -> io::Result<Metadata> {
        host()
            .fstat(self.fd.as_raw_fd())
            .map(|stat| Metadata { stat })
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        self.fd.try_clone().map(|fd| Self { fd })
    }

    pub fn set_permissions(&self, perm: Permissions) -> io::Result<()> {
        let _ = perm;
        Err(Error::from_raw_os_error(ENOSYS))
    }
}

fn read_fd(fd: RawFd, buf: &mut [u8]) -> io::Result<usize> {
    let len = buf.len().min(MAX_RW);
    host().read(fd, &mut buf[..len])
}

fn write_fd(fd: RawFd, buf: &[u8]) -> io::Result<usize> {
    host().write(fd, &buf[..buf.len().min(MAX_RW)])
}

fn seek_fd(fd: RawFd, pos: SeekFrom) -> io::Result<u64> {
    let (offset, whence) = match pos {
        SeekFrom::Start(offset) => (
            i64::try_from(offset).map_err(|_| Error::from(ErrorKind::InvalidInput))?,
            SEEK_SET,
        ),
        SeekFrom::End(offset) => (offset, SEEK_END),
        SeekFrom::Current(offset) => (offset, SEEK_CUR),
    };
    host().seek(fd, offset, whence)
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_fd(self.fd.as_raw_fd(), buf)
    }
}

impl Read for &File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        read_fd(self.fd.as_raw_fd(), buf)
    }
}

impl Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write_fd(self.fd.as_raw_fd(), buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Write for &File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write_fd(self.fd.as_raw_fd(), buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for File {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        seek_fd(self.fd.as_raw_fd(), pos)
    }
}

impl Seek for &File {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        seek_fd(self.fd.as_raw_fd(), pos)
    }
}

impl AsFd for File {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl AsRawFd for File {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl FromRawFd for File {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self {
            // SAFETY: the caller hands over an open descriptor it owns.
            fd: unsafe { OwnedFd::from_raw_fd(fd) },
        }
    }
}

impl IntoRawFd for File {
    fn into_raw_fd(self) -> RawFd {
        self.fd.into_raw_fd()
    }
}

impl From<OwnedFd> for File {
    fn from(fd: OwnedFd) -> Self {
        Self { fd }
    }
}

impl From<File> for OwnedFd {
    fn from(file: File) -> Self {
        file.fd
    }
}

impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("File")
            .field("fd", &self.fd.as_raw_fd())
            .finish()
    }
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenOptions {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
            create_new: false,
            mode: DEFAULT_FILE_MODE,
            custom_flags: 0,
        }
    }

    pub const fn read(&mut self, read: bool) -> &mut Self {
        self.read = read;
        self
    }

    pub const fn write(&mut self, write: bool) -> &mut Self {
        self.write = write;
        self
    }

    pub const fn append(&mut self, append: bool) -> &mut Self {
        self.append = append;
        self
    }

    pub const fn truncate(&mut self, truncate: bool) -> &mut Self {
        self.truncate = truncate;
        self
    }

    pub const fn create(&mut self, create: bool) -> &mut Self {
        self.create = create;
        self
    }

    pub const fn create_new(&mut self, create_new: bool) -> &mut Self {
        self.create_new = create_new;
        self
    }

    pub(crate) const fn set_mode(&mut self, mode: u32) -> &mut Self {
        self.mode = mode;
        self
    }

    pub(crate) const fn set_custom_flags(&mut self, flags: i32) -> &mut Self {
        self.custom_flags = flags;
        self
    }

    const fn access_mode(&self) -> io::Result<i32> {
        match (self.read, self.write, self.append) {
            (true, false, false) => Ok(O_RDONLY),
            (false, true, false) => Ok(O_WRONLY),
            (true, true, false) => Ok(O_RDWR),
            (false, _, true) => Ok(O_WRONLY | O_APPEND),
            (true, _, true) => Ok(O_RDWR | O_APPEND),
            (false, false, false) => Err(ACCESS_REQUIRED),
        }
    }

    const fn creation_mode(&self) -> io::Result<i32> {
        if !self.write && !self.append && (self.truncate || self.create || self.create_new) {
            return Err(INVALID_ACCESS);
        }
        if self.append && self.truncate && !self.create_new {
            return Err(INVALID_ACCESS);
        }
        Ok(match (self.create, self.truncate, self.create_new) {
            (false, false, false) => 0,
            (true, false, false) => O_CREAT,
            (false, true, false) => O_TRUNC,
            (true, true, false) => O_CREAT | O_TRUNC,
            (_, _, true) => O_CREAT | O_EXCL,
        })
    }

    /// Opens `path` with these options.
    pub fn open<P: AsRef<Path>>(&self, path: P) -> io::Result<File> {
        let flags = O_CLOEXEC | self.access_mode()? | self.creation_mode()? | self.custom_flags;
        let fd = host().open(path::host_bytes(path.as_ref()), flags, self.mode)?;
        // SAFETY: the host answered a new descriptor this call owns.
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}

impl From<Stat> for Metadata {
    fn from(stat: Stat) -> Self {
        Self { stat }
    }
}

impl Metadata {
    pub(crate) const fn stat(&self) -> &Stat {
        &self.stat
    }

    #[must_use]
    pub const fn file_type(&self) -> FileType {
        FileType {
            mode: self.stat.mode,
        }
    }

    #[must_use]
    pub const fn is_dir(&self) -> bool {
        self.file_type().is_dir()
    }

    #[must_use]
    pub const fn is_file(&self) -> bool {
        self.file_type().is_file()
    }

    #[must_use]
    pub const fn is_symlink(&self) -> bool {
        self.file_type().is_symlink()
    }

    #[must_use]
    #[allow(
        clippy::len_without_is_empty,
        reason = "std::fs::Metadata offers len alone"
    )]
    pub const fn len(&self) -> u64 {
        self.stat.size
    }

    #[must_use]
    pub const fn permissions(&self) -> Permissions {
        Permissions {
            mode: self.stat.mode,
        }
    }

    pub fn modified(&self) -> io::Result<SystemTime> {
        Ok(SystemTime::from_unix_nanos(self.stat.mtime_ns))
    }

    pub fn accessed(&self) -> io::Result<SystemTime> {
        Ok(SystemTime::from_unix_nanos(self.stat.atime_ns))
    }

    /// The creation time; the host status carries access, modification and change times alone,
    /// so this answers [`ErrorKind::Unsupported`].
    pub fn created(&self) -> io::Result<SystemTime> {
        Err(Error::const_message(
            ErrorKind::Unsupported,
            "creation time is unavailable on this host",
        ))
    }
}

impl fmt::Debug for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Metadata")
            .field("file_type", &self.file_type())
            .field("permissions", &self.permissions())
            .field("len", &self.len())
            .finish_non_exhaustive()
    }
}

impl FileType {
    const fn is(self, kind: u32) -> bool {
        self.mode & S_IFMT == kind
    }

    #[must_use]
    pub const fn is_dir(&self) -> bool {
        self.is(S_IFDIR)
    }

    #[must_use]
    pub const fn is_file(&self) -> bool {
        self.is(S_IFREG)
    }

    #[must_use]
    pub const fn is_symlink(&self) -> bool {
        self.is(S_IFLNK)
    }

    pub(crate) const fn is_block_device(self) -> bool {
        self.is(S_IFBLK)
    }

    pub(crate) const fn is_char_device(self) -> bool {
        self.is(S_IFCHR)
    }

    pub(crate) const fn is_fifo(self) -> bool {
        self.is(S_IFIFO)
    }

    pub(crate) const fn is_socket(self) -> bool {
        self.is(S_IFSOCK)
    }
}

impl fmt::Debug for FileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileType")
            .field("is_file", &self.is_file())
            .field("is_dir", &self.is_dir())
            .field("is_symlink", &self.is_symlink())
            .finish_non_exhaustive()
    }
}

impl Permissions {
    pub(crate) const fn from_mode(mode: u32) -> Self {
        Self { mode }
    }

    pub(crate) const fn mode(&self) -> u32 {
        self.mode
    }

    pub(crate) const fn set_mode(&mut self, mode: u32) {
        self.mode = mode;
    }

    #[must_use]
    pub const fn readonly(&self) -> bool {
        self.mode & S_IWALL == 0
    }

    pub const fn set_readonly(&mut self, readonly: bool) {
        if readonly {
            self.mode &= !S_IWALL;
        } else {
            self.mode |= S_IWALL;
        }
    }
}

impl DirEntry {
    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.dir.join(Path::from_bytes(&self.name))
    }

    #[must_use]
    pub fn file_name(&self) -> OsString {
        OsString::from_vec(self.name.clone())
    }

    /// The status of the entry itself, symbolic links unfollowed.
    pub fn metadata(&self) -> io::Result<Metadata> {
        symlink_metadata(self.path())
    }

    pub fn file_type(&self) -> io::Result<FileType> {
        self.metadata().map(|m| m.file_type())
    }
}

impl fmt::Debug for DirEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DirEntry").field(&self.path()).finish()
    }
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<io::Result<DirEntry>> {
        self.names.next().map(|name| {
            Ok(DirEntry {
                dir: self.dir.clone(),
                name,
            })
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.names.size_hint()
    }
}

impl fmt::Debug for ReadDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ReadDir").field(&self.dir).finish()
    }
}

/// Every byte of the file at `path`.
pub fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let size = file
        .metadata()
        .map_or(0, |m| usize::try_from(m.len()).unwrap_or(0));
    let mut bytes = Vec::with_capacity(size);
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// The UTF-8 text of the file at `path`.
pub fn read_to_string<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let mut text = String::new();
    File::open(path)?.read_to_string(&mut text)?;
    Ok(text)
}

/// Writes `contents` as the whole file at `path`.
pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    File::create(path)?.write_all(contents.as_ref())
}

/// The status of `path`, following symbolic links.
pub fn metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    host()
        .stat(path::host_bytes(path.as_ref()))
        .map(Metadata::from)
}

/// The status of `path` itself.
pub fn symlink_metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    host()
        .lstat(path::host_bytes(path.as_ref()))
        .map(Metadata::from)
}

/// Whether `path` exists.
pub fn exists<P: AsRef<Path>>(path: P) -> io::Result<bool> {
    match metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

/// The entries of the directory `path`.
pub fn read_dir<P: AsRef<Path>>(path: P) -> io::Result<ReadDir> {
    let dir = path.as_ref().to_path_buf();
    let names = host().listdir(path::host_bytes(&dir))?;
    Ok(ReadDir {
        dir,
        names: names.into_iter(),
    })
}

pub(crate) fn create_dir_with_mode(path: &Path, mode: u32) -> io::Result<()> {
    host().mkdir(path::host_bytes(path), mode)
}

/// Creates the directory `path`.
pub fn create_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    create_dir_with_mode(path.as_ref(), DEFAULT_DIR_MODE)
}

/// Creates the directory `path` and every missing parent.
pub fn create_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref();
    if path.as_os_str().is_empty() {
        return Ok(());
    }
    match create_dir(path) {
        Ok(()) => return Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(_) if path.is_dir() => return Ok(()),
        Err(e) => return Err(e),
    }
    match path.parent() {
        Some(parent) => create_dir_all(parent)?,
        None => return Err(TREE_INCOMPLETE),
    }
    match create_dir(path) {
        Ok(()) => Ok(()),
        Err(_) if path.is_dir() => Ok(()),
        Err(e) => Err(e),
    }
}

/// Removes the file `path`.
pub fn remove_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
    host().unlink(path::host_bytes(path.as_ref()))
}

/// Removes the empty directory `path`.
pub fn remove_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    host().rmdir(path::host_bytes(path.as_ref()))
}

/// Removes the directory `path` with everything inside it.
pub fn remove_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref();
    for entry in read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        if entry.file_type()?.is_dir() {
            remove_dir_all(&child)?;
        } else {
            remove_file(&child)?;
        }
    }
    remove_dir(path)
}

/// Moves `from` to `to`, replacing a file at `to`.
pub fn rename<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<()> {
    host().rename(
        path::host_bytes(from.as_ref()),
        path::host_bytes(to.as_ref()),
    )
}

/// Copies the bytes of `from` into `to`, answering the count.
pub fn copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<u64> {
    let mut source = File::open(from)?;
    let mut target = File::create(to)?;
    io::copy(&mut source, &mut target)
}

/// The target of the symbolic link `path`.
pub fn read_link<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    host()
        .readlink(path::host_bytes(path.as_ref()))
        .map(path::from_host_bytes)
}

/// The absolute form of the existing `path`, `.` and `..` resolved.
pub fn canonicalize<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    let absolute = path::absolute(path)?;
    let mut canonical = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::ParentDir => {
                canonical.pop();
            }
            Component::Normal(_) | Component::RootDir => canonical.push(component),
            Component::CurDir | Component::Prefix(_) => {}
        }
    }
    metadata(&canonical)?;
    Ok(canonical)
}

/// Sets the permission bits of `path`; permission changes stay outside the host interface, so
/// this answers `ENOSYS`.
pub fn set_permissions<P: AsRef<Path>>(path: P, perm: Permissions) -> io::Result<()> {
    let _ = (path, perm);
    Err(Error::from_raw_os_error(ENOSYS))
}

/// Creates a hard link; links stay outside the host interface, so this answers `ENOSYS`.
pub fn hard_link<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> io::Result<()> {
    let _ = (original, link);
    Err(Error::from_raw_os_error(ENOSYS))
}
