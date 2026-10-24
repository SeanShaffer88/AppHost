// Embed app.manifest on Windows with the SDK resource compiler.
//
// This is the dependency-free WinDevKit variant. A cross-target
// `cargo check` on a non-Windows host may not have rc.exe available;
// in that case the check continues with a warning. A Windows build
// with the SDK gets the manifest embedded normally.
fn main() {
    println!("cargo:rerun-if-changed=app.manifest");
    println!("cargo:rerun-if-changed=app.rc");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is set for build scripts");
    let obj = format!("{out_dir}\\app.res");
    match std::process::Command::new("rc.exe")
        .args(["/nologo", "/fo", &obj, "app.rc"])
        .status()
    {
        Ok(status) if status.success() => {
            println!("cargo:rustc-link-arg={obj}");
        }
        Ok(status) => panic!("rc.exe failed with status {status}"),
        Err(err) => {
            // Keep host-side windows-target compile checks useful on
            // machines without the Windows SDK. Windows CI has rc.exe.
            eprintln!("warning: rc.exe unavailable ({err}); app.manifest was not embedded");
        }
    }
}
