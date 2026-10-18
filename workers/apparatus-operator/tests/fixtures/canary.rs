//! PH-7: a live target and a completed remote exec precede any bounded observation.
//! This is NOT a certification of Service DNS/dataplane, `NetworkPolicy` or mesh.

use std::net::IpAddr;

use async_trait::async_trait;

pub const CANARY_LABEL: &str = "app=apparatus-system-canary";
pub const CANARY_SERVICE: &str = "apparatus-system-canary";
// The image is Debian bookworm + GNU wget. A shell footer proves exec completed;
// request/transport timeouts cannot silently masquerade as the wget result.
pub const WGET_SCRIPT: &str = "command -v wget >/dev/null 2>&1 || exit 127; tool=$(LC_ALL=C wget --version 2>/dev/null) || exit 127; case \"$tool\" in 'GNU Wget '*) ;; *) exit 126 ;; esac; printf 'M6_EXEC_READY\\nM6_WGET_GNU\\n'; LC_ALL=C wget --no-proxy --tries=1 --timeout=5 -S -O - \"$1\"; rc=$?; printf '\\nM6_WGET_EXIT=%s\\n' \"$rc\"; exit \"$rc\"";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanaryTarget {
    pub ip: IpAddr,
    pub pod: String,
    pub uid: uuid::Uuid,
}

impl CanaryTarget {
    pub fn url(&self) -> String {
        match self.ip {
            IpAddr::V4(ip) => format!("http://{ip}:8080/"),
            IpAddr::V6(ip) => format!("http://[{ip}]:8080/"),
        }
    }
}

pub struct PositiveHttp {
    pub status: u16,
    pub body: String,
}

pub struct WgetOutput {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[async_trait]
pub trait CanaryPort {
    async fn wait_ready(&mut self) -> Result<(), &'static str>;
    async fn endpoints(&mut self) -> Result<Vec<u8>, &'static str>;
    async fn positive_http(&mut self, target: &CanaryTarget) -> Result<PositiveHttp, &'static str>;
    async fn wget(&mut self, target: &CanaryTarget) -> Result<WgetOutput, &'static str>;
}

#[derive(Debug)]
pub struct CanaryObservation {
    positive_status: u16,
}

impl CanaryObservation {
    pub fn marker(&self) -> String {
        format!(
            "M6 H4 positive_http={} negative=wget-connect-timeout-bounded",
            self.positive_status
        )
    }
}

fn ready_target(json: &[u8]) -> Result<CanaryTarget, &'static str> {
    let endpoints: serde_json::Value =
        serde_json::from_slice(json).map_err(|_| "canary endpoints invalid")?;
    let subsets = endpoints["subsets"]
        .as_array()
        .ok_or("canary ready endpoints absent")?;
    for subset in subsets {
        if !subset["ports"].as_array().is_some_and(|ports| {
            ports
                .iter()
                .any(|p| p["port"] == 8080 && p["protocol"] == "TCP")
        }) {
            continue;
        }
        for address in subset["addresses"].as_array().into_iter().flatten() {
            let ip = address["ip"]
                .as_str()
                .and_then(|s| s.parse::<IpAddr>().ok());
            let pod = address["targetRef"]["name"].as_str();
            if address["targetRef"]["kind"] == "Pod" {
                if let (Some(ip), Some(pod)) = (ip, pod) {
                    let uid = address["targetRef"]["uid"]
                        .as_str()
                        .and_then(|uid| uuid::Uuid::parse_str(uid).ok());
                    if let Some(uid) = uid {
                        if address["targetRef"]["namespace"] == "apparatus-system"
                            && pod.len() <= 253
                            && pod.starts_with("apparatus-system-canary-")
                            && pod
                                .bytes()
                                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                        {
                            return Ok(CanaryTarget {
                                ip,
                                pod: pod.to_owned(),
                                uid,
                            });
                        }
                    }
                }
            }
        }
    }
    Err("canary ready endpoints absent")
}

fn classify_wget(output: &WgetOutput, target: &CanaryTarget) -> Result<(), &'static str> {
    let text = format!("{}\n{}", output.stdout, output.stderr);
    let lower = text.to_ascii_lowercase();
    if lower.contains("http/") {
        return Err("canary reached from plugin");
    }
    for signal in [
        "connection refused",
        "can't connect",
        "network is unreachable",
        "no route to host",
        "bad address",
        "unable to resolve",
        "name or service not known",
        "not found",
        "executable file",
        "error executing",
        "unable to upgrade",
        "i/o timeout",
        "context deadline exceeded",
        "timed out waiting",
        "server (timeout)",
    ] {
        if lower.contains(signal) {
            return Err("canary negative non-probative");
        }
    }
    if output.code != Some(4) || output.stdout != "M6_EXEC_READY\nM6_WGET_GNU\n\nM6_WGET_EXIT=4\n" {
        return Err("canary negative exec incomplete or invalid");
    }
    // Numeric ready endpoint: the identified GNU wget connect timeout cannot be DNS.
    let address = match target.ip {
        IpAddr::V4(ip) => format!("{ip}:8080"),
        IpAddr::V6(ip) => format!("[{ip}]:8080"),
    };
    if !output
        .stderr
        .lines()
        .any(|line| line == format!("Connecting to {address}... failed: Connection timed out."))
    {
        return Err("canary negative non-probative");
    }
    Ok(())
}

/// The real m6 and negative oracles share this exact causal ordering and classifier.
pub async fn observe_canary(port: &mut impl CanaryPort) -> Result<CanaryObservation, &'static str> {
    port.wait_ready()
        .await
        .map_err(|_| "canary readiness failed")?;
    let endpoints = port
        .endpoints()
        .await
        .map_err(|_| "canary endpoints read failed")?;
    let target = ready_target(&endpoints)?;
    let positive = port
        .positive_http(&target)
        .await
        .map_err(|_| "canary positive HTTP failed")?;
    // Busybox httpd in 05-system-canary.yaml serves an empty root: its expected
    // application response is 404, not mere TCP acceptance or an arbitrary HTTP.
    if positive.status != 404 || !positive.body.contains("404 Not Found") {
        return Err("canary positive HTTP unexpected response");
    }
    revalidate(port, &target).await?;
    let negative = port
        .wget(&target)
        .await
        .map_err(|_| "canary exec transport failed")?;
    classify_wget(&negative, &target)?;
    revalidate(port, &target).await?;
    Ok(CanaryObservation {
        positive_status: positive.status,
    })
}

async fn revalidate(
    port: &mut impl CanaryPort,
    expected: &CanaryTarget,
) -> Result<(), &'static str> {
    let endpoints = port
        .endpoints()
        .await
        .map_err(|_| "canary endpoint revalidation failed")?;
    if ready_target(&endpoints)? != *expected {
        return Err("canary endpoint changed");
    }
    Ok(())
}
