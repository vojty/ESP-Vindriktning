use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Bake values from .env (searched upwards from the crate dir) into the firmware
    if let Ok(path) = dotenvy::dotenv_override() {
        for item in dotenvy::from_path_iter(&path)? {
            let (key, value) = item?;
            println!("cargo:rustc-env={key}={value}");
        }
        println!("cargo:rerun-if-changed={}", path.display());
    }

    // TIMEZONE is an IANA name, the device gets the equivalent POSIX rule as TIMEZONE_POSIX
    println!("cargo:rerun-if-env-changed=TIMEZONE");
    let timezone = env::var("TIMEZONE").map_err(|_| "TIMEZONE is not set (.env or environment)")?;
    let posix =
        posix_tz(&timezone).ok_or_else(|| format!("Unknown IANA timezone: {timezone:?}"))?;
    println!("cargo:rustc-env=TIMEZONE_POSIX={posix}");

    embuild::espidf::sysenv::output();
    Ok(())
}

/// Looks up the POSIX TZ rule (e.g. "CET-1CEST,M3.5.0,M10.5.0/3") for an IANA name (e.g. "Europe/Prague").
/// TZif v2+ files end with the rule as the last line, newlib on the device can evaluate it without a tz database.
fn posix_tz(name: &str) -> Option<String> {
    let raw = tzdb_data::find_raw(name.as_bytes())?;
    let footer = raw.strip_suffix(b"\n")?;
    let start = footer.iter().rposition(|&b| b == b'\n')? + 1;
    let rule = std::str::from_utf8(&footer[start..]).ok()?;
    // zones without a footer rule can't be expressed for newlib
    (!rule.is_empty()).then(|| rule.to_owned())
}
