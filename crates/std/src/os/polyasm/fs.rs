//! `stat(2)` fields, permission modes and open flags of files.

use crate::{
    consts::{BLOCK_SIZE, NANOS_PER_SEC, STAT_BLOCK_UNIT},
    fs::{DirEntry, FileType, Metadata, OpenOptions, Permissions},
};

mod sealed {
    pub trait Sealed {}

    impl Sealed for crate::fs::DirEntry {}
    impl Sealed for crate::fs::FileType {}
    impl Sealed for crate::fs::Metadata {}
    impl Sealed for crate::fs::OpenOptions {}
    impl Sealed for crate::fs::Permissions {}
}

/// The `stat(2)` fields of [`Metadata`].
pub trait MetadataExt: sealed::Sealed {
    fn dev(&self) -> u64;
    fn ino(&self) -> u64;
    fn mode(&self) -> u32;
    fn nlink(&self) -> u64;
    fn uid(&self) -> u32;
    fn gid(&self) -> u32;
    fn rdev(&self) -> u64;
    fn size(&self) -> u64;
    fn atime(&self) -> i64;
    fn atime_nsec(&self) -> i64;
    fn mtime(&self) -> i64;
    fn mtime_nsec(&self) -> i64;
    fn ctime(&self) -> i64;
    fn ctime_nsec(&self) -> i64;
    fn blksize(&self) -> u64;
    fn blocks(&self) -> u64;
}

impl MetadataExt for Metadata {
    fn dev(&self) -> u64 {
        self.stat().dev
    }

    fn ino(&self) -> u64 {
        self.stat().ino
    }

    fn mode(&self) -> u32 {
        self.stat().mode
    }

    fn nlink(&self) -> u64 {
        self.stat().nlink
    }

    fn uid(&self) -> u32 {
        self.stat().uid
    }

    fn gid(&self) -> u32 {
        self.stat().gid
    }

    fn rdev(&self) -> u64 {
        0
    }

    fn size(&self) -> u64 {
        self.stat().size
    }

    fn atime(&self) -> i64 {
        self.stat().atime_ns.div_euclid(NANOS_PER_SEC)
    }

    fn atime_nsec(&self) -> i64 {
        self.stat().atime_ns.rem_euclid(NANOS_PER_SEC)
    }

    fn mtime(&self) -> i64 {
        self.stat().mtime_ns.div_euclid(NANOS_PER_SEC)
    }

    fn mtime_nsec(&self) -> i64 {
        self.stat().mtime_ns.rem_euclid(NANOS_PER_SEC)
    }

    fn ctime(&self) -> i64 {
        self.stat().ctime_ns.div_euclid(NANOS_PER_SEC)
    }

    fn ctime_nsec(&self) -> i64 {
        self.stat().ctime_ns.rem_euclid(NANOS_PER_SEC)
    }

    fn blksize(&self) -> u64 {
        BLOCK_SIZE
    }

    fn blocks(&self) -> u64 {
        self.stat().size.div_ceil(STAT_BLOCK_UNIT)
    }
}

/// The file kinds of [`FileType`] beyond files, directories and links.
pub trait FileTypeExt: sealed::Sealed {
    fn is_block_device(&self) -> bool;
    fn is_char_device(&self) -> bool;
    fn is_fifo(&self) -> bool;
    fn is_socket(&self) -> bool;
}

impl FileTypeExt for FileType {
    fn is_block_device(&self) -> bool {
        Self::is_block_device(*self)
    }

    fn is_char_device(&self) -> bool {
        Self::is_char_device(*self)
    }

    fn is_fifo(&self) -> bool {
        Self::is_fifo(*self)
    }

    fn is_socket(&self) -> bool {
        Self::is_socket(*self)
    }
}

/// The mode bits of [`Permissions`].
pub trait PermissionsExt: sealed::Sealed {
    fn mode(&self) -> u32;
    fn set_mode(&mut self, mode: u32);
    fn from_mode(mode: u32) -> Self;
}

impl PermissionsExt for Permissions {
    fn mode(&self) -> u32 {
        Self::mode(self)
    }

    fn set_mode(&mut self, mode: u32) {
        Self::set_mode(self, mode);
    }

    fn from_mode(mode: u32) -> Self {
        Self::from_mode(mode)
    }
}

/// Creation mode and raw `open(2)` flags of [`OpenOptions`].
pub trait OpenOptionsExt: sealed::Sealed {
    fn mode(&mut self, mode: u32) -> &mut Self;
    fn custom_flags(&mut self, flags: i32) -> &mut Self;
}

impl OpenOptionsExt for OpenOptions {
    fn mode(&mut self, mode: u32) -> &mut Self {
        self.set_mode(mode)
    }

    fn custom_flags(&mut self, flags: i32) -> &mut Self {
        self.set_custom_flags(flags)
    }
}

/// The inode number of a [`DirEntry`].
pub trait DirEntryExt: sealed::Sealed {
    fn ino(&self) -> u64;
}

impl DirEntryExt for DirEntry {
    fn ino(&self) -> u64 {
        self.metadata().map_or(0, |m| m.ino())
    }
}
