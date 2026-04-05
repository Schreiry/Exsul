use crate::events::types::{WsPeerStatus, WsServerStatus};
use crate::sync::websocket::{WsManager, WS_PORT};
use std::sync::Arc;
use tauri::{AppHandle, State};

/// Start the WebSocket server (idempotent).
#[tauri::command]
pub async fn start_ws_server(
    handle: AppHandle,
    manager: State<'_, Arc<WsManager>>,
) -> Result<(), String> {
    let running = *manager.server_running.lock().await;
    if running {
        return Ok(()); // already running
    }
    let mgr = manager.inner().clone();
    let h = handle.clone();
    tauri::async_runtime::spawn(async move {
        crate::sync::websocket::start_server(h, mgr).await;
    });
    Ok(())
}

/// Initiate a one-shot sync with a remote peer by IP address.
/// Returns the number of new events merged locally.
#[tauri::command]
pub async fn ws_connect_peer(
    handle: AppHandle,
    manager: State<'_, Arc<WsManager>>,
    target_ip: String,
) -> Result<u32, String> {
    let mgr = manager.inner().clone();
    crate::sync::websocket::connect_to_peer(handle, mgr, target_ip).await
}

/// Live status of the WS layer.
#[tauri::command]
pub async fn get_ws_status(
    manager: State<'_, Arc<WsManager>>,
) -> Result<WsServerStatus, String> {
    let running = *manager.server_running.lock().await;
    let peers_map = manager.peers.lock().await;
    let peers: Vec<WsPeerStatus> = peers_map
        .values()
        .map(|p| WsPeerStatus {
            node_id: p.node_id.clone(),
            alias: p.alias.clone(),
            ip: p.ip.clone(),
            state: p.state.clone(),
            last_sync: p.last_sync.clone(),
            events_merged: p.events_merged,
        })
        .collect();
    Ok(WsServerStatus {
        running,
        port: WS_PORT,
        peers,
    })
}
