
use std::{
    env,
    io::{Read, Write},
    fs::File,
    os::unix::net::{UnixStream, UnixListener},
};

use serde::{Serialize, Deserialize};
use nix::sys::socket;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

//
// Params (these mirror chook)
//

const ALLOWED_CALLING_PID: &str = "CHOOK__INTERNAL__ALLOWED_CALLING_PID";
const SOCKET_PATH: &str = "CHOOK__INTERNAL__SOCKET_PATH";
const LOG_TO: &str = "CHOOK__INTERNAL__LOG_TO";

//
// Macros
//

macro_rules! cerr {
    ($($arg:tt)*) => {{
        ChookError { msg: format!($($arg)*) }
    }}
}

macro_rules! log {
    ($logger:expr, $($arg:tt)*) => {
        $logger.log(&format!($($arg)*))
    }
}

//
// Main Implementation
//

/// This routine must be called in the `_init` routine
/// exported by the shim .so file.
pub fn run<F, ArgT, RetT>(f: F)
    where F: Fn(ArgT) -> RetT,
          for<'de> ArgT: Deserialize<'de>,
          RetT: Serialize
{
    let log_target = env::var(LOG_TO);
    let mut logger = match log_target.as_ref().map(|v| v.as_str()) {
        Ok("stderr") => Logger::new(LogSink::Stderr),
        Ok("stdout") => Logger::new(LogSink::Stdout),
        Ok(path) if path.starts_with("file://") => {
            match path.strip_prefix("file://")
                    .ok_or(cerr!("stripping file path"))
                    .and_then(|f| LogSink::file(f)) {
                Ok(sink) => Logger::new(sink),
                Err(_) => {
                    // no logger, so we can't even report it! Oh no!
                    Logger::new(LogSink::None)
                }
            }
        },
        _ => Logger::new(LogSink::None),
    };

    if let Err(err) = run_impl(&mut logger, f) {
        log!(logger, "ERROR: {:?}", err);
    }
}

/// The actual implementation, split out so we can do result
/// based error handling.
fn run_impl<F, ArgT, RetT>(l: &mut Logger, mut f: F) -> Result<(), ChookError>
    where F: Fn(ArgT) -> RetT,
          for<'de> ArgT: Deserialize<'de>,
          RetT: Serialize
{
    // Extract the parameters, and strip them from the
    // environment so they do not interfear with the running
    // of the process we are injected into.
    let allowed_calling_pid = env::var(ALLOWED_CALLING_PID)
        .map_err(|e| cerr!("getting allowed_calling_pid: {:?}", e))?;
    let socket_path = env::var(SOCKET_PATH)
        .map_err(|e| cerr!("getting socket_path: {:?}", e))?;
    env::remove_var(ALLOWED_CALLING_PID);
    env::remove_var(SOCKET_PATH);
    env::remove_var(LOG_TO);

    let allowed_calling_pid: i32 = allowed_calling_pid.parse()
        .map_err(|e| cerr!("parsing allowed calling pid: {:?}", e))?;

    let sock = UnixListener::bind(socket_path)
        .map_err(|e| cerr!("binding rpc socket: {:?}", e))?;
    log!(l, "bound socket");
    for stream in sock.incoming() {
        match stream {
            Ok(mut s) => handle(l, &mut s, allowed_calling_pid, &mut f)?,
            Err(e) => log!(l, "ERROR: accepting connection: {:?}", e),
        }
    }

    Err(cerr!("unexpected loop termination"))
}

fn handle<F, ArgT, RetT>(
    l: &mut Logger,
    stream: &mut UnixStream,
    allowed_calling_pid: i32,
    f: &mut F
) -> Result<(), ChookError>
    where F: Fn(ArgT) -> RetT,
          for<'de> ArgT: Deserialize<'de>,
          RetT: Serialize
{
    // Check to make sure we are not getting called by a rando.
    // This is not watertight because the parent might have disowned
    // us and exited, and then its PID gotten recycled, but it is
    // probably the best we can do.
    let peer_creds = socket::getsockopt(stream, socket::sockopt::PeerCredentials)
        .map_err(|e| cerr!("could not get peer creds from socket: {:?}", e))?;
    if peer_creds.uid() as i32 != allowed_calling_pid {
        return Err(cerr!("access denied: bad PID"));
    }

    let arg_length = stream.read_i64::<LittleEndian>()
        .map_err(|e| cerr!("reading arg length: {:?}", e))?;
    let mut arg_buf = vec![0; arg_length as usize];
    stream.read_exact(arg_buf.as_mut_slice())
        .map_err(|e| cerr!("reading arg body: {:?}", e))?;
    log!(l, "read arg length = {}", arg_length);
    let arg = bincode::deserialize(&arg_buf[..])
        .map_err(|e| cerr!("deserializing arg: {:?}", e))?;

    // actually call the user code
    let ret = f(arg);

    let ret_buf = bincode::serialize(&ret)
        .map_err(|e| cerr!("serializing ret: {:?}", e))?;
    stream.write_i64::<LittleEndian>(ret_buf.len() as i64)
        .map_err(|e| cerr!("writing ret length: {:?}", e))?;
    stream.write_all(&ret_buf)
        .map_err(|e| cerr!("writing ret: {:?}", e))?;
    log!(l, "wrote ret length = {}", arg_length);

    Ok(())
}

//
// Errors & Logging
//

enum LogSink {
    File(File),
    Stdout,
    Stderr,
    None,
}

impl LogSink {
    fn file(path: &str) -> Result<LogSink, ChookError> {
        let file = File::create(path).map_err(|e| cerr!("creating log file: {:?}", e))?;
        Ok(LogSink::File(file))
    }
}

struct Logger {
    sink: LogSink,
}

impl Logger {
    fn new(sink: LogSink) -> Self {
        Logger {
            sink,
        }
    }

    fn log(&mut self, message: &str) {
        match &mut self.sink {
            LogSink::File(f) => {
                let _ = writeln!(f, "{}", message);
            },
            LogSink::Stderr => eprintln!("{}", message),
            LogSink::Stdout => println!("{}", message),
            LogSink::None => {}
        }
    }
}

#[non_exhaustive]
#[derive(Debug)]
struct ChookError {
    msg: String,
}

impl std::fmt::Display for ChookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self.msg)?;
        Ok(())
    }
}

impl std::error::Error for ChookError {}
