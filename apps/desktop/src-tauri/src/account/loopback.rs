use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::watch,
};

use super::error::{AccountError, AccountResult};

/// One-shot HTTP listener on 127.0.0.1 with an OS-assigned port (RFC 8252 §7.3).
/// It accepts exactly one `/callback` request whose `state` matches; anything
/// else (favicon, wrong state) is answered and ignored.
pub struct Loopback {
    listener: TcpListener,
    pub redirect_uri: String,
}

pub struct CallbackParams {
    pub code: String,
    pub iss: Option<String>,
}

const PAGE_OK: &str = "<!doctype html><meta charset=utf-8><title>FormaCAD</title><body style=\"font:15px Segoe UI,sans-serif;background:#151719;color:#d5d7da;display:grid;place-items:center;height:100vh;margin:0\"><div><b>Готово.</b> Вернитесь в FormaCAD — окно можно закрыть.<br><span style=\"color:#797e86\">Done. Return to FormaCAD; you can close this tab.</span></div>";
const PAGE_FAIL: &str = "<!doctype html><meta charset=utf-8><title>FormaCAD</title><body style=\"font:15px Segoe UI,sans-serif;background:#151719;color:#d5d7da;display:grid;place-items:center;height:100vh;margin:0\"><div><b>Вход не выполнен.</b> Вернитесь в FormaCAD.<br><span style=\"color:#797e86\">Sign-in did not complete. Return to FormaCAD.</span></div>";

impl Loopback {
    pub async fn bind() -> AccountResult<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| AccountError::Storage(format!("loopback listener: {e}")))?;
        let port = listener
            .local_addr()
            .map_err(|e| AccountError::Storage(e.to_string()))?
            .port();
        Ok(Self {
            listener,
            redirect_uri: format!("http://127.0.0.1:{port}/callback"),
        })
    }

    pub async fn wait(
        self,
        expected_state: &str,
        timeout: Duration,
        mut cancel: watch::Receiver<bool>,
    ) -> AccountResult<CallbackParams> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let accept = tokio::select! {
                r = self.listener.accept() => r,
                _ = tokio::time::sleep_until(deadline) => return Err(AccountError::Timeout),
                _ = cancel.changed() => return Err(AccountError::Cancelled),
            };
            let Ok((mut stream, _)) = accept else {
                continue;
            };
            let mut buf = vec![0u8; 8192];
            let n = match tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buf)).await
            {
                Ok(Ok(n)) => n,
                _ => continue,
            };
            let request = String::from_utf8_lossy(&buf[..n]);
            let target = request
                .lines()
                .next()
                .and_then(|line| line.strip_prefix("GET "))
                .and_then(|rest| rest.split(' ').next())
                .unwrap_or("");
            let outcome = parse_callback(target, expected_state);
            let (status, body) = match &outcome {
                Some(Ok(_)) => ("200 OK", PAGE_OK),
                Some(Err(_)) => ("400 Bad Request", PAGE_FAIL),
                None => ("404 Not Found", ""),
            };
            let response = format!(
                "HTTP/1.1 {status}\r\ncontent-type: text/html; charset=utf-8\r\ncache-control: no-store\r\nreferrer-policy: no-referrer\r\nconnection: close\r\ncontent-length: {}\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes()).await;
            let _ = stream.shutdown().await;
            match outcome {
                Some(result) => return result,
                None => continue,
            }
        }
    }
}

/// `None` = not our callback (keep waiting); `Some(Err)` = our callback with an error.
fn parse_callback(target: &str, expected_state: &str) -> Option<AccountResult<CallbackParams>> {
    let url = url::Url::parse(&format!("http://127.0.0.1{target}")).ok()?;
    if url.path() != "/callback" {
        return None;
    }
    let get = |k: &str| {
        url.query_pairs()
            .find(|(key, _)| key == k)
            .map(|(_, v)| v.into_owned())
    };
    // A callback with a foreign state is ignored, not trusted (CSRF / mix-up protection).
    if get("state").as_deref() != Some(expected_state) {
        return None;
    }
    if let Some(error) = get("error") {
        return Some(Err(if error == "access_denied" {
            AccountError::Denied
        } else {
            AccountError::Server(format!("authorization error: {error}"))
        }));
    }
    match get("code") {
        Some(code) if !code.is_empty() && code.len() <= 128 => Some(Ok(CallbackParams {
            code,
            iss: get("iss"),
        })),
        _ => Some(Err(AccountError::Invalid("callback without code".into()))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_matching_state() {
        assert!(parse_callback("/favicon.ico", "s").is_none());
        assert!(parse_callback("/callback?code=x&state=other", "s").is_none());
        let ok = parse_callback(
            "/callback?code=abc&state=s&iss=https%3A%2F%2Fformacad.ru",
            "s",
        )
        .unwrap()
        .unwrap();
        assert_eq!(ok.code, "abc");
        assert_eq!(ok.iss.as_deref(), Some("https://formacad.ru"));
        assert!(matches!(
            parse_callback("/callback?error=access_denied&state=s", "s"),
            Some(Err(AccountError::Denied))
        ));
    }

    #[tokio::test]
    async fn receives_callback_over_tcp_and_times_out() {
        let lb = Loopback::bind().await.unwrap();
        let uri = lb.redirect_uri.clone();
        let (_tx, rx) = watch::channel(false);
        let waiter = tokio::spawn(async move { lb.wait("st", Duration::from_secs(5), rx).await });
        let port = url::Url::parse(&uri).unwrap().port().unwrap();
        let mut s = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        s.write_all(b"GET /callback?code=c1&state=st HTTP/1.1\r\nhost: x\r\n\r\n")
            .await
            .unwrap();
        let got = waiter.await.unwrap().unwrap();
        assert_eq!(got.code, "c1");

        let lb = Loopback::bind().await.unwrap();
        let (_tx, rx) = watch::channel(false);
        assert!(matches!(
            lb.wait("st", Duration::from_millis(50), rx).await,
            Err(AccountError::Timeout)
        ));
    }
}
