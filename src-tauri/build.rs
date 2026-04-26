fn main() {
    // Lib test executables do not get Tauri's app manifest resource.
    // Apply the manifest dependency at link time so Windows resolves TaskDialogIndirect.
    println!(
        "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
    );
    tauri_build::build()
}
