//! PolyASM extensions: Linux numbering of the host interface, byte views of OS strings, file
//! status fields, and float math.

pub mod errno;
pub mod ffi;
pub mod fs;
pub mod num;

/// `open(2)` flags, `lseek(2)` origins and the working-directory descriptor.
pub mod fcntl {
    pub use crate::consts::{
        AT_FDCWD, O_ACCMODE, O_APPEND, O_ASYNC, O_CLOEXEC, O_CREAT, O_DIRECTORY, O_DSYNC, O_EXCL,
        O_NDELAY, O_NOCTTY, O_NOFOLLOW, O_NONBLOCK, O_RDONLY, O_RDWR, O_SYNC, O_TRUNC, O_WRONLY,
        SEEK_CUR, SEEK_END, SEEK_SET,
    };
}

/// `poll(2)` events.
pub mod poll {
    pub use crate::consts::{
        POLLERR, POLLHUP, POLLIN, POLLNVAL, POLLOUT, POLLPRI, POLLRDBAND, POLLRDNORM, POLLWRBAND,
        POLLWRNORM,
    };
}

/// `stat(2)` file types and permission bits.
pub mod stat {
    pub use crate::consts::{
        S_IFBLK, S_IFCHR, S_IFDIR, S_IFIFO, S_IFLNK, S_IFMT, S_IFREG, S_IFSOCK, S_IRGRP, S_IROTH,
        S_IRUSR, S_IRWXG, S_IRWXO, S_IRWXU, S_ISGID, S_ISUID, S_ISVTX, S_IWGRP, S_IWOTH, S_IWUSR,
        S_IXGRP, S_IXOTH, S_IXUSR,
    };
}
