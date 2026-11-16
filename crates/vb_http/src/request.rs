use crate::RequestRejection;
use std::net::{IpAddr, SocketAddr};
use ureq::http::Uri;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum JsonMediaType {
    GitHub,
}

/// An explicit HTTPS endpoint and a single preselected public IPv4 peer.
/// TLS still validates api.github.com; this type proves neither ownership nor authority.
#[derive(Debug, Clone)]
pub struct PinnedGet {
    pub(crate) uri: Uri,
    pub(crate) address: SocketAddr,
    pub(crate) media: JsonMediaType,
}

impl PinnedGet {
    pub fn https(
        uri: &str,
        address: SocketAddr,
        media: JsonMediaType,
    ) -> Result<Self, RequestRejection> {
        if uri.len() > 2048 || uri.contains('#') {
            return Err(RequestRejection::Endpoint);
        }
        let uri: Uri = uri.parse().map_err(|_| RequestRejection::Endpoint)?;
        if uri.scheme_str() != Some("https")
            || !matches!(
                uri.authority().map(|a| a.as_str()),
                Some("api.github.com" | "api.github.com:443")
            )
        {
            return Err(RequestRejection::Endpoint);
        }
        if address.port() != 443 || !public_ipv4(address.ip()) {
            return Err(RequestRejection::Peer);
        }
        Ok(Self {
            uri,
            address,
            media,
        })
    }

    #[cfg(any(test, feature = "local-conformance"))]
    pub(crate) fn fixture(address: SocketAddr) -> Result<Self, RequestRejection> {
        if !address.ip().is_loopback() || address.port() == 0 {
            return Err(RequestRejection::Peer);
        }
        let uri = "http://fixture.invalid/observed"
            .parse()
            .map_err(|_| RequestRejection::Endpoint)?;
        Ok(Self {
            uri,
            address,
            media: JsonMediaType::GitHub,
        })
    }
}

fn public_ipv4(ip: IpAddr) -> bool {
    let IpAddr::V4(ip) = ip else {
        return false;
    };
    let [a, b, c, _] = ip.octets();
    !matches!(a, 0 | 10 | 127 | 224..=255)
        && !(a == 100 && (64..=127).contains(&b))
        && !(a == 169 && b == 254)
        && !(a == 172 && (16..=31).contains(&b))
        && !(a == 192 && (b == 0 || b == 168 || (b == 88 && c == 99)))
        && !(a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
        && !(a == 203 && b == 0 && c == 113)
}
