// spell-checker:disable

//! `posix` compatible module for `not(any(unix, windows))`

pub(crate) use module::module_def;

#[pymodule(name = "posix", with(
    super::os::_os,
    #[cfg(any(unix, target_os = "wasi", target_abi = "polyasm"))]
    super::posix_unix_like::_posix_unix_like,
))]
pub(crate) mod module {
    use crate::{
        Py, PyResult, VirtualMachine,
        builtins::PyStrRef,
        stdlib::os::{DirFd, SupportFunc, SymlinkArgs},
    };

    #[derive(FromArgs)]
    pub(super) struct AccessArgs<'a> {
        #[pyarg(any)]
        path: PyStrRef,
        #[pyarg(any)]
        mode: u8,
        #[pyarg(flatten)]
        dir_fd: DirFd<'a, 0>,
        #[pyarg(named, default)]
        effective_ids: bool,
        #[pyarg(named, default = true)]
        follow_symlinks: bool,
    }

    #[pyfunction]
    pub(super) fn access(args: AccessArgs<'_>, vm: &VirtualMachine) -> PyResult<bool> {
        let [] = args.dir_fd.0;
        let _ = (args.effective_ids, args.follow_symlinks);
        cfg_select! {
            target_abi = "polyasm" => {
                use crate::convert::IntoPyException;

                let path = args.path.to_string_lossy();
                rustpython_host_env::posix::check_access(std::path::Path::new(&*path), args.mode)
                    .map_err(|err| err.into_pyexception(vm))
            }
            _ => {
                let _ = (args.path, args.mode);
                os_unimpl("os.access", vm)
            }
        }
    }

    #[cfg(target_abi = "polyasm")]
    #[pyfunction]
    const fn getppid() -> i32 {
        rustpython_host_env::posix::getppid()
    }

    #[cfg(target_abi = "polyasm")]
    #[pyfunction]
    const fn getuid() -> u32 {
        rustpython_host_env::posix::getuid()
    }

    #[cfg(target_abi = "polyasm")]
    #[pyfunction]
    const fn geteuid() -> u32 {
        rustpython_host_env::posix::geteuid()
    }

    #[cfg(target_abi = "polyasm")]
    #[pyfunction]
    const fn getgid() -> u32 {
        rustpython_host_env::posix::getgid()
    }

    #[cfg(target_abi = "polyasm")]
    #[pyfunction]
    const fn getegid() -> u32 {
        rustpython_host_env::posix::getegid()
    }

    #[cfg(target_abi = "polyasm")]
    #[pyfunction]
    fn umask(mask: u32) -> u32 {
        rustpython_host_env::posix::umask(mask)
    }

    #[cfg(not(any(target_os = "wasi", target_abi = "polyasm")))]
    #[derive(FromArgs)]
    struct RemoveArgs<'a> {
        #[pyarg(any)]
        path: OsPath,
        #[pyarg(flatten)]
        dir_fd: DirFd<'a, 0>,
    }

    #[cfg(not(any(target_os = "wasi", target_abi = "polyasm")))]
    #[pyfunction]
    #[pyfunction(name = "unlink")]
    fn remove(args: RemoveArgs<'_>, vm: &VirtualMachine) -> PyResult<()> {
        let [] = args.dir_fd.0;
        std::fs::remove_file(&args.path).map_err(|err| err.into_pyexception(vm))
    }

    #[pyfunction]
    pub(super) fn symlink(args: SymlinkArgs<'_>, vm: &VirtualMachine) -> PyResult<()> {
        #[cfg(target_abi = "polyasm")]
        let _ = (args.src, args.dst, args.target_is_directory);
        #[cfg(not(target_abi = "polyasm"))]
        let _ = args;
        os_unimpl("os.symlink", vm)
    }

    #[allow(dead_code)]
    fn os_unimpl<T>(func: &str, vm: &VirtualMachine) -> PyResult<T> {
        Err(vm.new_os_error(format!("{func} is not supported on this platform")))
    }

    pub(crate) fn support_funcs() -> Vec<SupportFunc> {
        Vec::new()
    }

    pub(crate) fn module_exec(
        vm: &VirtualMachine,
        module: &Py<crate::builtins::PyModule>,
    ) -> PyResult<()> {
        __module_exec(vm, module);
        super::super::os::module_exec(vm, module)?;
        Ok(())
    }
}
