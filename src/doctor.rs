use std::{io::IsTerminal, path::PathBuf};

use serde::Serialize;

use crate::config;

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub version: &'static str,
    pub os: &'static str,
    pub architecture: &'static str,
    pub system_collection_supported: bool,
    pub stdout_is_terminal: bool,
    pub config_path: Option<PathBuf>,
    pub config_valid: bool,
    pub checks: Vec<DoctorCheck>,
}

#[derive(Debug, Serialize)]
pub struct DoctorCheck {
    pub name: &'static str,
    pub ok: bool,
    pub detail: String,
}

pub fn inspect() -> DoctorReport {
    let config_path = config::discover_path(None);
    let config_result = config_path.as_deref().map(config::load);
    let config_valid = config_result.as_ref().is_none_or(Result::is_ok);
    let mut checks = vec![
        DoctorCheck {
            name: "system collector",
            ok: sysinfo::IS_SUPPORTED_SYSTEM,
            detail: if sysinfo::IS_SUPPORTED_SYSTEM {
                "supported on this platform".to_owned()
            } else {
                "sysinfo does not support this platform".to_owned()
            },
        },
        DoctorCheck {
            name: "terminal",
            ok: true,
            detail: if std::io::stdout().is_terminal() {
                "stdout is an interactive terminal".to_owned()
            } else {
                "stdout is not interactive; use report/check for scripts".to_owned()
            },
        },
        DoctorCheck {
            name: "port collector",
            ok: listeners::IS_OS_SUPPORTED,
            detail: if listeners::IS_OS_SUPPORTED {
                "native socket discovery is supported".to_owned()
            } else {
                "native socket discovery is unavailable on this platform".to_owned()
            },
        },
    ];
    if let (Some(path), Some(result)) = (&config_path, config_result) {
        checks.push(DoctorCheck {
            name: "configuration",
            ok: result.is_ok(),
            detail: match result {
                Ok(_) => format!("loaded {}", path.display()),
                Err(error) => error.to_string(),
            },
        });
    } else {
        checks.push(DoctorCheck {
            name: "configuration",
            ok: true,
            detail: "no wsentry.toml found; defaults will be used".to_owned(),
        });
    }

    DoctorReport {
        version: env!("CARGO_PKG_VERSION"),
        os: std::env::consts::OS,
        architecture: std::env::consts::ARCH,
        system_collection_supported: sysinfo::IS_SUPPORTED_SYSTEM,
        stdout_is_terminal: std::io::stdout().is_terminal(),
        config_path,
        config_valid,
        checks,
    }
}

pub fn text(report: &DoctorReport) -> String {
    let mut output = format!(
        "wsentry doctor {} ({} {})\n",
        report.version, report.os, report.architecture
    );
    for check in &report.checks {
        output.push_str(&format!(
            "{} {:<18} {}\n",
            if check.ok { "OK" } else { "!!" },
            check.name,
            check.detail
        ));
    }
    output
}
