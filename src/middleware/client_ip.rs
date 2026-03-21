use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::Next,
    response::Response,
};
use std::net::{IpAddr, SocketAddr};

use crate::server::AppState;

#[derive(Clone, Copy, Debug)]
pub struct ClientIp(pub IpAddr);

pub async fn resolve_client_ip(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let peer_ip = addr.ip();

    let client_ip = match state.config.trusted_proxy {
        Some(proxy) if proxy == peer_ip => request
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next())
            .and_then(|s| s.trim().parse::<IpAddr>().ok())
            .unwrap_or(peer_ip),
        _ => peer_ip,
    };

    request.extensions_mut().insert(ClientIp(client_ip));
    next.run(request).await
}
