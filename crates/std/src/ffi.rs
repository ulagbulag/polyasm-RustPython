//! C strings, C types, and [`OsStr`]/[`OsString`] as plain bytes.
//!
//! An operating-system string of a PolyASM guest is an arbitrary byte sequence, the POSIX
//! convention; [`crate::os::polyasm::ffi`] exposes the bytes.

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
    cmp,
    fmt::{self, Write as _},
    hash,
    ops::{self, Deref, DerefMut},
    str,
};

pub use alloc_crate::ffi::{CString, FromVecWithNulError, IntoStringError, NulError};
pub use core::ffi::{
    CStr, FromBytesUntilNulError, FromBytesWithNulError, c_char, c_double, c_float, c_int, c_long,
    c_longlong, c_schar, c_short, c_uchar, c_uint, c_ulong, c_ulonglong, c_ushort, c_void,
};

/// A borrowed operating-system string: bytes of any value.
#[repr(transparent)]
pub struct OsStr {
    inner: [u8],
}

/// An owned operating-system string: bytes of any value.
#[derive(Clone, Default)]
pub struct OsString {
    inner: Vec<u8>,
}

/// [`OsStr`] shown with invalid UTF-8 replaced, made by [`OsStr::display`].
pub struct Display<'a> {
    os_str: &'a OsStr,
}

impl OsStr {
    pub fn new<S: AsRef<Self> + ?Sized>(s: &S) -> &Self {
        s.as_ref()
    }

    pub(crate) const fn from_bytes(bytes: &[u8]) -> &Self {
        // SAFETY: `OsStr` is a transparent wrapper of `[u8]`.
        unsafe { &*(core::ptr::from_ref::<[u8]>(bytes) as *const Self) }
    }

    pub(crate) const fn from_bytes_mut(bytes: &mut [u8]) -> &mut Self {
        // SAFETY: `OsStr` is a transparent wrapper of `[u8]`.
        unsafe { &mut *(core::ptr::from_mut::<[u8]>(bytes) as *mut Self) }
    }

    pub(crate) const fn bytes(&self) -> &[u8] {
        &self.inner
    }

    /// The string from its bytes.
    ///
    /// # Safety
    ///
    /// Every byte sequence is a valid operating-system string here, so the call holds for any
    /// `bytes`; the function keeps the signature of std.
    #[must_use]
    pub const unsafe fn from_encoded_bytes_unchecked(bytes: &[u8]) -> &Self {
        Self::from_bytes(bytes)
    }

    #[must_use]
    pub const fn as_encoded_bytes(&self) -> &[u8] {
        &self.inner
    }

    #[must_use]
    pub const fn to_str(&self) -> Option<&str> {
        match str::from_utf8(&self.inner) {
            Ok(s) => Some(s),
            Err(_) => None,
        }
    }

    #[must_use]
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.inner)
    }

    #[must_use]
    pub fn to_os_string(&self) -> OsString {
        OsString {
            inner: self.inner.to_vec(),
        }
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.inner.len()
    }

    #[must_use]
    pub fn into_os_string(self: Box<Self>) -> OsString {
        let raw = Box::into_raw(self) as *mut [u8];
        // SAFETY: `OsStr` is a transparent wrapper of `[u8]`.
        OsString {
            inner: unsafe { Box::from_raw(raw) }.into_vec(),
        }
    }

    #[must_use]
    pub const fn is_ascii(&self) -> bool {
        self.inner.is_ascii()
    }

    pub const fn make_ascii_lowercase(&mut self) {
        self.inner.make_ascii_lowercase();
    }

    pub const fn make_ascii_uppercase(&mut self) {
        self.inner.make_ascii_uppercase();
    }

    #[must_use]
    pub fn to_ascii_lowercase(&self) -> OsString {
        OsString {
            inner: self.inner.to_ascii_lowercase(),
        }
    }

    #[must_use]
    pub fn to_ascii_uppercase(&self) -> OsString {
        OsString {
            inner: self.inner.to_ascii_uppercase(),
        }
    }

    pub fn eq_ignore_ascii_case<S: AsRef<Self>>(&self, other: S) -> bool {
        self.inner.eq_ignore_ascii_case(&other.as_ref().inner)
    }

    #[must_use]
    pub const fn display(&self) -> Display<'_> {
        Display { os_str: self }
    }
}

impl OsString {
    #[must_use]
    pub const fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub(crate) const fn from_vec(inner: Vec<u8>) -> Self {
        Self { inner }
    }

    pub(crate) fn into_vec(self) -> Vec<u8> {
        self.inner
    }

    /// The string from its bytes.
    ///
    /// # Safety
    ///
    /// Every byte sequence is a valid operating-system string here, so the call holds for any
    /// `bytes`; the function keeps the signature of std.
    #[must_use]
    pub const unsafe fn from_encoded_bytes_unchecked(bytes: Vec<u8>) -> Self {
        Self { inner: bytes }
    }

    #[must_use]
    pub fn into_encoded_bytes(self) -> Vec<u8> {
        self.inner
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Vec::with_capacity(capacity),
        }
    }

    #[must_use]
    pub fn as_os_str(&self) -> &OsStr {
        OsStr::from_bytes(&self.inner)
    }

    pub fn as_mut_os_str(&mut self) -> &mut OsStr {
        OsStr::from_bytes_mut(&mut self.inner)
    }

    /// The string as UTF-8 text, or the string itself when its bytes are other than UTF-8.
    pub fn into_string(self) -> Result<String, Self> {
        String::from_utf8(self.inner).map_err(|e| Self {
            inner: e.into_bytes(),
        })
    }

    pub fn push<T: AsRef<OsStr>>(&mut self, s: T) {
        self.inner.extend_from_slice(&s.as_ref().inner);
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

    pub fn reserve_exact(&mut self, additional: usize) {
        self.inner.reserve_exact(additional);
    }

    pub fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit();
    }

    pub fn truncate(&mut self, len: usize) {
        self.inner.truncate(len);
    }

    #[must_use]
    pub fn into_boxed_os_str(self) -> Box<OsStr> {
        let raw = Box::into_raw(self.inner.into_boxed_slice()) as *mut OsStr;
        // SAFETY: `OsStr` is a transparent wrapper of `[u8]`.
        unsafe { Box::from_raw(raw) }
    }
}

impl Deref for OsString {
    type Target = OsStr;

    fn deref(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl DerefMut for OsString {
    fn deref_mut(&mut self) -> &mut OsStr {
        self.as_mut_os_str()
    }
}

impl ops::Index<ops::RangeFull> for OsString {
    type Output = OsStr;

    fn index(&self, _index: ops::RangeFull) -> &OsStr {
        self.as_os_str()
    }
}

impl Borrow<OsStr> for OsString {
    fn borrow(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl ToOwned for OsStr {
    type Owned = OsString;

    fn to_owned(&self) -> OsString {
        self.to_os_string()
    }
}

impl AsRef<Self> for OsStr {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AsRef<OsStr> for OsString {
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl AsRef<OsStr> for str {
    fn as_ref(&self) -> &OsStr {
        OsStr::from_bytes(self.as_bytes())
    }
}

impl AsRef<OsStr> for String {
    fn as_ref(&self) -> &OsStr {
        OsStr::from_bytes(self.as_bytes())
    }
}

impl From<String> for OsString {
    fn from(s: String) -> Self {
        Self {
            inner: s.into_bytes(),
        }
    }
}

impl<T: ?Sized + AsRef<OsStr>> From<&T> for OsString {
    fn from(s: &T) -> Self {
        s.as_ref().to_os_string()
    }
}

impl From<Box<OsStr>> for OsString {
    fn from(boxed: Box<OsStr>) -> Self {
        boxed.into_os_string()
    }
}

impl From<OsString> for Box<OsStr> {
    fn from(s: OsString) -> Self {
        s.into_boxed_os_str()
    }
}

impl From<&OsStr> for Box<OsStr> {
    fn from(s: &OsStr) -> Self {
        s.to_os_string().into_boxed_os_str()
    }
}

impl From<OsString> for Arc<OsStr> {
    fn from(s: OsString) -> Self {
        let arc: Arc<[u8]> = Arc::from(s.inner);
        // SAFETY: `OsStr` is a transparent wrapper of `[u8]`.
        unsafe { Self::from_raw(Arc::into_raw(arc) as *const OsStr) }
    }
}

impl From<OsString> for Rc<OsStr> {
    fn from(s: OsString) -> Self {
        let rc: Rc<[u8]> = Rc::from(s.inner);
        // SAFETY: `OsStr` is a transparent wrapper of `[u8]`.
        unsafe { Self::from_raw(Rc::into_raw(rc) as *const OsStr) }
    }
}

impl<'a> From<&'a OsStr> for Cow<'a, OsStr> {
    fn from(s: &'a OsStr) -> Self {
        Cow::Borrowed(s)
    }
}

impl From<OsString> for Cow<'_, OsStr> {
    fn from(s: OsString) -> Self {
        Cow::Owned(s)
    }
}

impl From<Cow<'_, OsStr>> for OsString {
    fn from(s: Cow<'_, OsStr>) -> Self {
        s.into_owned()
    }
}

impl Clone for Box<OsStr> {
    fn clone(&self) -> Self {
        self.to_os_string().into_boxed_os_str()
    }
}

impl Default for &OsStr {
    fn default() -> Self {
        OsStr::from_bytes(&[])
    }
}

impl PartialEq for OsStr {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Eq for OsStr {}

impl PartialOrd for OsStr {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OsStr {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl hash::Hash for OsStr {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl PartialEq for OsString {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Eq for OsString {}

impl PartialOrd for OsString {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OsString {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl hash::Hash for OsString {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_os_str().hash(state);
    }
}

macro_rules! impl_cmp {
    ($lhs:ty, $rhs:ty) => {
        impl PartialEq<$rhs> for $lhs {
            fn eq(&self, other: &$rhs) -> bool {
                <OsStr as PartialEq>::eq(self.as_ref(), other.as_ref())
            }
        }

        impl PartialEq<$lhs> for $rhs {
            fn eq(&self, other: &$lhs) -> bool {
                <OsStr as PartialEq>::eq(self.as_ref(), other.as_ref())
            }
        }
    };
}

impl_cmp!(OsString, OsStr);
impl_cmp!(OsString, &OsStr);
impl_cmp!(OsString, str);
impl_cmp!(OsString, &str);
impl_cmp!(OsString, String);
impl_cmp!(OsStr, str);
impl_cmp!(OsStr, String);
impl_cmp!(&OsStr, String);

impl fmt::Debug for OsStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('"')?;
        for chunk in self.inner.utf8_chunks() {
            for c in chunk.valid().chars() {
                if c == '\'' {
                    f.write_char(c)?;
                } else {
                    for escaped in c.escape_debug() {
                        f.write_char(escaped)?;
                    }
                }
            }
            for byte in chunk.invalid() {
                write!(f, "\\x{byte:02X}")?;
            }
        }
        f.write_char('"')
    }
}

impl fmt::Debug for OsString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_os_str(), f)
    }
}

impl fmt::Display for Display<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.os_str.inner.is_empty() {
            return f.pad("");
        }
        for chunk in self.os_str.inner.utf8_chunks() {
            f.write_str(chunk.valid())?;
            if !chunk.invalid().is_empty() {
                f.write_str("\u{FFFD}")?;
            }
        }
        Ok(())
    }
}

impl fmt::Debug for Display<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.os_str, f)
    }
}

impl<T: AsRef<OsStr>> Extend<T> for OsString {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for s in iter {
            self.push(s);
        }
    }
}

impl<T: AsRef<OsStr>> FromIterator<T> for OsString {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut buf = Self::new();
        buf.extend(iter);
        buf
    }
}

impl str::FromStr for OsString {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s))
    }
}

impl fmt::Write for OsString {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push(s);
        Ok(())
    }
}
