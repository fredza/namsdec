use std::process::Command;

fn main() {
    let build_date = Command::new("date")
        .arg("+%Y%m%d")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=NAMSDEC_BUILD_DATE={build_date}");
}
