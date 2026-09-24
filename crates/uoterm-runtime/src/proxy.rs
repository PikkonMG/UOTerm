//! A way out to the shard through a proxy: SOCKS5 (RFC 1928, with the user
//! and password of RFC 1929) or an HTTP proxy's CONNECT. A proxy is written
//! as `socks5://[user:password@]host:port` or `http://[user:password@]host:port`.
//! Its password is never written to a log: the proxy shows itself with the
//! password left out.

use std::fmt;
use std::str::FromStr;

use base64::Engine;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const SCHEME_SOCKS5: &str = "socks5://";
const SCHEME_HTTP: &str = "http://";

const SOCKS_VERSION: u8 = 0x05;
const SOCKS_AUTH_NONE: u8 = 0x00;
const SOCKS_AUTH_PASSWORD: u8 = 0x02;
const SOCKS_AUTH_REFUSED: u8 = 0xFF;
const SOCKS_PASSWORD_VERSION: u8 = 0x01;
const SOCKS_PASSWORD_OK: u8 = 0x00;
const SOCKS_CONNECT: u8 = 0x01;
const SOCKS_RESERVED: u8 = 0x00;
const SOCKS_ADDRESS_IPV4: u8 = 0x01;
const SOCKS_ADDRESS_NAME: u8 = 0x03;
const SOCKS_ADDRESS_IPV6: u8 = 0x04;
const SOCKS_REPLY_OK: u8 = 0x00;
const IPV4_LEN: usize = 4;
const IPV6_LEN: usize = 16;
const PORT_LEN: usize = 2;
/// The longest name and password a SOCKS5 field holds.
const SOCKS_FIELD_MAX: usize = 255;
/// The longest answer head an HTTP proxy may send before its blank line.
const HTTP_HEAD_MAX: usize = 8192;
const HTTP_HEAD_END: &[u8] = b"\r\n\r\n";
const HTTP_OK: &str = "200";

/// The kind of proxy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProxyKind {
    Socks5,
    HttpConnect,
}

/// A proxy the session reaches the shard through.
#[derive(Clone, PartialEq, Eq)]
pub struct Proxy {
    pub kind: ProxyKind,
    pub host: String,
    pub port: u16,
    user: Option<(String, String)>,
}

impl fmt::Debug for Proxy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

/// The proxy as a person reads it, with the password left out.
impl fmt::Display for Proxy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scheme = match self.kind {
            ProxyKind::Socks5 => SCHEME_SOCKS5,
            ProxyKind::HttpConnect => SCHEME_HTTP,
        };
        match &self.user {
            Some((user, _)) => write!(f, "{scheme}{user}:***@{}:{}", self.host, self.port),
            None => write!(f, "{scheme}{}:{}", self.host, self.port),
        }
    }
}

impl FromStr for Proxy {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        let (kind, rest) = if let Some(rest) = text.strip_prefix(SCHEME_SOCKS5) {
            (ProxyKind::Socks5, rest)
        } else if let Some(rest) = text.strip_prefix(SCHEME_HTTP) {
            (ProxyKind::HttpConnect, rest)
        } else {
            return Err("a proxy is socks5://host:port or http://host:port".into());
        };
        let rest = rest.trim_end_matches('/');
        let (user, place) = match rest.rsplit_once('@') {
            Some((who, place)) => {
                let (name, password) = who
                    .split_once(':')
                    .ok_or("a proxy user is written user:password")?;
                if name.len() > SOCKS_FIELD_MAX || password.len() > SOCKS_FIELD_MAX {
                    return Err("a proxy user or password is longer than 255 bytes".into());
                }
                (Some((name.to_string(), password.to_string())), place)
            }
            None => (None, rest),
        };
        let (host, port) = place
            .rsplit_once(':')
            .ok_or("a proxy names its port: host:port")?;
        let port = port.parse().map_err(|_| "the proxy port is no number")?;
        let host = host.trim_matches(|c| c == '[' || c == ']').to_string();
        if host.is_empty() {
            return Err("a proxy names its host".into());
        }
        Ok(Self {
            kind,
            host,
            port,
            user,
        })
    }
}

impl serde::Serialize for Proxy {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.url())
    }
}

impl<'de> serde::Deserialize<'de> for Proxy {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

impl Proxy {
    /// The proxy as it is written, password and all, for the config file.
    fn url(&self) -> String {
        let scheme = match self.kind {
            ProxyKind::Socks5 => SCHEME_SOCKS5,
            ProxyKind::HttpConnect => SCHEME_HTTP,
        };
        match &self.user {
            Some((user, password)) => {
                format!("{scheme}{user}:{password}@{}:{}", self.host, self.port)
            }
            None => format!("{scheme}{}:{}", self.host, self.port),
        }
    }
}

fn refused(why: impl Into<String>) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::ConnectionRefused, why.into())
}

/// A TCP link to `host`:`port`, through the proxy when there is one.
pub async fn connect(proxy: Option<&Proxy>, host: &str, port: u16) -> std::io::Result<TcpStream> {
    let Some(proxy) = proxy else {
        return TcpStream::connect((host, port)).await;
    };
    let mut stream = TcpStream::connect((proxy.host.as_str(), proxy.port)).await?;
    match proxy.kind {
        ProxyKind::Socks5 => socks5_connect(&mut stream, proxy, host, port).await?,
        ProxyKind::HttpConnect => http_connect(&mut stream, proxy, host, port).await?,
    }
    tracing::info!(%proxy, host, port, "connected through the proxy");
    Ok(stream)
}

async fn socks5_connect(
    stream: &mut TcpStream,
    proxy: &Proxy,
    host: &str,
    port: u16,
) -> std::io::Result<()> {
    let method = if proxy.user.is_some() {
        SOCKS_AUTH_PASSWORD
    } else {
        SOCKS_AUTH_NONE
    };
    stream.write_all(&[SOCKS_VERSION, 1, method]).await?;
    let mut chosen = [0u8; 2];
    stream.read_exact(&mut chosen).await?;
    if chosen[0] != SOCKS_VERSION || chosen[1] == SOCKS_AUTH_REFUSED || chosen[1] != method {
        return Err(refused("the SOCKS5 proxy refused the way to sign in"));
    }
    if let Some((user, password)) = &proxy.user {
        let mut hello = vec![SOCKS_PASSWORD_VERSION, user.len() as u8];
        hello.extend_from_slice(user.as_bytes());
        hello.push(password.len() as u8);
        hello.extend_from_slice(password.as_bytes());
        stream.write_all(&hello).await?;
        let mut answer = [0u8; 2];
        stream.read_exact(&mut answer).await?;
        if answer[1] != SOCKS_PASSWORD_OK {
            return Err(refused("the SOCKS5 proxy refused the user and password"));
        }
    }
    let mut ask = vec![SOCKS_VERSION, SOCKS_CONNECT, SOCKS_RESERVED];
    match host.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(ip)) => {
            ask.push(SOCKS_ADDRESS_IPV4);
            ask.extend_from_slice(&ip.octets());
        }
        Ok(std::net::IpAddr::V6(ip)) => {
            ask.push(SOCKS_ADDRESS_IPV6);
            ask.extend_from_slice(&ip.octets());
        }
        Err(_) => {
            if host.len() > SOCKS_FIELD_MAX {
                return Err(refused("the shard's host name is longer than 255 bytes"));
            }
            ask.push(SOCKS_ADDRESS_NAME);
            ask.push(host.len() as u8);
            ask.extend_from_slice(host.as_bytes());
        }
    }
    ask.extend_from_slice(&port.to_be_bytes());
    stream.write_all(&ask).await?;
    let mut head = [0u8; 4];
    stream.read_exact(&mut head).await?;
    if head[1] != SOCKS_REPLY_OK {
        return Err(refused(format!(
            "the SOCKS5 proxy could not reach the shard (reply {})",
            head[1]
        )));
    }
    let bound = match head[3] {
        SOCKS_ADDRESS_IPV4 => IPV4_LEN,
        SOCKS_ADDRESS_IPV6 => IPV6_LEN,
        SOCKS_ADDRESS_NAME => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).await?;
            usize::from(len[0])
        }
        _ => return Err(refused("the SOCKS5 proxy answered an unknown address kind")),
    };
    let mut rest = vec![0u8; bound + PORT_LEN];
    stream.read_exact(&mut rest).await?;
    Ok(())
}

async fn http_connect(
    stream: &mut TcpStream,
    proxy: &Proxy,
    host: &str,
    port: u16,
) -> std::io::Result<()> {
    let target = format!("{host}:{port}");
    let mut ask = format!("CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n");
    if let Some((user, password)) = &proxy.user {
        let token = base64::engine::general_purpose::STANDARD.encode(format!("{user}:{password}"));
        ask.push_str(&format!("Proxy-Authorization: Basic {token}\r\n"));
    }
    ask.push_str("\r\n");
    stream.write_all(ask.as_bytes()).await?;
    // Byte by byte, so nothing the shard sends after the answer is read.
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(HTTP_HEAD_END) {
        if head.len() >= HTTP_HEAD_MAX {
            return Err(refused("the HTTP proxy's answer has no end"));
        }
        stream.read_exact(&mut byte).await?;
        head.push(byte[0]);
    }
    let status = String::from_utf8_lossy(&head);
    let code = status.split_whitespace().nth(1).unwrap_or_default();
    if code != HTTP_OK {
        return Err(refused(format!(
            "the HTTP proxy would not connect to the shard (status {code})"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    const PING: &[u8] = b"ping";

    #[test]
    fn a_proxy_is_read_from_its_url_and_hides_its_password() {
        let proxy: Proxy = "socks5://ann:secret@10.0.0.2:1080".parse().unwrap();
        assert_eq!(proxy.kind, ProxyKind::Socks5);
        assert_eq!((proxy.host.as_str(), proxy.port), ("10.0.0.2", 1080));
        assert!(!format!("{proxy}").contains("secret"));
        assert!(!format!("{proxy:?}").contains("secret"));
        assert_eq!(proxy.url(), "socks5://ann:secret@10.0.0.2:1080");
        let http: Proxy = "http://proxy.local:3128/".parse().unwrap();
        assert_eq!(http.kind, ProxyKind::HttpConnect);
        assert!("ftp://x:1".parse::<Proxy>().is_err());
        assert!("socks5://nohost".parse::<Proxy>().is_err());
        assert!("socks5://ann@h:1".parse::<Proxy>().is_err());
    }

    /// A proxy that speaks SOCKS5 with a password, then echoes.
    async fn fake_socks5() -> std::net::SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut hello = [0u8; 3];
            s.read_exact(&mut hello).await.unwrap();
            assert_eq!(hello, [SOCKS_VERSION, 1, SOCKS_AUTH_PASSWORD]);
            s.write_all(&[SOCKS_VERSION, SOCKS_AUTH_PASSWORD])
                .await
                .unwrap();
            let mut head = [0u8; 2];
            s.read_exact(&mut head).await.unwrap();
            let mut user = vec![0u8; usize::from(head[1])];
            s.read_exact(&mut user).await.unwrap();
            let mut len = [0u8; 1];
            s.read_exact(&mut len).await.unwrap();
            let mut password = vec![0u8; usize::from(len[0])];
            s.read_exact(&mut password).await.unwrap();
            let ok = user == b"ann" && password == b"secret";
            s.write_all(&[SOCKS_PASSWORD_VERSION, u8::from(!ok)])
                .await
                .unwrap();
            let mut ask = [0u8; 4];
            s.read_exact(&mut ask).await.unwrap();
            assert_eq!(ask[3], SOCKS_ADDRESS_NAME);
            let mut len = [0u8; 1];
            s.read_exact(&mut len).await.unwrap();
            let mut rest = vec![0u8; usize::from(len[0]) + PORT_LEN];
            s.read_exact(&mut rest).await.unwrap();
            s.write_all(&[
                SOCKS_VERSION,
                SOCKS_REPLY_OK,
                0,
                SOCKS_ADDRESS_IPV4,
                0,
                0,
                0,
                0,
                0,
                0,
            ])
            .await
            .unwrap();
            let mut echo = [0u8; 4];
            s.read_exact(&mut echo).await.unwrap();
            s.write_all(&echo).await.unwrap();
        });
        addr
    }

    /// A proxy that answers CONNECT with the status given, then echoes.
    async fn fake_http(status: &'static str) -> std::net::SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(HTTP_HEAD_END) {
                s.read_exact(&mut byte).await.unwrap();
                head.push(byte[0]);
            }
            let text = String::from_utf8(head).unwrap();
            assert!(text.starts_with("CONNECT shard.example:2593 HTTP/1.1"));
            s.write_all(format!("HTTP/1.1 {status} x\r\n\r\n").as_bytes())
                .await
                .unwrap();
            let mut echo = [0u8; 4];
            if s.read_exact(&mut echo).await.is_ok() {
                let _ = s.write_all(&echo).await;
            }
        });
        addr
    }

    async fn echoed(mut stream: TcpStream) -> Vec<u8> {
        stream.write_all(PING).await.unwrap();
        let mut back = vec![0u8; PING.len()];
        stream.read_exact(&mut back).await.unwrap();
        back
    }

    #[tokio::test]
    async fn a_socks5_proxy_carries_the_link_after_its_password() {
        let addr = fake_socks5().await;
        let proxy: Proxy = format!("socks5://ann:secret@{addr}").parse().unwrap();
        let stream = connect(Some(&proxy), "shard.example", 2593).await.unwrap();
        assert_eq!(echoed(stream).await, PING);
    }

    #[tokio::test]
    async fn an_http_proxy_carries_the_link_only_after_a_yes() {
        let addr = fake_http(HTTP_OK).await;
        let proxy: Proxy = format!("http://{addr}").parse().unwrap();
        let stream = connect(Some(&proxy), "shard.example", 2593).await.unwrap();
        assert_eq!(echoed(stream).await, PING);
        let addr = fake_http("407").await;
        let proxy: Proxy = format!("http://{addr}").parse().unwrap();
        assert!(connect(Some(&proxy), "shard.example", 2593).await.is_err());
    }
}
