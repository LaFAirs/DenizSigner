//! Retry only a rejected initial SRP request, never password proofs or 2FA.
use rootcause::Report;
use std::time::Duration;

pub fn apple_service_unavailable(report: &Report) -> bool {
    report.iter_reports().any(|cause| {
        cause
            .downcast_current_context::<reqwest_auth::Error>()
            .is_some_and(|error| {
                error.status() == Some(reqwest_auth::StatusCode::SERVICE_UNAVAILABLE)
                    && error.url().is_some_and(|url| {
                        url.scheme() == "https" && url.host_str() == Some("gsa.apple.com")
                    })
            })
    })
}

pub fn initial_login_retry_delay(
    report: &Report,
    retries: usize,
    two_factor_started: bool,
) -> Option<Duration> {
    if two_factor_started {
        return None;
    }
    let seconds = *[3, 8].get(retries)?;
    let initial_request = report.iter_reports().any(|cause| {
        cause
            .downcast_current_context::<&'static str>()
            .is_some_and(|context| *context == "Failed to send initial login request")
    });
    (initial_request && apple_service_unavailable(report)).then(|| Duration::from_secs(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rootcause::prelude::*;

    async fn response_report(status: u16, url: &str, context: &'static str) -> Report {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0; 4096];
            socket.read(&mut buffer).await.unwrap();
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 {status} Test\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        });
        let response = reqwest_auth::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .get(local)
            .send()
            .await
            .unwrap();
        let error = response
            .error_for_status()
            .unwrap_err()
            .with_url(url.parse().unwrap());
        server.await.unwrap();
        Err::<(), _>(error).context(context).unwrap_err().into()
    }

    #[tokio::test]
    async fn retries_only_initial_apple_503_and_stops_after_two_retries() {
        let report = response_report(
            503,
            "https://gsa.apple.com/grandslam/GsService2",
            "Failed to send initial login request",
        )
        .await;
        assert_eq!(
            initial_login_retry_delay(&report, 0, false),
            Some(Duration::from_secs(3))
        );
        assert_eq!(
            initial_login_retry_delay(&report, 1, false),
            Some(Duration::from_secs(8))
        );
        assert_eq!(initial_login_retry_delay(&report, 2, false), None);
        assert_eq!(initial_login_retry_delay(&report, 0, true), None);
        assert_eq!(initial_login_retry_delay(&report, usize::MAX, false), None);
        assert!(matches!(
            crate::error::AppError::from(report),
            crate::error::AppError::AppleServiceUnavailable(_)
        ));
    }

    #[tokio::test]
    async fn does_not_retry_auth_errors_other_hosts_or_later_login_steps() {
        for (status, url, context) in [
            (
                401,
                "https://gsa.apple.com/grandslam/GsService2",
                "Failed to send initial login request",
            ),
            (
                429,
                "https://gsa.apple.com/grandslam/GsService2",
                "Failed to send initial login request",
            ),
            (
                503,
                "https://ani.example.org/v3/get_headers",
                "Failed to send initial login request",
            ),
            (
                503,
                "https://gsa.apple.com/grandslam/GsService2",
                "Failed to send login completion request",
            ),
            (
                503,
                "https://gsa.apple.com/auth/verify",
                "Failed to validate 2FA code",
            ),
        ] {
            let report = response_report(status, url, context).await;
            assert_eq!(
                initial_login_retry_delay(&report, 0, false),
                None,
                "{status} {url} {context}"
            );
        }
    }
}
