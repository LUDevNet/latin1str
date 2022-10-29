//! # latin1str
//!
//! This crate is a thin wrapper around [`encoding_rs`](https://docs.rs/encoding_rs) that provides
//! types to work with WINDOWS-1252 (aka Latin-1) encoded strings.
//!
//! The main points about these types is that they:
//!
//! - Are not nul-terminated
//! - Contain no nul-bytes
//! - Are infallibly convertible to UTF-8
//! - Are infallibly convertible from ASCII
//! - Are infallibly convertible from a [`[u8]`][slice]
//!
//! You can use this if none of the following alternatives fit:
//!
//! - [`CStr`][`std::ffi::CStr`], which requires trailing nul-bytes
//! - [`str`], which is UTF-8 encoded
//! - [`[u8]`][slice], which lacks a defined encoding
//!
//! There are two types provided:
//!
//! - [`Latin1String`] based on [`String`]
//! - [`Latin1Str`] based on [`str`]

#![warn(missing_docs)]

use std::{
    borrow::{Borrow, Cow},
    fmt,
    io::{self, BufRead},
    ops::Deref,
};

use encoding_rs::WINDOWS_1252;
use memchr::memchr;

#[repr(transparent)]
#[derive(Ord, PartialOrd, Eq, PartialEq)]
/// An owned latin-1 encoded string
pub struct Latin1String {
    inner: Box<[u8]>,
}

impl Latin1String {
    /// Create a new string
    ///
    /// ## Safety
    ///
    /// Must not contain null bytes
    pub const unsafe fn new(inner: Box<[u8]>) -> Self {
        Self { inner }
    }

    /// Create a new instance from a rust string.
    ///
    /// **Note**: This encodes any unavailable unicode codepoints as their equivalent HTML-Entity.
    /// This is an implementation detail of the `encoding_rs` crate and not really useful for this crate.
    ///
    /// ```
    /// use latin1str::Latin1String;
    ///
    /// assert_eq!(Latin1String::encode("Hello World!").as_bytes(), b"Hello World!");
    /// assert_eq!(Latin1String::encode("Frühling").as_bytes(), b"Fr\xFChling");
    /// ```
    pub fn encode(string: &str) -> Cow<Latin1Str> {
        let (res, _, _) = WINDOWS_1252.encode(string);
        match res {
            Cow::Owned(o) => Cow::Owned(Self {
                inner: o.into_boxed_slice(),
            }),
            Cow::Borrowed(b) => Cow::Borrowed(unsafe { Latin1Str::from_bytes_unchecked(b) }),
        }
    }

    /// Create a new instance by reading from a [`BufRead`] until a null terminator is found
    ///
    /// ```
    /// use std::io::{Read, Cursor};
    /// use latin1str::Latin1String;
    ///
    /// let bytes = b"Hello World!\0";
    /// let mut cur = Cursor::new(bytes);
    /// let s = Latin1String::read_cstring(&mut cur).unwrap();
    /// assert_eq!(s.decode().as_ref(), "Hello World!");
    /// assert_eq!(cur.read(&mut []).ok(), Some(0));
    /// ```
    pub fn read_cstring<R: BufRead>(reader: &mut R) -> Result<Self, io::Error> {
        let mut string: Vec<u8> = Vec::new();
        reader.read_until(0x00, &mut string)?;
        if string.ends_with(&[0x00]) {
            string.pop();
        }
        Ok(Self {
            inner: string.into_boxed_slice(),
        })
    }
}

impl Borrow<Latin1Str> for Latin1String {
    fn borrow(&self) -> &Latin1Str {
        unsafe { Latin1Str::from_bytes_unchecked(&self.inner) }
    }
}

impl Deref for Latin1String {
    type Target = Latin1Str;

    fn deref(&self) -> &Self::Target {
        self.borrow()
    }
}

impl From<Cow<'_, Latin1Str>> for Latin1String {
    fn from(cow: Cow<'_, Latin1Str>) -> Self {
        cow.into_owned()
    }
}

impl From<&Latin1Str> for Latin1String {
    fn from(src: &Latin1Str) -> Latin1String {
        src.to_owned()
    }
}

#[repr(transparent)]
#[derive(PartialEq, PartialOrd, Eq, Ord)]
/// A borrowed latin-1 encoded string (like `&str`)
pub struct Latin1Str {
    #[allow(dead_code)]
    inner: [u8],
}

#[cfg(feature = "serde-derives")]
impl serde::Serialize for Latin1Str {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.decode().as_ref())
    }
}

impl fmt::Debug for &'_ Latin1Str {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.decode().fmt(f)
    }
}

impl ToOwned for Latin1Str {
    type Owned = Latin1String;

    fn to_owned(&self) -> Self::Owned {
        Latin1String {
            inner: self.as_bytes().into(),
        }
    }
}

impl Latin1Str {
    /// Turns some bytes into a Latin1Str slice
    ///
    /// ## Safety
    ///
    /// The byte slice may not contain any null bytes
    pub const unsafe fn from_bytes_unchecked(text: &[u8]) -> &Self {
        &*(text as *const [u8] as *const Latin1Str)
    }

    /// Wrap all bytes before the first nul as a [`Latin1Str`]
    ///
    /// This method will never fail
    /// 
    /// ```
    /// # use latin1str::Latin1Str;
    /// let s = Latin1Str::from_bytes_until_nul(b"Hello\0World!");
    /// assert_eq!(s.as_bytes(), b"Hello");
    /// let s = Latin1Str::from_bytes_until_nul(b"Hello World!");
    /// assert_eq!(s.as_bytes(), b"Hello World!");
    /// ```
    pub fn from_bytes_until_nul(mut bytes: &[u8]) -> &Self {
        if let Some(nullpos) = memchr(0, bytes) {
            bytes = bytes.split_at(nullpos).0;
        }
        // SAFETY: if there was a nul in here, the if above would have removed it
        unsafe { Self::from_bytes_unchecked(bytes) }
    }

    #[deprecated = "Use `from_bytes_until_nul` instead"]
    /// Alias of [`Latin1Str::from_bytes_until_nul`]
    pub fn new(bytes: &[u8]) -> &Self {
        Self::from_bytes_until_nul(bytes)
    }


    /// Get the bytes of the string
    /// 
    /// ```
    /// # use latin1str::Latin1Str;
    /// let s = Latin1Str::from_bytes_until_nul(b"Hello World!");
    /// assert_eq!(s.as_bytes(), b"Hello World!")
    /// ```
    pub const fn as_bytes(&self) -> &[u8] {
        &self.inner
    }

    /// Get the bytes of the string
    pub const fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check whether the str is empty
    /// 
    /// ```
    /// # use latin1str::Latin1Str;
    /// assert!(Latin1Str::from_bytes_until_nul(b"").is_empty());
    /// assert!(!Latin1Str::from_bytes_until_nul(b"a").is_empty());
    /// ```
    pub const fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Decode the string
    /// 
    /// ```
    /// # use latin1str::Latin1Str;
    /// let s = Latin1Str::from_bytes_until_nul(b"Fr\xFChling");
    /// assert_eq!(s.decode().as_ref(), "Frühling");
    /// ```
    pub fn decode(&self) -> Cow<str> {
        WINDOWS_1252.decode(self.as_bytes()).0
    }
}
