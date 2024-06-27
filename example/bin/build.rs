
use std::{
    fs,
    path::PathBuf,
    env,
};

/* Uncomment for debugging, you can't print normally from a build.rs
macro_rules! bprintln {
    ($($arg:tt)*) => {{
        println!("cargo:warning={}", format!($($arg)*))
    }}
}
*/

fn main() {
    // TODO: wrap this in a helper provided by the `chook` crate.
    let mut dest = PathBuf::from(env::var("OUT_DIR").expect("to have an out dir"));
    let mut src = dest.clone();
    src.pop();
    src.pop();
    src.pop();
    dest.push("libchookexampleshim.so");
    src.push("libchookexampleshim.so");

    fs::copy(src, dest).expect("to copy the so file");
}
