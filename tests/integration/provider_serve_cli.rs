use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const PROVIDER_RUNTIME_ENV: [&str; 6] = [
    "ASP_PROVIDER_ARTIFACT_DIGEST",
    "ASP_PROVIDER_REGISTRATION_DIGEST",
    "ASP_PROVIDER_RUNTIME_CONTRACT_DIGEST",
    "ASP_PROVIDER_ID",
    "ASP_PROVIDER_LANGUAGE_ID",
    "ASP_CLIENT_SERVER_HOST",
];

fn command(args: &[&str]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_asp-rust"));
    command.args(args);
    for name in PROVIDER_RUNTIME_ENV {
        command.env_remove(name);
    }
    command.output().expect("run the real asp-rust CLI")
}

#[test]
fn serve_is_the_direct_public_provider_runtime_command() {
    let output = command(&["serve"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 asp-rust stderr");
    assert!(
        stderr.contains("provider runtime requires ASP_PROVIDER_ARTIFACT_DIGEST"),
        "direct serve must enter the provider runtime: {stderr}"
    );
}

#[test]
fn serve_publishes_v1_bootstrap_health_and_shutdown_over_http_json() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_asp-rust"))
        .arg("serve")
        .env(
            "ASP_PROVIDER_ARTIFACT_DIGEST",
            format!("blake3-256:{}", "e".repeat(64)),
        )
        .env(
            "ASP_PROVIDER_REGISTRATION_DIGEST",
            format!("sha256:{}", "1".repeat(64)),
        )
        .env(
            "ASP_PROVIDER_RUNTIME_CONTRACT_DIGEST",
            format!("sha256:{}", "2".repeat(64)),
        )
        .env("ASP_PROVIDER_ID", "asp-rust")
        .env("ASP_PROVIDER_LANGUAGE_ID", "rust")
        .env("ASP_CLIENT_SERVER_HOST", "127.0.0.1:0")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn asp-rust serve");
    let stdout = child.stdout.take().expect("asp-rust stdout");
    let (line_tx, line_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = BufReader::new(stdout).read_line(&mut line).map(|_| line);
        let _ = line_tx.send(result);
    });

    let result = (|| -> Result<(), String> {
        let bootstrap_line = line_rx
            .recv_timeout(Duration::from_secs(2))
            .map_err(|error| format!("wait for bootstrap: {error}"))?
            .map_err(|error| format!("read bootstrap: {error}"))?;
        let bootstrap: serde_json::Value = serde_json::from_str(&bootstrap_line)
            .map_err(|error| format!("decode bootstrap: {error}"))?;
        assert_eq!(
            bootstrap,
            serde_json::json!({
                "schemaId": "agent.semantic-protocols.asp-client-server-bootstrap",
                "schemaVersion": "1",
                "transport": "http-json",
                "state": "ready",
                "endpoint": bootstrap["endpoint"],
            })
        );
        let endpoint = bootstrap["endpoint"]
            .as_str()
            .ok_or_else(|| "bootstrap endpoint is absent".to_owned())?;
        let authority = endpoint
            .strip_prefix("http://")
            .and_then(|value| value.strip_suffix('/'))
            .ok_or_else(|| format!("bootstrap endpoint is not canonical: {endpoint}"))?;

        let health = http_request(
            authority,
            "GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        )?;
        let health_body = http_body(&health)?;
        let health: serde_json::Value =
            serde_json::from_str(health_body).map_err(|error| format!("decode health: {error}"))?;
        assert_eq!(health["providerId"], "asp-rust");
        assert_eq!(health["languageId"], "rust");
        assert_eq!(health["transport"], "http-json");
        assert!(health.get("state").is_none());

        let shutdown = http_request(
            authority,
            "POST /shutdown HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
        )?;
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(http_body(&shutdown)?)
                .map_err(|error| format!("decode shutdown: {error}"))?,
            serde_json::json!({"state": "draining"})
        );

        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if let Some(status) = child
                .try_wait()
                .map_err(|error| format!("poll asp-rust serve: {error}"))?
            {
                if status.success() {
                    return Ok(());
                }
                return Err(format!("asp-rust serve exited with {status}"));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Err("asp-rust serve did not stop after /shutdown".to_owned())
    })();
    if result.is_err() {
        let _ = child.kill();
    }
    let status = child.wait().ok();
    let mut stderr = String::new();
    if let Some(mut child_stderr) = child.stderr.take() {
        let _ = child_stderr.read_to_string(&mut stderr);
    }
    if let Err(error) = result {
        panic!("asp-rust HTTP runtime contract: {error}; status={status:?}; stderr={stderr}");
    }
}

fn http_request(authority: &str, request: &str) -> Result<String, String> {
    let mut stream = TcpStream::connect(authority)
        .map_err(|error| format!("connect asp-rust serve: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("set HTTP read timeout: {error}"))?;
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("write HTTP request: {error}"))?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("read HTTP response: {error}"))?;
    if !response.starts_with("HTTP/1.1 200") {
        return Err(format!("unexpected HTTP response: {response}"));
    }
    Ok(response)
}

fn http_body(response: &str) -> Result<&str, String> {
    response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .ok_or_else(|| "HTTP response body is absent".to_owned())
}

#[test]
fn provider_binary_rejects_every_non_server_command() {
    let output = command(&["not-a-command"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 asp-rust stderr");
    assert!(stderr.contains("only admitted process argument is `serve`"));
    assert!(!stderr.contains("provider runtime requires"));
}
