//! The HTTP client for the player's remote API, over a local port made by `adb forward`.

use std::fmt;
use std::time::Duration;

use reqwest::header::{HeaderValue, CONTENT_TYPE};
use reqwest::{Client, Method};
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::model::{
    Accepted, Applied, ApplyRequest, Capabilities, Refusal, Reply, Schema, Snapshot, Status,
};
use crate::error::{Error, Result};

/// The player answers a change within 10 s at most; leave room for that.
const LIMIT: Duration = Duration::from_secs(15);
/// A schema is some tens of kilobytes; nothing legitimate is near this.
const BODY_LIMIT: usize = 2 << 20;

/// The Remote Control PIN. It lives in memory only and never prints.
#[derive(Clone)]
pub struct Pin(String);

impl Pin {
    pub fn new(pin: impl Into<String>) -> Self {
        Self(pin.into().trim().to_string())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for Pin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Pin(..)")
    }
}

/// One player reached through a forwarded loopback port.
#[derive(Debug, Clone)]
pub struct Player {
    base: String,
    pin: Pin,
    http: Client,
}

impl Player {
    pub fn new(port: u16, pin: Pin) -> Result<Self> {
        // Loopback must never go through a system proxy.
        let http = Client::builder()
            .no_proxy()
            // A redirect would re-send the PIN header to another host.
            .redirect(reqwest::redirect::Policy::none())
            .timeout(LIMIT)
            .connect_timeout(Duration::from_secs(5))
            .user_agent(concat!("compas-tools/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            base: format!("http://127.0.0.1:{port}/api/v1"),
            pin,
            http,
        })
    }

    async fn send(
        &self,
        method: Method,
        path: &str,
        body: Option<Vec<u8>>,
        with_pin: bool,
    ) -> Result<(u16, Value)> {
        let mut request = self.http.request(method, format!("{}{path}", self.base));
        if with_pin {
            let mut pin = HeaderValue::from_str(&self.pin.0).map_err(|_| {
                Error::Rejected("The PIN has characters that cannot be sent.".into())
            })?;
            pin.set_sensitive(true);
            request = request.header("X-Compas-PIN", pin);
        }
        if let Some(body) = body {
            request = request.header(CONTENT_TYPE, "application/json").body(body);
        }
        let response = request.send().await?;
        let status = response.status().as_u16();
        if (300..400).contains(&status) {
            // The player never redirects; something else answered on this port.
            return Err(Error::HttpStatus {
                status,
                url: path.to_string(),
            });
        }
        let bytes = response.bytes().await?;
        if bytes.len() > BODY_LIMIT {
            return Err(Error::Rejected(
                "The player sent an answer that is too large.".into(),
            ));
        }
        // An empty or non-JSON body (an old status page, a proxy) is kept as null.
        Ok((
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        ))
    }

    /// Turns an answer into data on 2xx and a [`Refusal`] otherwise.
    fn reply<T: DeserializeOwned>(status: u16, body: Value) -> Result<Reply<T>> {
        if (200..300).contains(&status) {
            let data = serde_json::from_value(body)
                .map_err(|_| Error::Rejected("The player's answer was not understood.".into()))?;
            return Ok(Reply::Ok { data });
        }
        let code = match body.get("code").and_then(Value::as_str) {
            Some(code) => code.to_string(),
            None if status == 401 => "pin_required".to_string(),
            None => format!("http_{status}"),
        };
        let part = |name: &str| body.get(name).cloned().unwrap_or(Value::Null);
        Ok(Reply::Refused {
            refusal: Refusal {
                status,
                code,
                errors: part("errors"),
                current: part("current"),
            },
        })
    }

    /// `GET /capabilities`, which needs no PIN.
    pub async fn capabilities(&self) -> Result<Capabilities> {
        let (status, body) = self.send(Method::GET, "/capabilities", None, false).await?;
        match Self::reply::<Capabilities>(status, body)? {
            Reply::Ok { data } => Ok(data),
            Reply::Refused { refusal } => Err(Error::HttpStatus {
                status: refusal.status,
                url: "/api/v1/capabilities".into(),
            }),
        }
    }

    pub async fn status(&self) -> Result<Reply<Status>> {
        let (status, body) = self.send(Method::GET, "/status", None, true).await?;
        Self::reply(status, body)
    }

    pub async fn schema(&self) -> Result<Reply<Schema>> {
        let (status, body) = self
            .send(Method::GET, "/settings/schema", None, true)
            .await?;
        Self::reply(status, body)
    }

    /// Every value, or with `since` (revision, epoch) only what changed after it.
    pub async fn settings(&self, since: Option<(u64, u64)>) -> Result<Reply<Snapshot>> {
        let path = match since {
            Some((revision, epoch)) => format!("/settings?since={revision}&epoch={epoch}"),
            None => "/settings".to_string(),
        };
        let (status, body) = self.send(Method::GET, &path, None, true).await?;
        Self::reply(status, body)
    }

    pub async fn apply(&self, request: &ApplyRequest) -> Result<Reply<Applied>> {
        let body = serde_json::to_vec(request)
            .map_err(|e| Error::Rejected(format!("The change could not be encoded: {e}")))?;
        let (status, body) = self
            .send(Method::POST, "/settings", Some(body), true)
            .await?;
        Self::reply(status, body)
    }

    /// Starts the library scan ("Update Music Database"); it needs no body.
    pub async fn rescan(&self) -> Result<Reply<Accepted>> {
        let (status, body) = self
            .send(
                Method::POST,
                "/settings/actions/library.rescan",
                Some(b"{}".to_vec()),
                true,
            )
            .await?;
        Self::reply(status, body)
    }
}
