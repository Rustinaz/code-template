//! Linux platform integration module.

use std::fmt;
use std::sync::OnceLock;

/// Linux platform information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformInfo {
    /// Kernel release, as `uname -r` prints it, e.g. `"6.8.0-45-generic"`.
    pub kernel_version: String,
    /// Distribution name, e.g. `"Ubuntu 24.04.2 LTS"`.
    pub distribution: String,
}

impl fmt::Display for PlatformInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Linux ({} on {})",
            self.kernel_version, self.distribution
        )
    }
}

static CACHE: OnceLock<PlatformInfo> = OnceLock::new();

/// Reported when a file is missing or unreadable, so callers never have to
/// distinguish "empty" from "could not read".
const UNKNOWN: &str = "unknown";

/// Get basic platform information, caching the result after the first call.
#[must_use]
pub fn platform_info() -> PlatformInfo {
    CACHE.get_or_init(read_platform_info).clone()
}

fn read_platform_info() -> PlatformInfo {
    PlatformInfo {
        kernel_version: kernel_release().unwrap_or_else(|| UNKNOWN.to_string()),
        distribution: distribution_name().unwrap_or_else(|| UNKNOWN.to_string()),
    }
}

/// Reads the kernel release from `/proc/sys/kernel/osrelease`.
fn kernel_release() -> Option<String> {
    read_trimmed("/proc/sys/kernel/osrelease")
}

/// Reads `PRETTY_NAME` from `/etc/os-release`, falling back to the copy most
/// distributions install under `/usr/lib`.
fn distribution_name() -> Option<String> {
    std::fs::read_to_string("/etc/os-release")
        .or_else(|_| std::fs::read_to_string("/usr/lib/os-release"))
        .ok()
        .and_then(|contents| parse_os_release(&contents, "PRETTY_NAME"))
}

/// Extracts one `KEY=value` entry from an os-release file, stripping the
/// optional surrounding quotes that the spec allows.
fn parse_os_release(contents: &str, key: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let (k, v) = line.split_once('=')?;
        (k.trim() == key).then(|| unquote(v.trim()).to_string())
    })
}

/// Removes one layer of matching single or double quotes.
fn unquote(value: &str) -> &str {
    let bytes = value.as_bytes();
    let quoted = bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''));
    if quoted {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

fn read_trimmed(path: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Initialize platform-specific functionality.
pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Initializing Linux platform integration");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn kernel_version_is_a_kernel_release_not_a_crate_version() {
        let info = platform_info();
        assert!(!info.kernel_version.is_empty());
        assert_ne!(
            info.kernel_version,
            env!("CARGO_PKG_VERSION"),
            "the kernel release must not be the version of this crate"
        );
    }

    #[test]
    fn parses_pretty_name_with_quotes() {
        let sample = "NAME=\"Ubuntu\"\nPRETTY_NAME=\"Ubuntu 24.04.2 LTS\"\nID=ubuntu\n";
        assert_eq!(
            parse_os_release(sample, "PRETTY_NAME").as_deref(),
            Some("Ubuntu 24.04.2 LTS")
        );
    }

    #[test]
    fn parses_pretty_name_without_quotes() {
        assert_eq!(
            parse_os_release("PRETTY_NAME=Arch Linux\n", "PRETTY_NAME").as_deref(),
            Some("Arch Linux")
        );
    }

    #[test]
    fn missing_key_is_none() {
        assert_eq!(parse_os_release("ID=ubuntu\n", "PRETTY_NAME"), None);
    }

    #[test]
    fn partial_key_name_does_not_match() {
        // `PRETTY_NAME_X` must not be mistaken for `PRETTY_NAME`.
        assert_eq!(parse_os_release("PRETTY_NAME_X=no\n", "PRETTY_NAME"), None);
    }

    #[test]
    fn display_includes_kernel_and_distribution() {
        let info = PlatformInfo {
            kernel_version: "6.8.0".to_string(),
            distribution: "Ubuntu 24.04".to_string(),
        };
        assert_eq!(info.to_string(), "Linux (6.8.0 on Ubuntu 24.04)");
    }
}
