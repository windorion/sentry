use crate::model::{SocketProtocol, SocketSnapshot};

pub fn collect() -> Result<Vec<SocketSnapshot>, String> {
    let mut sockets = listeners::get_all()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|listener| SocketSnapshot {
            protocol: if listener.protocol.to_string().eq_ignore_ascii_case("tcp") {
                SocketProtocol::Tcp
            } else {
                SocketProtocol::Udp
            },
            local_address: listener.socket.ip().to_string(),
            local_port: listener.socket.port(),
            remote_address: None,
            remote_port: None,
            state: listener.state.to_string().to_ascii_uppercase(),
            associated_pids: vec![listener.process.pid],
            process_names: vec![listener.process.name],
        })
        .collect::<Vec<_>>();
    sockets.sort_by(|left, right| {
        socket_priority(left)
            .cmp(&socket_priority(right))
            .then_with(|| left.local_port.cmp(&right.local_port))
            .then_with(|| left.local_address.cmp(&right.local_address))
    });
    Ok(sockets)
}

fn socket_priority(socket: &SocketSnapshot) -> u8 {
    match (socket.protocol, socket.state.as_str()) {
        (SocketProtocol::Tcp, "LISTEN") => 0,
        (SocketProtocol::Tcp, "ESTABLISHED") => 1,
        (SocketProtocol::Udp, _) => 2,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prioritizes_listening_tcp_sockets() {
        let socket = SocketSnapshot {
            protocol: SocketProtocol::Tcp,
            local_address: "127.0.0.1".to_owned(),
            local_port: 8080,
            remote_address: None,
            remote_port: None,
            state: "LISTEN".to_owned(),
            associated_pids: vec![42],
            process_names: vec!["api".to_owned()],
        };
        assert_eq!(socket_priority(&socket), 0);
    }
}
