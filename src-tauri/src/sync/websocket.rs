// ============================================================
// EXSUL P2P WebSocket Engine
// Transport: tokio-tungstenite over TCP (works over Tailscale mesh VPN)
// Port: 8765
// Protocol: JSON message frames with typed discriminant "type"
// Trust model: node_id checked against trusted_nodes table
// ============================================================

use crate::db::Database;
use crate::events::{store, types::*};
use crate::sync::hlc::HybridLogicalClock;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_tungstenite::{accept_async, connect_async, tungstenite::Message};

pub const WS_PORT: u16 = 8765;

// ──────────────────────────────────────────────────────────────
// Wire-level message protocol
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Frame {
    /// First message both sides send after TCP connect.
    Hello {
        node_id: String,
        #[serde(default)]
        app_version: String,
    },
    /// Rejected — remote node_id not in our trusted list.
    Rejected { reason: String },
    /// Request delta events since a given HLC (or all if None).
    SyncReq { since_hlc: Option<String> },
    /// Response carrying event records.
    SyncData { events: Vec<EventRecord> },
    /// Final ACK after applying events.
    SyncAck { merged: u32 },
    /// Heartbeat.
    Ping,
    Pong,
}

impl Frame {
    fn to_text(&self) -> Result<Message, String> {
        serde_json::to_string(self)
            .map(Message::Text)
            .map_err(|e| e.to_string())
    }

    fn from_text(msg: &str) -> Result<Self, String> {
        serde_json::from_str(msg).map_err(|e| e.to_string())
    }
}

// ──────────────────────────────────────────────────────────────
// Shared peer registry (Arc<Mutex<…>> so Tauri commands can read it)
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivePeer {
    pub node_id: String,
    pub alias: Option<String>,
    pub ip: String,
    pub state: WsPeerState,
    pub last_sync: Option<String>,
    pub events_merged: u32,
    pub app_version: Option<String>,
}

pub struct WsManager {
    pub peers: Mutex<HashMap<String, LivePeer>>,
    pub server_running: Mutex<bool>,
}

impl WsManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            peers: Mutex::new(HashMap::new()),
            server_running: Mutex::new(false),
        })
    }
}

// ──────────────────────────────────────────────────────────────
// Helper: fetch local node_id from DB
// ──────────────────────────────────────────────────────────────

fn local_node_id(handle: &AppHandle) -> String {
    let db = handle.state::<Database>();
    let conn = db.conn.lock().unwrap();
    crate::db::queries::get_node_id(&conn).unwrap_or_else(|_| "unknown".to_string())
}

// ──────────────────────────────────────────────────────────────
// Helper: check if a node_id is trusted
// ──────────────────────────────────────────────────────────────

fn is_trusted(handle: &AppHandle, node_id: &str) -> bool {
    let db = handle.state::<Database>();
    let conn = db.conn.lock().unwrap();
    crate::db::queries::is_trusted_node(&conn, node_id).unwrap_or(false)
}

// ──────────────────────────────────────────────────────────────
// Helper: get alias for a node_id
// ──────────────────────────────────────────────────────────────

fn get_alias(handle: &AppHandle, node_id: &str) -> Option<String> {
    let db = handle.state::<Database>();
    let conn = db.conn.lock().unwrap();
    crate::db::queries::get_trusted_node_alias(&conn, node_id).ok().flatten()
}

// ──────────────────────────────────────────────────────────────
// Helper: fetch events since an HLC for outgoing sync
// ──────────────────────────────────────────────────────────────

fn events_since(handle: &AppHandle, since_hlc: Option<&str>) -> Vec<EventRecord> {
    let db = handle.state::<Database>();
    let conn = db.conn.lock().unwrap();
    crate::db::queries::get_events(&conn, since_hlc, Some(5000)).unwrap_or_default()
}

// ──────────────────────────────────────────────────────────────
// Helper: apply incoming events
// ──────────────────────────────────────────────────────────────

fn apply_events(handle: &AppHandle, events: Vec<EventRecord>) -> u32 {
    let db = handle.state::<Database>();
    let hlc = handle.state::<HybridLogicalClock>();
    let mut merged = 0u32;
    for ev in &events {
        match store::insert_remote_event(&db, &hlc, ev) {
            Ok(true) => merged += 1,
            _ => {}
        }
    }
    merged
}

// ──────────────────────────────────────────────────────────────
// Helper: emit UI event
// ──────────────────────────────────────────────────────────────

fn emit_peer_update(handle: &AppHandle, peers: Vec<LivePeer>) {
    let _ = handle.emit("sync://peers-updated", peers);
}

// ──────────────────────────────────────────────────────────────
// SERVER — accept incoming connections
// ──────────────────────────────────────────────────────────────

pub async fn start_server(handle: AppHandle, manager: Arc<WsManager>) {
    let addr = format!("0.0.0.0:{}", WS_PORT);
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            log::error!("[WS] Failed to bind {}: {}", addr, e);
            return;
        }
    };

    {
        let mut running = manager.server_running.lock().await;
        *running = true;
    }

    log::info!("[WS] Server listening on {}", addr);

    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                let handle_clone = handle.clone();
                let mgr_clone = manager.clone();
                tokio::spawn(async move {
                    handle_server_conn(handle_clone, mgr_clone, stream, peer_addr).await;
                });
            }
            Err(e) => {
                log::error!("[WS] Accept error: {}", e);
            }
        }
    }
}

async fn handle_server_conn(
    handle: AppHandle,
    manager: Arc<WsManager>,
    stream: TcpStream,
    peer_addr: SocketAddr,
) {
    let ws = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            log::warn!("[WS] Handshake failed from {}: {}", peer_addr, e);
            return;
        }
    };

    let (mut tx, mut rx) = ws.split();
    let local_id = local_node_id(&handle);
    let ip = peer_addr.ip().to_string();

    // Send Hello
    let local_version = handle.package_info().version.to_string();
    if let Ok(msg) = (Frame::Hello { node_id: local_id.clone(), app_version: local_version }).to_text() {
        let _ = tx.send(msg).await;
    }

    // Expect Hello back
    let (peer_node_id, peer_version) = match rx.next().await {
        Some(Ok(Message::Text(t))) => match Frame::from_text(&t) {
            Ok(Frame::Hello { node_id, app_version }) => (node_id, app_version),
            _ => {
                log::warn!("[WS] Expected Hello from {}", ip);
                return;
            }
        },
        _ => return,
    };

    // Trust check
    if !is_trusted(&handle, &peer_node_id) {
        let _ = tx
            .send(
                Frame::Rejected {
                    reason: format!("node {} not in trusted list", peer_node_id),
                }
                .to_text()
                .unwrap_or(Message::Close(None)),
            )
            .await;
        log::warn!("[WS] Rejected untrusted node {} from {}", peer_node_id, ip);
        return;
    }

    let alias = get_alias(&handle, &peer_node_id);
    log::info!("[WS] Trusted peer connected: {} ({})", peer_node_id, ip);

    // Store peer version in sync_state
    {
        let db = handle.state::<Database>();
        let conn = db.conn.lock().unwrap();
        let _ = conn.execute(
            "UPDATE sync_state SET remote_version = ?1 WHERE peer_node_id = ?2",
            rusqlite::params![peer_version, peer_node_id],
        );
    }

    // Register peer as connected
    {
        let mut peers = manager.peers.lock().await;
        peers.insert(
            peer_node_id.clone(),
            LivePeer {
                node_id: peer_node_id.clone(),
                alias: alias.clone(),
                ip: ip.clone(),
                state: WsPeerState::Connected,
                last_sync: None,
                events_merged: 0,
                app_version: Some(peer_version.clone()),
            },
        );
        let list: Vec<LivePeer> = peers.values().cloned().collect();
        emit_peer_update(&handle, list);
    }

    let mut total_merged = 0u32;

    // Process incoming frames until disconnect
    while let Some(msg) = rx.next().await {
        match msg {
            Ok(Message::Text(t)) => match Frame::from_text(&t) {
                Ok(Frame::SyncReq { since_hlc }) => {
                    // Peer requests our events
                    let evs = events_since(&handle, since_hlc.as_deref());
                    let resp = Frame::SyncData { events: evs };
                    if let Ok(m) = resp.to_text() {
                        let _ = tx.send(m).await;
                    }
                }
                Ok(Frame::SyncData { events }) => {
                    // Peer sends their events
                    let merged = apply_events(&handle, events);
                    total_merged += merged;
                    let ack = Frame::SyncAck { merged };
                    if let Ok(m) = ack.to_text() {
                        let _ = tx.send(m).await;
                    }
                    // Update peer last_sync
                    let now = chrono::Utc::now().to_rfc3339();
                    let mut peers = manager.peers.lock().await;
                    if let Some(p) = peers.get_mut(&peer_node_id) {
                        p.last_sync = Some(now);
                        p.events_merged = total_merged;
                    }
                    let list: Vec<LivePeer> = peers.values().cloned().collect();
                    emit_peer_update(&handle, list);
                }
                Ok(Frame::Ping) => {
                    let _ = tx.send(Frame::Pong.to_text().unwrap()).await;
                }
                Ok(Frame::SyncAck { merged }) => {
                    log::info!("[WS] Peer {} acked {} events", peer_node_id, merged);
                }
                _ => {}
            },
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }

    // Peer disconnected
    {
        let mut peers = manager.peers.lock().await;
        if let Some(p) = peers.get_mut(&peer_node_id) {
            p.state = WsPeerState::Disconnected;
        }
        let list: Vec<LivePeer> = peers.values().cloned().collect();
        emit_peer_update(&handle, list);
    }

    log::info!("[WS] Peer {} disconnected", peer_node_id);
}

// ──────────────────────────────────────────────────────────────
// CLIENT — connect to a remote peer and perform full sync
// ──────────────────────────────────────────────────────────────

pub async fn connect_to_peer(
    handle: AppHandle,
    manager: Arc<WsManager>,
    target_ip: String,
) -> Result<u32, String> {
    let url = format!("ws://{}:{}", target_ip, WS_PORT);
    let local_id = local_node_id(&handle);

    log::info!("[WS] Connecting to {} as {}", url, local_id);

    // Register as Connecting
    {
        let mut peers = manager.peers.lock().await;
        peers.entry(target_ip.clone()).or_insert(LivePeer {
            node_id: String::new(),
            alias: None,
            ip: target_ip.clone(),
            state: WsPeerState::Connecting,
            last_sync: None,
            events_merged: 0,
            app_version: None,
        });
        let list: Vec<LivePeer> = peers.values().cloned().collect();
        emit_peer_update(&handle, list);
    }

    let (ws, _) = connect_async(&url)
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    let (mut tx, mut rx) = ws.split();

    // Send Hello
    let local_version = handle.package_info().version.to_string();
    tx.send(Frame::Hello { node_id: local_id.clone(), app_version: local_version }.to_text()?)
        .await
        .map_err(|e| e.to_string())?;

    // Expect Hello back
    let (peer_node_id, peer_version) = match rx.next().await {
        Some(Ok(Message::Text(t))) => match Frame::from_text(&t)? {
            Frame::Hello { node_id, app_version } => (node_id, app_version),
            Frame::Rejected { reason } => return Err(format!("Rejected: {}", reason)),
            _ => return Err("Unexpected first frame".to_string()),
        },
        Some(Ok(Message::Close(_))) => return Err("Peer closed connection".to_string()),
        _ => return Err("No Hello received".to_string()),
    };

    // Check local trust
    if !is_trusted(&handle, &peer_node_id) {
        return Err(format!("Peer {} not in our trusted list — add them first", peer_node_id));
    }

    let alias = get_alias(&handle, &peer_node_id);

    log::info!("[WS] Trusted peer: {} at {}", peer_node_id, target_ip);

    // Store peer version in sync_state
    {
        let db = handle.state::<Database>();
        let conn = db.conn.lock().unwrap();
        let _ = conn.execute(
            "UPDATE sync_state SET remote_version = ?1 WHERE peer_node_id = ?2",
            rusqlite::params![peer_version, peer_node_id],
        );
    }

    // Update registry
    {
        let mut peers = manager.peers.lock().await;
        let entry = peers.entry(target_ip.clone()).or_insert(LivePeer {
            node_id: peer_node_id.clone(),
            alias: alias.clone(),
            ip: target_ip.clone(),
            state: WsPeerState::Connecting,
            last_sync: None,
            events_merged: 0,
            app_version: None,
        });
        entry.node_id = peer_node_id.clone();
        entry.alias = alias;
        entry.state = WsPeerState::Connected;
        entry.app_version = Some(peer_version.clone());
        let list: Vec<LivePeer> = peers.values().cloned().collect();
        emit_peer_update(&handle, list);
    }

    // --- Phase 1: we request their events ---
    // Determine since_hlc from sync_state for this peer
    let since_hlc = {
        let db = handle.state::<Database>();
        let conn = db.conn.lock().unwrap();
        crate::db::queries::get_peer_last_hlc(&conn, &peer_node_id).ok().flatten()
    };

    tx.send(Frame::SyncReq { since_hlc }.to_text()?)
        .await
        .map_err(|e| e.to_string())?;

    let mut total_merged = 0u32;

    // Receive their SyncData
    if let Some(Ok(Message::Text(t))) = rx.next().await {
        if let Ok(Frame::SyncData { events }) = Frame::from_text(&t) {
            total_merged += apply_events(&handle, events);
        }
    }

    // --- Phase 2: we send our events ---
    let our_events = events_since(&handle, None);
    tx.send(Frame::SyncData { events: our_events }.to_text()?)
        .await
        .map_err(|e| e.to_string())?;

    // Wait for their ACK
    if let Some(Ok(Message::Text(t))) = rx.next().await {
        if let Ok(Frame::SyncAck { merged }) = Frame::from_text(&t) {
            log::info!("[WS] Remote merged {} of our events", merged);
        }
    }

    let now = chrono::Utc::now().to_rfc3339();

    // Update registry
    {
        let mut peers = manager.peers.lock().await;
        if let Some(p) = peers.get_mut(&target_ip) {
            p.last_sync = Some(now);
            p.events_merged = total_merged;
            p.state = WsPeerState::Disconnected; // one-shot sync, not persistent
        }
        let list: Vec<LivePeer> = peers.values().cloned().collect();
        emit_peer_update(&handle, list);
    }

    log::info!("[WS] Sync complete with {}: {} events merged", peer_node_id, total_merged);
    Ok(total_merged)
}
