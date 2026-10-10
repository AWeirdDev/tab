use krun_sys as k;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::ptr;

fn check(call: &str, rc: i32) {
    if rc < 0 {
        eprintln!("{call}: {} ({rc})", std::io::Error::from_raw_os_error(-rc));
        std::process::exit(1);
    }
}

fn main() {
    let root = std::env::args_os()
        .nth(1)
        .expect("usage: krun-hello ROOTFS");
    let root = std::fs::canonicalize(root).expect("rootfs must exist");
    let root = CString::new(root.as_os_str().as_bytes()).expect("NUL in rootfs path");
    let exe = CString::new("/bin/hello").unwrap();
    // libkrun 1.19.6 reads an internal slice of MAX_ARGS=4096 pointers.
    // Supply full-size, zero-filled arrays, not merely a one-pointer sentinel.
    let argv = [ptr::null(); 4096]; // arguments AFTER argv[0]; init sets argv[0].
    let path = CString::new("PATH=/bin:/usr/bin").unwrap();
    let mut env = [ptr::null(); 4096];
    env[0] = path.as_ptr(); // never pass null: that inherits host environment.
    unsafe {
        check("log", k::krun_set_log_level(3));
        let ctx = k::krun_create_ctx();
        check("create", ctx);
        check("config", k::krun_set_vm_config(ctx as u32, 1, 256));
        check("root", k::krun_set_root(ctx as u32, root.as_ptr()));
        check("workdir", k::krun_set_workdir(ctx as u32, c"/".as_ptr()));
        check(
            "exec",
            k::krun_set_exec(ctx as u32, exe.as_ptr(), argv.as_ptr(), env.as_ptr()),
        );
        // Success never returns: VMM exits this process with the guest exit code.
        let rc = k::krun_start_enter(ctx as u32);
        check("start", rc);
    }
}
