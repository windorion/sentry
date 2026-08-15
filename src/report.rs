use std::{fs, io, path::PathBuf};

use chrono::Utc;

use crate::{format, model::SystemSnapshot};

pub fn text(snapshot: &SystemSnapshot) -> String {
    let memory_percent = format::percent(snapshot.memory_used_bytes, snapshot.memory_total_bytes);
    let mut output = format!(
        "Windorion Sentry diagnostic report\n\
         Generated: {}\n\
         Host: {}\n\
         OS: {}\n\
         Kernel: {}\n\
         Uptime: {}\n\
         CPU: {:.1}%\n\
         Load: {:.2} {:.2} {:.2}\n\
         Memory: {} / {} ({memory_percent:.1}%)\n\
         Processes: {}\n\
         Disks: {}\n\
         Network interfaces: {}\n\
         Ports and sockets: {}\n\
         Service checks: {}\n",
        snapshot.timestamp.to_rfc3339(),
        snapshot.host_name,
        snapshot.os_name,
        snapshot.kernel_version,
        snapshot
            .uptime_seconds
            .map(format::duration)
            .unwrap_or_else(|| "unavailable".to_owned()),
        snapshot.cpu_usage_percent,
        snapshot.load_average.one,
        snapshot.load_average.five,
        snapshot.load_average.fifteen,
        format::bytes(snapshot.memory_used_bytes),
        format::bytes(snapshot.memory_total_bytes),
        snapshot.processes.len(),
        snapshot.disks.len(),
        snapshot.networks.len(),
        snapshot.sockets.len(),
        snapshot.services.len(),
    );

    if !snapshot.services.is_empty() {
        output.push_str("\nServices:\n");
        for service in &snapshot.services {
            output.push_str(&format!(
                "- {}: {:?} ({})\n",
                service.name, service.state, service.target
            ));
        }
    }
    output
}

pub fn json(snapshot: &SystemSnapshot) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(snapshot)
}

pub fn redact(mut snapshot: SystemSnapshot) -> SystemSnapshot {
    snapshot.host_name = "[redacted]".to_owned();
    for process in &mut snapshot.processes {
        if !process.command.is_empty() {
            process.command = "[redacted]".to_owned();
        }
    }
    for socket in &mut snapshot.sockets {
        socket.local_address = "[redacted]".to_owned();
        socket.remote_address = socket
            .remote_address
            .as_ref()
            .map(|_| "[redacted]".to_owned());
    }
    for service in &mut snapshot.services {
        service.target = "[redacted]".to_owned();
    }
    snapshot
}

pub fn write_json(snapshot: &SystemSnapshot) -> io::Result<PathBuf> {
    let filename = format!("wsentry-report-{}.json", Utc::now().format("%Y%m%d-%H%M%S"));
    let path = std::env::current_dir()?.join(filename);
    let bytes = serde_json::to_vec_pretty(snapshot).map_err(io::Error::other)?;
    fs::write(&path, bytes)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use crate::collector::{DemoCollector, SnapshotSource};

    use super::*;

    #[test]
    fn redacts_sensitive_snapshot_fields() {
        let mut collector = DemoCollector::new();
        let snapshot = redact(collector.sample());

        assert_eq!(snapshot.host_name, "[redacted]");
        assert!(
            snapshot
                .processes
                .iter()
                .all(|process| process.command == "[redacted]")
        );
        assert!(
            snapshot
                .sockets
                .iter()
                .all(|socket| socket.local_address == "[redacted]")
        );
        assert!(
            snapshot
                .services
                .iter()
                .all(|service| service.target == "[redacted]")
        );
    }
}
