
use std::{
    io::{Read, Write},
    marker::PhantomData,
    os::unix::net::UnixStream,
    path::PathBuf,
    process::Command,
};

use serde::{Serialize, Deserialize};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

//
// Params (these mirror chook-shim)
//

const ALLOWED_CALLING_PID: &str = "CHOOK__INTERNAL__ALLOWED_CALLING_PID";
const SOCKET_PATH: &str = "CHOOK__INTERNAL__SOCKET_PATH";
const LOG_TO: &str = "CHOOK__INTERNAL__LOG_TO";

//
// Errors
//

macro_rules! cerr {
    ($($arg:tt)*) => {{
        ChookError::Err { msg: format!($($arg)*) }
    }}
}

#[non_exhaustive]
#[derive(Debug)]
pub enum ChookError {
    Err { msg: String },
    __NonExhaustive,
}

impl std::fmt::Display for ChookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            ChookError::Err { msg } => write!(f, "{}", msg)?,
            _ => write!(f, "{:?}", self)?,
        }

        Ok(())
    }
}

impl std::error::Error for ChookError {}

//
// Main Implementation
//

/// A handle that can be used to call a routine injected into
/// a child binary.
pub struct Hook<ArgT, RetT> {
    /// Only stored for lifetime extension purposes.
    _control_socket_dir: tempfile::TempDir,
    control_socket: PathBuf,
    _arg_marker: PhantomData<ArgT>,
    _ret_marker: PhantomData<RetT>,
}

impl<ArgT, RetT> Hook<ArgT, RetT>
    where ArgT: Serialize,
        for<'de> RetT: Deserialize<'de>
{
    /// Register hooklib with the given command. Hooklib
    /// will be injected into the child process when cmd
    /// is run, and the Hook instance returned will
    /// be able to be used in order to call into the
    /// library.
    ///
    /// WARNING: There is no type checking making sure that
    /// ArgT and RetT match up with the ArgT and RetT
    /// compiled into hooklib, so be sure that they match up.
    pub fn new(
        cmd: &mut Command,
        hooklib: PathBuf,
    ) -> Result<Self, ChookError> {
        Self::new_with_log_mode(cmd, hooklib, LogMode::None)
    }

    // Create a chook handle with a custom log mode for debugging
    // the shim.
    pub fn new_with_log_mode(
        cmd: &mut Command,
        hooklib: PathBuf,
        log_mode: LogMode,
    ) -> Result<Self, ChookError> {
        // TODO: will this actually work, or will it get injected
        // after linking?
        cmd.env("LD_PRELOAD", hooklib);

        let self_pid = nix::unistd::Pid::this();
        cmd.env(ALLOWED_CALLING_PID, format!("{}", self_pid));

        let control_socket_dir = tempfile::TempDir::new()
            .map_err(|e| cerr!("creating control socket dir: {:?}", e))?;
        let mut control_socket = PathBuf::from(control_socket_dir.path());
        control_socket.push("chook_control.sock");
        cmd.env(SOCKET_PATH, &control_socket);

        match log_mode {
            LogMode::File(path) =>
                cmd.env(LOG_TO, format!("file://{:?}", path)),
            LogMode::Stdout => cmd.env(LOG_TO, "stdout"),
            LogMode::Stderr => cmd.env(LOG_TO, "stderr"),
            LogMode::None => cmd,
        };

        Ok(Hook {
            _control_socket_dir: control_socket_dir,
            control_socket,
            _arg_marker: PhantomData,
            _ret_marker: PhantomData,
        })
    }

    pub fn call(&self, arg: ArgT) -> Result<RetT, ChookError>
        where ArgT: Serialize,
              for<'de> RetT: Deserialize<'de>
    {
        let mut stream = UnixStream::connect(&self.control_socket)
            .map_err(|e| cerr!("dialing control socket: {:?}", e))?;

        // write the arg
        let arg_buf = bincode::serialize(&arg)
            .map_err(|e| cerr!("serializing arg: {:?}", e))?;
        stream.write_i64::<LittleEndian>(arg_buf.len() as i64)
            .map_err(|e| cerr!("writing ret length: {:?}", e))?;
        stream.write_all(&arg_buf)
            .map_err(|e| cerr!("writing ret: {:?}", e))?;

        // read the ret
        let ret_length = stream.read_i64::<LittleEndian>()
            .map_err(|e| cerr!("reading ret length: {:?}", e))?;
        let mut ret_buf = vec![0; ret_length as usize];
        stream.read_exact(ret_buf.as_mut_slice())
            .map_err(|e| cerr!("reading ret body: {:?}", e))?;
        let ret = bincode::deserialize(&ret_buf[..])
            .map_err(|e| cerr!("deserializing ret: {:?}", e))?;

        Ok(ret)
    }
}

/// Indicates how the chook shim should log. You usually
/// don't need to worry about this, but it might be useful
/// for debugging.
pub enum LogMode {
    File(PathBuf),
    Stdout,
    Stderr,
    None,
}
