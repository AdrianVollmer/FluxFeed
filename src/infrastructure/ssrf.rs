use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum SsrfError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("URL resolves to private/internal IP address")]
    PrivateIpAddress,

    #[error("DNS resolution failed: {0}")]
    DnsResolutionFailed(String),

    #[error("URL scheme not allowed: {0}")]
    InvalidScheme(String),
}

/// Check if an IPv4 address is private/internal
fn is_private_ipv4(ip: &Ipv4Addr) -> bool {
    // Loopback (127.0.0.0/8)
    if ip.is_loopback() {
        return true;
    }

    // Private networks (RFC 1918)
    // 10.0.0.0/8
    if ip.octets()[0] == 10 {
        return true;
    }
    // 172.16.0.0/12
    if ip.octets()[0] == 172 && (ip.octets()[1] >= 16 && ip.octets()[1] <= 31) {
        return true;
    }
    // 192.168.0.0/16
    if ip.octets()[0] == 192 && ip.octets()[1] == 168 {
        return true;
    }

    // Link-local (169.254.0.0/16) - includes AWS metadata endpoint
    if ip.octets()[0] == 169 && ip.octets()[1] == 254 {
        return true;
    }

    // Localhost alternatives
    // 0.0.0.0/8 (current network, often used as localhost)
    if ip.octets()[0] == 0 {
        return true;
    }

    // Documentation addresses (shouldn't be routable)
    // 192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24
    if (ip.octets()[0] == 192 && ip.octets()[1] == 0 && ip.octets()[2] == 2)
        || (ip.octets()[0] == 198 && ip.octets()[1] == 51 && ip.octets()[2] == 100)
        || (ip.octets()[0] == 203 && ip.octets()[1] == 0 && ip.octets()[2] == 113)
    {
        return true;
    }

    false
}

/// Check if an IPv6 address is private/internal
fn is_private_ipv6(ip: &Ipv6Addr) -> bool {
    // Loopback (::1)
    if ip.is_loopback() {
        return true;
    }

    // Unspecified (::)
    if ip.is_unspecified() {
        return true;
    }

    // Link-local (fe80::/10)
    let segments = ip.segments();
    if segments[0] & 0xffc0 == 0xfe80 {
        return true;
    }

    // Unique local addresses (fc00::/7) - equivalent to private IPv4
    if segments[0] & 0xfe00 == 0xfc00 {
        return true;
    }

    // IPv4-mapped IPv6 addresses (::ffff:0:0/96)
    // Check if it maps to a private IPv4
    if let Some(ipv4) = ip.to_ipv4_mapped() {
        return is_private_ipv4(&ipv4);
    }

    false
}

/// Check if an IP address is private/internal
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_private_ipv4(v4),
        IpAddr::V6(v6) => is_private_ipv6(v6),
    }
}

/// Validate a URL for SSRF protection
///
/// This function:
/// 1. Parses the URL and validates the scheme
/// 2. Resolves the hostname to IP addresses
/// 3. Checks that none of the resolved IPs are private/internal
///
/// This should be called both at feed creation time and at fetch time
/// to protect against DNS rebinding attacks.
pub fn validate_url(url_str: &str) -> Result<(), SsrfError> {
    // Parse URL
    let url = Url::parse(url_str).map_err(|e| SsrfError::InvalidUrl(e.to_string()))?;

    // Only allow http and https
    match url.scheme() {
        "http" | "https" => {}
        scheme => return Err(SsrfError::InvalidScheme(scheme.to_string())),
    }

    // Get host
    let host = url
        .host_str()
        .ok_or_else(|| SsrfError::InvalidUrl("No host in URL".to_string()))?;

    // Get port (default to 80 for http, 443 for https)
    let port = url.port_or_known_default().unwrap_or(80);

    // Resolve hostname to IP addresses
    let socket_addr = format!("{}:{}", host, port);
    let addrs: Vec<_> = socket_addr
        .to_socket_addrs()
        .map_err(|e| SsrfError::DnsResolutionFailed(e.to_string()))?
        .collect();

    if addrs.is_empty() {
        return Err(SsrfError::DnsResolutionFailed(
            "No addresses resolved".to_string(),
        ));
    }

    // Check all resolved IPs
    for addr in addrs {
        if is_private_ip(&addr.ip()) {
            tracing::warn!(
                "SSRF protection: URL {} resolves to private IP {}",
                url_str,
                addr.ip()
            );
            return Err(SsrfError::PrivateIpAddress);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_private_ipv4_loopback() {
        assert!(is_private_ipv4(&Ipv4Addr::new(127, 0, 0, 1)));
        assert!(is_private_ipv4(&Ipv4Addr::new(127, 255, 255, 255)));
    }

    #[test]
    fn test_private_ipv4_10_network() {
        assert!(is_private_ipv4(&Ipv4Addr::new(10, 0, 0, 1)));
        assert!(is_private_ipv4(&Ipv4Addr::new(10, 255, 255, 255)));
    }

    #[test]
    fn test_private_ipv4_172_network() {
        assert!(is_private_ipv4(&Ipv4Addr::new(172, 16, 0, 1)));
        assert!(is_private_ipv4(&Ipv4Addr::new(172, 31, 255, 255)));
        assert!(!is_private_ipv4(&Ipv4Addr::new(172, 15, 0, 1)));
        assert!(!is_private_ipv4(&Ipv4Addr::new(172, 32, 0, 1)));
    }

    #[test]
    fn test_private_ipv4_192_168_network() {
        assert!(is_private_ipv4(&Ipv4Addr::new(192, 168, 0, 1)));
        assert!(is_private_ipv4(&Ipv4Addr::new(192, 168, 255, 255)));
    }

    #[test]
    fn test_private_ipv4_link_local() {
        // AWS metadata endpoint
        assert!(is_private_ipv4(&Ipv4Addr::new(169, 254, 169, 254)));
        assert!(is_private_ipv4(&Ipv4Addr::new(169, 254, 0, 1)));
    }

    #[test]
    fn test_public_ipv4() {
        assert!(!is_private_ipv4(&Ipv4Addr::new(8, 8, 8, 8)));
        assert!(!is_private_ipv4(&Ipv4Addr::new(1, 1, 1, 1)));
        assert!(!is_private_ipv4(&Ipv4Addr::new(104, 16, 0, 1)));
    }

    #[test]
    fn test_private_ipv6_loopback() {
        assert!(is_private_ipv6(&Ipv6Addr::LOCALHOST));
    }

    #[test]
    fn test_private_ipv6_link_local() {
        assert!(is_private_ipv6(&Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1)));
    }

    #[test]
    fn test_private_ipv6_unique_local() {
        assert!(is_private_ipv6(&Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 1)));
        assert!(is_private_ipv6(&Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 1)));
    }
}
