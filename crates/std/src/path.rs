//! POSIX paths over bytes: [`Path`], [`PathBuf`] and their [`Component`]s.

use alloc_crate::{
    borrow::{Cow, ToOwned},
    boxed::Box,
    rc::Rc,
    string::String,
    sync::Arc,
    vec::Vec,
};
use core::{
    borrow::Borrow,
    cmp, error, fmt, hash,
    iter::FusedIterator,
    ops::{Deref, DerefMut},
    str::FromStr,
};

use crate::{
    ffi::{OsStr, OsString},
    fs, io,
};

pub use crate::consts::{MAIN_SEPARATOR, MAIN_SEPARATOR_STR};

/// Whether `c` separates path components.
#[must_use]
pub const fn is_separator(c: char) -> bool {
    c == MAIN_SEPARATOR
}

const fn is_sep_byte(b: u8) -> bool {
    b == b'/'
}

/// A Windows path prefix, kept for the std signatures. A POSIX path parses to roots, current
/// and parent directories, and normal components alone.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Prefix<'a> {
    Verbatim(&'a OsStr),
    VerbatimUNC(&'a OsStr, &'a OsStr),
    VerbatimDisk(u8),
    DeviceNS(&'a OsStr),
    UNC(&'a OsStr, &'a OsStr),
    Disk(u8),
}

impl Prefix<'_> {
    #[must_use]
    pub const fn is_verbatim(&self) -> bool {
        matches!(
            *self,
            Prefix::Verbatim(_) | Prefix::VerbatimDisk(_) | Prefix::VerbatimUNC(..)
        )
    }
}

/// A Windows path prefix as a component, kept for the std signatures.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PrefixComponent<'a> {
    raw: &'a OsStr,
    parsed: Prefix<'a>,
}

impl<'a> PrefixComponent<'a> {
    #[must_use]
    pub const fn kind(&self) -> Prefix<'a> {
        self.parsed
    }

    #[must_use]
    pub const fn as_os_str(&self) -> &'a OsStr {
        self.raw
    }
}

/// One component of a path.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Component<'a> {
    Prefix(PrefixComponent<'a>),
    RootDir,
    CurDir,
    ParentDir,
    Normal(&'a OsStr),
}

impl<'a> Component<'a> {
    #[must_use]
    pub fn as_os_str(self) -> &'a OsStr {
        match self {
            Component::Prefix(p) => p.as_os_str(),
            Component::RootDir => OsStr::from_bytes(b"/"),
            Component::CurDir => OsStr::from_bytes(b"."),
            Component::ParentDir => OsStr::from_bytes(b".."),
            Component::Normal(path) => path,
        }
    }
}

impl AsRef<OsStr> for Component<'_> {
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl AsRef<Path> for Component<'_> {
    fn as_ref(&self) -> &Path {
        self.as_os_str().as_ref()
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum State {
    StartDir = 1,
    Body = 2,
    Done = 3,
}

/// Iterator over the [`Component`]s of a [`Path`].
#[derive(Clone)]
pub struct Components<'a> {
    path: &'a [u8],
    has_root: bool,
    front: State,
    back: State,
}

/// Iterator over the components of a [`Path`] as [`OsStr`] slices.
#[derive(Clone)]
pub struct Iter<'a> {
    inner: Components<'a>,
}

impl<'a> Components<'a> {
    fn len_before_body(&self) -> usize {
        let root = usize::from(self.front <= State::StartDir && self.has_root);
        let cur_dir = usize::from(self.front <= State::StartDir && self.include_cur_dir());
        root + cur_dir
    }

    fn finished(&self) -> bool {
        self.front == State::Done || self.back == State::Done || self.front > self.back
    }

    /// The part of the path still to iterate.
    #[must_use]
    pub fn as_path(&self) -> &'a Path {
        let mut comps = self.clone();
        if comps.front == State::Body {
            comps.trim_left();
        }
        if comps.back == State::Body {
            comps.trim_right();
        }
        Path::from_bytes(comps.path)
    }

    fn include_cur_dir(&self) -> bool {
        if self.has_root {
            return false;
        }
        match self.path {
            [b'.'] => true,
            [b'.', b, ..] => is_sep_byte(*b),
            _ => false,
        }
    }

    fn parse_single_component(comp: &[u8]) -> Option<Component<'_>> {
        match comp {
            b"." | b"" => None,
            b".." => Some(Component::ParentDir),
            _ => Some(Component::Normal(OsStr::from_bytes(comp))),
        }
    }

    fn parse_next_component(&self) -> (usize, Option<Component<'a>>) {
        let (extra, comp) = match self.path.iter().position(|b| is_sep_byte(*b)) {
            None => (0, self.path),
            Some(i) => (1, &self.path[..i]),
        };
        (comp.len() + extra, Self::parse_single_component(comp))
    }

    fn parse_next_component_back(&self) -> (usize, Option<Component<'a>>) {
        let start = self.len_before_body();
        let (extra, comp) = match self.path[start..].iter().rposition(|b| is_sep_byte(*b)) {
            None => (0, &self.path[start..]),
            Some(i) => (1, &self.path[start + i + 1..]),
        };
        (comp.len() + extra, Self::parse_single_component(comp))
    }

    fn trim_left(&mut self) {
        while !self.path.is_empty() {
            let (size, comp) = self.parse_next_component();
            if comp.is_some() {
                return;
            }
            self.path = &self.path[size..];
        }
    }

    fn trim_right(&mut self) {
        while self.path.len() > self.len_before_body() {
            let (size, comp) = self.parse_next_component_back();
            if comp.is_some() {
                return;
            }
            self.path = &self.path[..self.path.len() - size];
        }
    }
}

impl<'a> Iterator for Components<'a> {
    type Item = Component<'a>;

    fn next(&mut self) -> Option<Component<'a>> {
        while !self.finished() {
            match self.front {
                State::Body if !self.path.is_empty() => {
                    let (size, comp) = self.parse_next_component();
                    self.path = &self.path[size..];
                    if comp.is_some() {
                        return comp;
                    }
                }
                State::Body => self.front = State::Done,
                State::StartDir => {
                    self.front = State::Body;
                    if self.has_root {
                        self.path = &self.path[1..];
                        return Some(Component::RootDir);
                    } else if self.include_cur_dir() {
                        self.path = &self.path[1..];
                        return Some(Component::CurDir);
                    }
                }
                State::Done => unreachable!(),
            }
        }
        None
    }
}

impl<'a> DoubleEndedIterator for Components<'a> {
    fn next_back(&mut self) -> Option<Component<'a>> {
        while !self.finished() {
            match self.back {
                State::Body if self.path.len() > self.len_before_body() => {
                    let (size, comp) = self.parse_next_component_back();
                    self.path = &self.path[..self.path.len() - size];
                    if comp.is_some() {
                        return comp;
                    }
                }
                State::Body => self.back = State::StartDir,
                State::StartDir => {
                    self.back = State::Done;
                    if self.has_root {
                        self.path = &self.path[..self.path.len() - 1];
                        return Some(Component::RootDir);
                    } else if self.include_cur_dir() {
                        self.path = &self.path[..self.path.len() - 1];
                        return Some(Component::CurDir);
                    }
                }
                State::Done => unreachable!(),
            }
        }
        None
    }
}

impl FusedIterator for Components<'_> {}

impl PartialEq for Components<'_> {
    fn eq(&self, other: &Self) -> bool {
        if self.path.len() == other.path.len()
            && self.front == other.front
            && self.back == State::Body
            && other.back == State::Body
            && self.path == other.path
        {
            return true;
        }
        Iterator::eq(self.clone().rev(), other.clone().rev())
    }
}

impl Eq for Components<'_> {}

impl PartialOrd for Components<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Components<'_> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        Iterator::cmp(self.clone(), other.clone())
    }
}

impl AsRef<Path> for Components<'_> {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl AsRef<OsStr> for Components<'_> {
    fn as_ref(&self) -> &OsStr {
        self.as_path().as_os_str()
    }
}

impl fmt::Debug for Components<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl<'a> Iter<'a> {
    #[must_use]
    pub fn as_path(&self) -> &'a Path {
        self.inner.as_path()
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a OsStr;

    fn next(&mut self) -> Option<&'a OsStr> {
        self.inner.next().map(Component::as_os_str)
    }
}

impl<'a> DoubleEndedIterator for Iter<'a> {
    fn next_back(&mut self) -> Option<&'a OsStr> {
        self.inner.next_back().map(Component::as_os_str)
    }
}

impl FusedIterator for Iter<'_> {}

impl AsRef<Path> for Iter<'_> {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl AsRef<OsStr> for Iter<'_> {
    fn as_ref(&self) -> &OsStr {
        self.as_path().as_os_str()
    }
}

impl fmt::Debug for Iter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

/// Iterator over a [`Path`] and its ancestors.
#[derive(Clone, Debug)]
pub struct Ancestors<'a> {
    next: Option<&'a Path>,
}

impl<'a> Iterator for Ancestors<'a> {
    type Item = &'a Path;

    fn next(&mut self) -> Option<&'a Path> {
        let next = self.next;
        self.next = next.and_then(Path::parent);
        next
    }
}

impl FusedIterator for Ancestors<'_> {}

/// The error [`Path::strip_prefix`] answers when the path starts elsewhere than the prefix.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StripPrefixError(());

impl fmt::Display for StripPrefixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("prefix not found")
    }
}

impl error::Error for StripPrefixError {}

/// A borrowed path.
#[repr(transparent)]
pub struct Path {
    inner: OsStr,
}

/// An owned, growable path.
#[derive(Clone, Default)]
pub struct PathBuf {
    inner: OsString,
}

/// [`Path`] shown with invalid UTF-8 replaced, made by [`Path::display`].
pub struct Display<'a> {
    path: &'a Path,
}

fn iter_after<'a, 'b, I, J>(mut iter: I, mut prefix: J) -> Option<I>
where
    I: Iterator<Item = Component<'a>> + Clone,
    J: Iterator<Item = Component<'b>>,
{
    loop {
        let mut iter_next = iter.clone();
        match (iter_next.next(), prefix.next()) {
            (Some(ref x), Some(ref y)) if x == y => {}
            (Some(_) | None, Some(_)) => return None,
            (_, None) => return Some(iter),
        }
        iter = iter_next;
    }
}

/// Splits a file name into the part before its last `.` and the extension after it.
fn rsplit_file_at_dot(file: &OsStr) -> (Option<&OsStr>, Option<&OsStr>) {
    let bytes = file.bytes();
    if bytes == b".." {
        return (Some(file), None);
    }
    match bytes.iter().rposition(|b| *b == b'.') {
        None | Some(0) => (Some(file), None),
        Some(i) => (
            Some(OsStr::from_bytes(&bytes[..i])),
            Some(OsStr::from_bytes(&bytes[i + 1..])),
        ),
    }
}

impl Path {
    pub fn new<S: AsRef<OsStr> + ?Sized>(s: &S) -> &Self {
        Self::from_os_str(s.as_ref())
    }

    const fn from_os_str(s: &OsStr) -> &Self {
        // SAFETY: `Path` is a transparent wrapper of `OsStr`.
        unsafe { &*(core::ptr::from_ref::<OsStr>(s) as *const Self) }
    }

    pub(crate) const fn from_bytes(bytes: &[u8]) -> &Self {
        Self::from_os_str(OsStr::from_bytes(bytes))
    }

    pub(crate) const fn bytes(&self) -> &[u8] {
        self.inner.bytes()
    }

    #[must_use]
    pub const fn as_os_str(&self) -> &OsStr {
        &self.inner
    }

    pub const fn as_mut_os_str(&mut self) -> &mut OsStr {
        &mut self.inner
    }

    #[must_use]
    pub const fn to_str(&self) -> Option<&str> {
        self.inner.to_str()
    }

    #[must_use]
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        self.inner.to_string_lossy()
    }

    #[must_use]
    pub fn to_path_buf(&self) -> PathBuf {
        PathBuf {
            inner: self.inner.to_os_string(),
        }
    }

    /// Whether the path starts at the root.
    #[must_use]
    pub fn is_absolute(&self) -> bool {
        self.has_root()
    }

    #[must_use]
    pub fn is_relative(&self) -> bool {
        !self.is_absolute()
    }

    #[must_use]
    pub fn has_root(&self) -> bool {
        self.bytes().first().is_some_and(|b| is_sep_byte(*b))
    }

    /// The path up to its final component, when it has one.
    #[must_use]
    pub fn parent(&self) -> Option<&Self> {
        let mut comps = self.components();
        let comp = comps.next_back();
        comp.and_then(|p| match p {
            Component::Normal(_) | Component::CurDir | Component::ParentDir => {
                Some(comps.as_path())
            }
            _ => None,
        })
    }

    #[must_use]
    pub const fn ancestors(&self) -> Ancestors<'_> {
        Ancestors { next: Some(self) }
    }

    /// The final component, when it is a normal file or directory name.
    #[must_use]
    pub fn file_name(&self) -> Option<&OsStr> {
        self.components().next_back().and_then(|p| match p {
            Component::Normal(p) => Some(p),
            _ => None,
        })
    }

    /// The path with `base` removed from its start.
    pub fn strip_prefix<P: AsRef<Self>>(&self, base: P) -> Result<&Self, StripPrefixError> {
        iter_after(self.components(), base.as_ref().components())
            .map(|c| c.as_path())
            .ok_or(StripPrefixError(()))
    }

    pub fn starts_with<P: AsRef<Self>>(&self, base: P) -> bool {
        iter_after(self.components(), base.as_ref().components()).is_some()
    }

    pub fn ends_with<P: AsRef<Self>>(&self, child: P) -> bool {
        iter_after(self.components().rev(), child.as_ref().components().rev()).is_some()
    }

    /// The final file name up to its extension.
    #[must_use]
    pub fn file_stem(&self) -> Option<&OsStr> {
        self.file_name()
            .map(rsplit_file_at_dot)
            .and_then(|(before, after)| before.or(after))
    }

    /// The final file name up to its first `.`.
    #[must_use]
    pub fn file_prefix(&self) -> Option<&OsStr> {
        self.file_name().map(|name| {
            let bytes = name.bytes();
            match bytes.iter().skip(1).position(|b| *b == b'.') {
                Some(i) => OsStr::from_bytes(&bytes[..=i]),
                None => name,
            }
        })
    }

    /// The extension of the final file name.
    #[must_use]
    pub fn extension(&self) -> Option<&OsStr> {
        self.file_name()
            .map(rsplit_file_at_dot)
            .and_then(|(before, after)| before.and(after))
    }

    pub fn join<P: AsRef<Self>>(&self, path: P) -> PathBuf {
        let mut buf = self.to_path_buf();
        buf.push(path);
        buf
    }

    pub fn with_file_name<S: AsRef<OsStr>>(&self, file_name: S) -> PathBuf {
        let mut buf = self.to_path_buf();
        buf.set_file_name(file_name);
        buf
    }

    pub fn with_extension<S: AsRef<OsStr>>(&self, extension: S) -> PathBuf {
        let mut buf = self.to_path_buf();
        buf.set_extension(extension);
        buf
    }

    #[must_use]
    pub fn components(&self) -> Components<'_> {
        Components {
            path: self.bytes(),
            has_root: self.has_root(),
            front: State::StartDir,
            back: State::Body,
        }
    }

    #[must_use]
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            inner: self.components(),
        }
    }

    #[must_use]
    pub const fn display(&self) -> Display<'_> {
        Display { path: self }
    }

    pub fn metadata(&self) -> io::Result<fs::Metadata> {
        fs::metadata(self)
    }

    pub fn symlink_metadata(&self) -> io::Result<fs::Metadata> {
        fs::symlink_metadata(self)
    }

    pub fn canonicalize(&self) -> io::Result<PathBuf> {
        fs::canonicalize(self)
    }

    pub fn read_link(&self) -> io::Result<PathBuf> {
        fs::read_link(self)
    }

    pub fn read_dir(&self) -> io::Result<fs::ReadDir> {
        fs::read_dir(self)
    }

    #[must_use]
    pub fn exists(&self) -> bool {
        fs::metadata(self).is_ok()
    }

    pub fn try_exists(&self) -> io::Result<bool> {
        fs::exists(self)
    }

    #[must_use]
    pub fn is_file(&self) -> bool {
        fs::metadata(self).is_ok_and(|m| m.is_file())
    }

    #[must_use]
    pub fn is_dir(&self) -> bool {
        fs::metadata(self).is_ok_and(|m| m.is_dir())
    }

    #[must_use]
    pub fn is_symlink(&self) -> bool {
        fs::symlink_metadata(self).is_ok_and(|m| m.file_type().is_symlink())
    }

    #[must_use]
    pub fn into_path_buf(self: Box<Self>) -> PathBuf {
        let raw = Box::into_raw(self) as *mut OsStr;
        // SAFETY: `Path` is a transparent wrapper of `OsStr`.
        PathBuf {
            inner: unsafe { Box::from_raw(raw) }.into_os_string(),
        }
    }
}

impl PathBuf {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: OsString::new(),
        }
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: OsString::with_capacity(capacity),
        }
    }

    #[must_use]
    pub fn as_path(&self) -> &Path {
        self
    }

    pub fn as_mut_path(&mut self) -> &mut Path {
        self
    }

    /// Extends the path with `path`; an absolute `path` replaces it.
    pub fn push<P: AsRef<Path>>(&mut self, path: P) {
        let path = path.as_ref();
        if path.is_absolute() {
            self.inner.clear();
        } else if self.inner.bytes().last().is_some_and(|b| !is_sep_byte(*b)) {
            self.inner.push(MAIN_SEPARATOR_STR);
        }
        self.inner.push(path.as_os_str());
    }

    /// Removes the final component, answering whether one was there.
    pub fn pop(&mut self) -> bool {
        match self.parent().map(|p| p.bytes().len()) {
            Some(len) => {
                self.inner.truncate(len);
                true
            }
            None => false,
        }
    }

    pub fn set_file_name<S: AsRef<OsStr>>(&mut self, file_name: S) {
        if self.file_name().is_some() {
            let popped = self.pop();
            debug_assert!(popped);
        }
        self.push(file_name.as_ref());
    }

    /// Replaces the extension of the final file name, answering whether a file name was there.
    pub fn set_extension<S: AsRef<OsStr>>(&mut self, extension: S) -> bool {
        let Some(stem) = self.file_stem() else {
            return false;
        };
        let end =
            stem.bytes().as_ptr() as usize + stem.len() - self.inner.bytes().as_ptr() as usize;
        self.inner.truncate(end);
        let extension = extension.as_ref();
        if !extension.is_empty() {
            self.inner.push(".");
            self.inner.push(extension);
        }
        true
    }

    /// Appends `extension` after a `.` to the final file name.
    pub fn add_extension<S: AsRef<OsStr>>(&mut self, extension: S) -> bool {
        if self.file_name().is_none() {
            return false;
        }
        let extension = extension.as_ref();
        if !extension.is_empty() {
            while self.inner.bytes().last().is_some_and(|b| is_sep_byte(*b)) {
                let len = self.inner.len() - 1;
                self.inner.truncate(len);
            }
            self.inner.push(".");
            self.inner.push(extension);
        }
        true
    }

    #[must_use]
    pub fn into_os_string(self) -> OsString {
        self.inner
    }

    #[must_use]
    pub fn into_boxed_path(self) -> Box<Path> {
        let raw = Box::into_raw(self.inner.into_boxed_os_str()) as *mut Path;
        // SAFETY: `Path` is a transparent wrapper of `OsStr`.
        unsafe { Box::from_raw(raw) }
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional);
    }

    pub fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit();
    }

    pub fn as_mut_os_string(&mut self) -> &mut OsString {
        &mut self.inner
    }
}

impl Deref for PathBuf {
    type Target = Path;

    fn deref(&self) -> &Path {
        Path::new(&self.inner)
    }
}

impl DerefMut for PathBuf {
    fn deref_mut(&mut self) -> &mut Path {
        let os_str: &mut OsStr = &mut self.inner;
        // SAFETY: `Path` is a transparent wrapper of `OsStr`.
        unsafe { &mut *(core::ptr::from_mut::<OsStr>(os_str) as *mut Path) }
    }
}

impl Borrow<Path> for PathBuf {
    fn borrow(&self) -> &Path {
        self
    }
}

impl ToOwned for Path {
    type Owned = PathBuf;

    fn to_owned(&self) -> PathBuf {
        self.to_path_buf()
    }
}

impl AsRef<OsStr> for Path {
    fn as_ref(&self) -> &OsStr {
        &self.inner
    }
}

impl AsRef<OsStr> for PathBuf {
    fn as_ref(&self) -> &OsStr {
        &self.inner
    }
}

impl AsRef<Self> for Path {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AsRef<Path> for PathBuf {
    fn as_ref(&self) -> &Path {
        self
    }
}

impl AsRef<Path> for OsStr {
    fn as_ref(&self) -> &Path {
        Path::from_os_str(self)
    }
}

impl AsRef<Path> for OsString {
    fn as_ref(&self) -> &Path {
        Path::from_os_str(self)
    }
}

impl AsRef<Path> for Cow<'_, OsStr> {
    fn as_ref(&self) -> &Path {
        Path::from_os_str(self)
    }
}

impl AsRef<Path> for str {
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}

impl AsRef<Path> for String {
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}

impl<T: ?Sized + AsRef<OsStr>> From<&T> for PathBuf {
    fn from(s: &T) -> Self {
        Self {
            inner: s.as_ref().to_os_string(),
        }
    }
}

impl From<OsString> for PathBuf {
    fn from(inner: OsString) -> Self {
        Self { inner }
    }
}

impl From<String> for PathBuf {
    fn from(s: String) -> Self {
        Self {
            inner: OsString::from(s),
        }
    }
}

impl From<PathBuf> for OsString {
    fn from(path: PathBuf) -> Self {
        path.inner
    }
}

impl From<Box<Path>> for PathBuf {
    fn from(boxed: Box<Path>) -> Self {
        boxed.into_path_buf()
    }
}

impl From<PathBuf> for Box<Path> {
    fn from(path: PathBuf) -> Self {
        path.into_boxed_path()
    }
}

impl From<&Path> for Box<Path> {
    fn from(path: &Path) -> Self {
        path.to_path_buf().into_boxed_path()
    }
}

impl Clone for Box<Path> {
    fn clone(&self) -> Self {
        self.to_path_buf().into_boxed_path()
    }
}

impl From<PathBuf> for Arc<Path> {
    fn from(path: PathBuf) -> Self {
        let arc: Arc<OsStr> = Arc::from(path.inner);
        // SAFETY: `Path` is a transparent wrapper of `OsStr`.
        unsafe { Self::from_raw(Arc::into_raw(arc) as *const Path) }
    }
}

impl From<&Path> for Arc<Path> {
    fn from(path: &Path) -> Self {
        Self::from(path.to_path_buf())
    }
}

impl From<PathBuf> for Rc<Path> {
    fn from(path: PathBuf) -> Self {
        let rc: Rc<OsStr> = Rc::from(path.inner);
        // SAFETY: `Path` is a transparent wrapper of `OsStr`.
        unsafe { Self::from_raw(Rc::into_raw(rc) as *const Path) }
    }
}

impl<'a> From<&'a Path> for Cow<'a, Path> {
    fn from(path: &'a Path) -> Self {
        Cow::Borrowed(path)
    }
}

impl From<PathBuf> for Cow<'_, Path> {
    fn from(path: PathBuf) -> Self {
        Cow::Owned(path)
    }
}

impl<'a> From<&'a PathBuf> for Cow<'a, Path> {
    fn from(path: &'a PathBuf) -> Self {
        Cow::Borrowed(path.as_path())
    }
}

impl From<Cow<'_, Path>> for PathBuf {
    fn from(path: Cow<'_, Path>) -> Self {
        path.into_owned()
    }
}

impl FromStr for PathBuf {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s))
    }
}

impl<P: AsRef<Path>> FromIterator<P> for PathBuf {
    fn from_iter<I: IntoIterator<Item = P>>(iter: I) -> Self {
        let mut buf = Self::new();
        buf.extend(iter);
        buf
    }
}

impl<P: AsRef<Path>> Extend<P> for PathBuf {
    fn extend<I: IntoIterator<Item = P>>(&mut self, iter: I) {
        for path in iter {
            self.push(path);
        }
    }
}

impl<'a> IntoIterator for &'a Path {
    type Item = &'a OsStr;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a PathBuf {
    type Item = &'a OsStr;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        self.components() == other.components()
    }
}

impl Eq for Path {}

impl PartialOrd for Path {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Path {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.components().cmp(other.components())
    }
}

impl hash::Hash for Path {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        for component in self.components() {
            component.hash(state);
        }
    }
}

impl PartialEq for PathBuf {
    fn eq(&self, other: &Self) -> bool {
        self.as_path() == other.as_path()
    }
}

impl Eq for PathBuf {}

impl PartialOrd for PathBuf {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PathBuf {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_path().cmp(other.as_path())
    }
}

impl hash::Hash for PathBuf {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_path().hash(state);
    }
}

macro_rules! impl_cmp {
    ($lhs:ty, $rhs:ty) => {
        impl PartialEq<$rhs> for $lhs {
            fn eq(&self, other: &$rhs) -> bool {
                <Path as PartialEq>::eq(self.as_ref(), other.as_ref())
            }
        }

        impl PartialEq<$lhs> for $rhs {
            fn eq(&self, other: &$lhs) -> bool {
                <Path as PartialEq>::eq(self.as_ref(), other.as_ref())
            }
        }
    };
}

impl_cmp!(PathBuf, Path);
impl_cmp!(PathBuf, &Path);
impl_cmp!(Cow<'_, Path>, Path);
impl_cmp!(Cow<'_, Path>, &Path);
impl_cmp!(Cow<'_, Path>, PathBuf);

impl fmt::Debug for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl fmt::Debug for PathBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_path(), f)
    }
}

impl fmt::Display for Display<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.path.inner.display(), f)
    }
}

impl fmt::Debug for Display<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.path, f)
    }
}

/// `path` made absolute against the working directory, its `.` components removed.
pub fn absolute<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    let path = path.as_ref();
    let mut absolute = if path.is_absolute() {
        PathBuf::new()
    } else {
        crate::env::current_dir()?
    };
    for component in path.components() {
        match component {
            Component::RootDir => absolute.push(MAIN_SEPARATOR_STR),
            Component::CurDir | Component::Prefix(_) => {}
            Component::ParentDir | Component::Normal(_) => absolute.push(component),
        }
    }
    if absolute.as_os_str().is_empty() {
        absolute.push(MAIN_SEPARATOR_STR);
    }
    Ok(absolute)
}

/// The bytes of `path`, as the host takes them.
pub(crate) const fn host_bytes(path: &Path) -> &[u8] {
    path.bytes()
}

/// A path from the bytes the host answers.
pub(crate) fn from_host_bytes(bytes: Vec<u8>) -> PathBuf {
    PathBuf {
        inner: OsString::from_vec(bytes),
    }
}
