use std::{
    ffi::OsString,
    os::fd::{AsFd, BorrowedFd, OwnedFd},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
};

use anyhow::{Context, Result};
use rand::distr::{Alphanumeric, SampleString};
use rand::rng;
use rustix::process::{PidfdFlags, Signal, pidfd_open};

pub fn find_executable_in_path(name: impl AsRef<std::ffi::OsStr>) -> Option<PathBuf> {
    let name = name.as_ref();
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let full_path = dir.join(name);
            if full_path.is_file() {
                Some(full_path)
            } else {
                None
            }
        })
    })
}

pub struct Runtime {
    cmd: Command,
    run_path: PathBuf,
    /// Symlinks created by bind_mount() that should be cleaned up on drop.
    symlinks: Vec<PathBuf>,
}

impl Runtime {
    pub fn new(command: Vec<OsString>) -> Result<Self> {
        let run_path =
            std::env::temp_dir().join(format!("mm.{}", Alphanumeric.sample_string(&mut rng(), 16)));
        std::fs::create_dir_all(&run_path)?;

        let mut cmd = Command::new(&command[0]);
        cmd.args(&command[1..]);
        cmd.env_clear();

        // Set essential envs
        cmd.env("XDG_RUNTIME_DIR", &run_path);

        Ok(Self { cmd, run_path, symlinks: Vec::new() })
    }

    /// Set a pre-exec hook (runs in the child process after fork, before exec).
    ///
    /// This is a thin wrapper over `std::os::unix::process::CommandExt::pre_exec`
    /// so call sites don't need to access the inner `Command`.
    ///
    /// # Safety
    /// Same safety requirements as `CommandExt::pre_exec`.
    pub unsafe fn pre_exec<F>(&mut self, f: F)
    where
        F: FnMut() -> std::io::Result<()> + Send + Sync + 'static,
    {
        unsafe {
            self.cmd.pre_exec(f);
        }
    }

    pub fn set_env(&mut self, key: impl AsRef<std::ffi::OsStr>, val: impl AsRef<std::ffi::OsStr>) {
        self.cmd.env(key, val);
    }

    pub fn set_stdout(&mut self, stdio: impl AsFd) -> Result<()> {
        let fd = stdio.as_fd().try_clone_to_owned()?;
        self.cmd.stdout(Stdio::from(fd));
        Ok(())
    }

    pub fn set_stderr(&mut self, stdio: impl AsFd) -> Result<()> {
        let fd = stdio.as_fd().try_clone_to_owned()?;
        self.cmd.stderr(Stdio::from(fd));
        Ok(())
    }

    pub fn extern_run_path(&self) -> &Path {
        &self.run_path
    }

    #[allow(dead_code)]
    pub fn intern_run_path(&self) -> &Path {
        &self.run_path
    }

    pub fn spawn(mut self) -> Result<ProcessHandle> {
        unsafe {
            self.cmd.pre_exec(|| {
                rustix::process::set_parent_process_death_signal(Some(Signal::KILL))?;
                Ok(())
            });
        }

        let child = self.cmd.spawn().context("spawning process")?;
        let pid = rustix::process::Pid::from_raw(child.id() as i32).unwrap();
        let pidfd = pidfd_open(pid, PidfdFlags::empty()).context("opening pidfd")?;

        Ok(ProcessHandle {
            child,
            pidfd,
            run_path: self.run_path,
            symlinks: self.symlinks,
        })
    }

    /// In non-container mode, create a symlink `dst → src` so that paths
    /// expected inside a container (e.g. `/tmp/.X11-unix`) resolve correctly
    /// on the host.  The symlink is tracked and removed when the process exits.
    pub fn bind_mount(&mut self, src: impl AsRef<Path>, dst: impl AsRef<Path>) {
        let src = src.as_ref();
        let dst = dst.as_ref();

        if let Some(parent) = dst.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Remove any existing entry at dst before creating the symlink.
        let _ = std::fs::remove_dir(dst);
        let _ = std::fs::remove_file(dst);

        if std::os::unix::fs::symlink(src, dst).is_ok() {
            self.symlinks.push(dst.to_path_buf());
        }
    }

    #[allow(dead_code)]
    pub fn internal_bind_mount(&mut self, _src: impl AsRef<Path>, _dst: impl AsRef<Path>) {}

    #[allow(dead_code)]
    pub fn setup_hook(&mut self, _f: impl FnOnce(&mut ProcessHandle) -> Result<()> + 'static) {}
}

pub struct ProcessHandle {
    child: Child,
    pidfd: OwnedFd,
    run_path: PathBuf,
    symlinks: Vec<PathBuf>,
}

impl AsFd for ProcessHandle {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.pidfd.as_fd()
    }
}

impl ProcessHandle {
    pub fn pid(&self) -> rustix::process::Pid {
        rustix::process::Pid::from_raw(self.child.id() as i32).unwrap()
    }

    pub fn pidfd(&self) -> BorrowedFd<'_> {
        self.pidfd.as_fd()
    }

    pub fn signal(&mut self, signal: Signal) -> Result<()> {
        rustix::process::pidfd_send_signal(self.pidfd.as_fd(), signal).context("signaling process")
    }

    pub fn wait(&mut self) -> Result<ExitStatus> {
        self.child.wait().context("waiting for process")
    }

    #[allow(dead_code)]
    pub fn fs_mount<S>(
        &self,
        _dst: impl AsRef<Path>,
        _fstype: impl AsRef<str>,
        _attr: rustix::mount::MountAttrFlags,
        _options: impl AsRef<[(S, S)]>,
    ) -> Result<()> {
        tracing::warn!("fs_mount not supported in non-container mode");
        Ok(())
    }

    #[allow(dead_code)]
    pub fn fuse_mount(
        &self,
        _dst: impl AsRef<Path>,
        _fsname: impl AsRef<str>,
        _st_mode: u32,
    ) -> Result<OwnedFd> {
        anyhow::bail!("fuse_mount not supported in non-container mode")
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.run_path);
        // Remove symlinks created by bind_mount() (e.g. /tmp/.X11-unix).
        for link in &self.symlinks {
            let _ = std::fs::remove_file(link);
            let _ = std::fs::remove_dir(link);
        }
    }
}
