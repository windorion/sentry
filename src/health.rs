use std::time::{Duration, Instant};

use chrono::Utc;
use reqwest::Client;
use tokio::{net::TcpStream, task::JoinSet, time::timeout};

use crate::{
    config::ServiceConfig,
    model::{HealthState, ServiceStatus},
};

pub async fn check_all(services: &[ServiceConfig]) -> Vec<ServiceStatus> {
    let client = Client::builder()
        .user_agent(concat!("wsentry/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("reqwest client builds");

    let mut checks = JoinSet::new();
    for service in services {
        let client = client.clone();
        let service = service.clone();
        checks.spawn(async move { check_one(&client, &service).await });
    }

    let mut results = Vec::with_capacity(services.len());
    while let Some(result) = checks.join_next().await {
        if let Ok(status) = result {
            results.push(status);
        }
    }
    results.sort_by(|left, right| left.name.cmp(&right.name));
    results
}

async fn check_one(client: &Client, service: &ServiceConfig) -> ServiceStatus {
    let timeout_duration =
        humantime::parse_duration(&service.timeout).unwrap_or_else(|_| Duration::from_secs(3));

    if let Some(url) = &service.health {
        return check_http(client, service, url, timeout_duration).await;
    }
    if let Some(target) = &service.tcp {
        return check_tcp(service, target, timeout_duration).await;
    }

    ServiceStatus {
        name: service.name.clone(),
        target: "not configured".to_owned(),
        state: HealthState::Unknown,
        latency_ms: None,
        status_code: None,
        checked_at: Utc::now(),
        message: Some("service requires either `health` or `tcp`".to_owned()),
    }
}

async fn check_http(
    client: &Client,
    service: &ServiceConfig,
    url: &str,
    timeout_duration: Duration,
) -> ServiceStatus {
    let started = Instant::now();
    let result = client.get(url).timeout(timeout_duration).send().await;
    match result {
        Ok(response) => {
            let code = response.status();
            ServiceStatus {
                name: service.name.clone(),
                target: url.to_owned(),
                state: if code.is_success() {
                    HealthState::Healthy
                } else {
                    HealthState::Unhealthy
                },
                latency_ms: Some(started.elapsed().as_millis()),
                status_code: Some(code.as_u16()),
                checked_at: Utc::now(),
                message: (!code.is_success()).then(|| format!("HTTP {code}")),
            }
        }
        Err(error) => ServiceStatus {
            name: service.name.clone(),
            target: url.to_owned(),
            state: HealthState::Unhealthy,
            latency_ms: Some(started.elapsed().as_millis()),
            status_code: None,
            checked_at: Utc::now(),
            message: Some(error.to_string()),
        },
    }
}

async fn check_tcp(
    service: &ServiceConfig,
    target: &str,
    timeout_duration: Duration,
) -> ServiceStatus {
    let started = Instant::now();
    match timeout(timeout_duration, TcpStream::connect(target)).await {
        Ok(Ok(_stream)) => ServiceStatus {
            name: service.name.clone(),
            target: target.to_owned(),
            state: HealthState::Healthy,
            latency_ms: Some(started.elapsed().as_millis()),
            status_code: None,
            checked_at: Utc::now(),
            message: None,
        },
        Ok(Err(error)) => ServiceStatus {
            name: service.name.clone(),
            target: target.to_owned(),
            state: HealthState::Unhealthy,
            latency_ms: Some(started.elapsed().as_millis()),
            status_code: None,
            checked_at: Utc::now(),
            message: Some(error.to_string()),
        },
        Err(_) => ServiceStatus {
            name: service.name.clone(),
            target: target.to_owned(),
            state: HealthState::Unhealthy,
            latency_ms: Some(started.elapsed().as_millis()),
            status_code: None,
            checked_at: Utc::now(),
            message: Some(format!(
                "timed out after {}",
                humantime::format_duration(timeout_duration)
            )),
        },
    }
}

#[cfg(test)]
mod tests {
    use tokio::net::TcpListener;

    use super::*;

    #[tokio::test]
    async fn returns_checks_in_stable_name_order() {
        let service = |name: &str| ServiceConfig {
            name: name.to_owned(),
            health: None,
            tcp: None,
            interval: "30s".to_owned(),
            timeout: "1s".to_owned(),
        };

        let statuses = check_all(&[service("worker"), service("api")]).await;

        assert_eq!(statuses.len(), 2);
        assert_eq!(statuses[0].name, "api");
        assert_eq!(statuses[1].name, "worker");
    }

    #[tokio::test]
    async fn reports_missing_targets_as_unknown() {
        let service = ServiceConfig {
            name: "incomplete".to_owned(),
            health: None,
            tcp: None,
            interval: "30s".to_owned(),
            timeout: "1s".to_owned(),
        };

        let statuses = check_all(&[service]).await;

        assert_eq!(statuses[0].state, HealthState::Unknown);
        assert!(statuses[0].message.is_some());
    }

    #[tokio::test]
    async fn reports_reachable_tcp_target_as_healthy() {
        let listener = match TcpListener::bind("127.0.0.1:0").await {
            Ok(listener) => listener,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => return,
            Err(error) => panic!("bind local listener: {error}"),
        };
        let address = listener.local_addr().expect("listener address");
        let accepted = tokio::spawn(async move {
            listener.accept().await.expect("accept health check");
        });
        let service = ServiceConfig {
            name: "local".to_owned(),
            health: None,
            tcp: Some(address.to_string()),
            interval: "5s".to_owned(),
            timeout: "1s".to_owned(),
        };

        let statuses = check_all(&[service]).await;
        accepted.await.expect("listener task completes");

        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].state, HealthState::Healthy);
        assert!(statuses[0].message.is_none());
    }
}
