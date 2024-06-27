use std::{
    fs,
    env,
    path::{Path, PathBuf},
    io::Write,
    process,
    process::Command,
};

fn main() {
    let overlay_so = OverlaySo::new().expect("to produce an overlay");

    let mut cmd = Command::new("cat");
    cmd
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::inherit());
    let hook = chook::Hook::<types::Arg, types::Ret>::new(&mut cmd, overlay_so.path())
        .expect("hook creation to succeed");

    let mut proc = cmd.spawn().expect("child to spawn");

    let ret = hook.call(types::Arg {
        print_this_string: String::from("print me from the hook"),
        trim_this_string: String::from("    oh no, whitespace     "),
        print_this_int: 42,
        double_this_int: 5,
    }).expect("call to succeed");

    println!("ret = {:?}", ret);

    proc.kill().expect("to kill child");
    proc.wait().expect("to reap child");
}

/// A handle to an overlay .so file. It is normally stored as embedded data in the motd
/// rlib, but for the life of one of these handles it gets written out to a tmp file.
/// The overlay file gets cleaned up when this handle falls out of scope.
///
/// TODO: move this into the chook crate itself. Users should not need to
/// worry about this, they shouuld just be able to put the name of the shared library
/// (i.e. chookexampleshim).
struct OverlaySo {
    _overlay_dir: tempfile::TempDir,
    path: PathBuf,
}

impl OverlaySo {
    fn new() -> anyhow::Result<Self> {
        let overlay_blob = include_bytes!(concat!(env!("OUT_DIR"), "/libchookexampleshim.so"));

        let overlay_dir = tempfile::TempDir::with_prefix("chook_shim")?;
            // .map_err(|e| merr!("making tmp pam_motd_overlay.so dir: {}", e))?;
        let mut path = PathBuf::from(overlay_dir.path());
        path.push("pam_motd_overlay.so");

        let mut overlay_file = fs::File::create(&path)?;
            // .map_err(|e| merr!("making pam_motd_overlay.so: {}", e))?;
        overlay_file
            .write_all(overlay_blob)?;
            // .map_err(|e| merr!("writing pam_motd_overlay.so: {}", e))?;

        Ok(OverlaySo {
            _overlay_dir: overlay_dir,
            path,
        })
    }

    fn path(&self) -> &Path {
        self.path.as_path()
    }
}
