//! The printing and debugging macros of std, writing through the host.

/// Prints to standard output.
pub macro print($($arg:tt)*) {
    $crate::io::_print($crate::format_args!($($arg)*))
}

/// Prints to standard output, with a newline.
pub macro println {
    () => {
        $crate::io::_print($crate::format_args!("\n"))
    },
    ($($arg:tt)*) => {
        $crate::io::_print_line($crate::format_args!($($arg)*))
    },
}

/// Prints to standard error.
pub macro eprint($($arg:tt)*) {
    $crate::io::_eprint($crate::format_args!($($arg)*))
}

/// Prints to standard error, with a newline.
pub macro eprintln {
    () => {
        $crate::io::_eprint($crate::format_args!("\n"))
    },
    ($($arg:tt)*) => {
        $crate::io::_eprint_line($crate::format_args!($($arg)*))
    },
}

/// Prints an expression and its value to standard error, answering the value.
pub macro dbg {
    () => {
        $crate::eprintln!("[{}:{}:{}]", $crate::file!(), $crate::line!(), $crate::column!())
    },
    ($val:expr $(,)?) => {
        match $val {
            tmp => {
                $crate::eprintln!(
                    "[{}:{}:{}] {} = {:#?}",
                    $crate::file!(),
                    $crate::line!(),
                    $crate::column!(),
                    $crate::stringify!($val),
                    &tmp,
                );
                tmp
            }
        }
    },
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg!($val)),+,)
    },
}
