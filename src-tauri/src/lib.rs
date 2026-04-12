mod commands;
mod db;
mod events;
mod sync;

use sync::hlc::HybridLogicalClock;
use sync::websocket::WsManager;
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            db::init(app.handle())?;

            // Initialize HLC with the node_id from the database
            let node_id = {
                let db_state = app.handle().state::<db::Database>();
                let conn = db_state.conn.lock().unwrap();
                db::queries::get_node_id(&conn).expect("node_id must exist after db::init")
            };

            app.handle().manage(HybridLogicalClock::new(node_id));

            // Initialize WebSocket manager and auto-start server
            let ws_manager = WsManager::new();
            app.handle().manage(ws_manager.clone());

            let h = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                sync::websocket::start_server(h, ws_manager).await;
            });

            // On startup: check for overdue unconfirmed orders and emit event to frontend
            let h2 = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                let overdue_result = {
                    let db_state = h2.state::<db::Database>();
                    let conn = db_state.conn.lock().ok();
                    conn.and_then(|c| db::queries::get_overdue_unconfirmed_orders(&c).ok())
                };
                if let Some(overdue) = overdue_result {
                    if !overdue.is_empty() {
                        let _ = h2.emit("overdue-orders", overdue);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // ── Inventory ──
            commands::inventory::add_item,
            commands::inventory::update_item,
            commands::inventory::get_items,
            commands::inventory::get_item,
            commands::inventory::record_sale,
            commands::inventory::adjust_stock,
            commands::inventory::change_price,
            commands::inventory::save_item_image,
            commands::inventory::delete_item,
            commands::inventory::duplicate_item,
            commands::inventory::delete_all_items,
            // ── Events / Audit ──
            commands::events::get_events,
            commands::events::get_price_history,
            commands::audit::get_audit_logs,
            // ── Sync (CRDT) ──
            commands::sync::get_sync_state,
            commands::sync::merge_remote_events,
            commands::sync::get_node_id,
            // ── Backup ──
            commands::backup::export_backup,
            commands::backup::import_backup,
            commands::backup::import_backup_data,
            // ── Categories ──
            commands::categories::create_category,
            commands::categories::get_categories,
            commands::categories::update_category,
            commands::categories::delete_category,
            // ── Orders ──
            commands::orders::create_order,
            commands::orders::update_order_status,
            commands::orders::get_orders,
            commands::orders::get_order,
            commands::orders::add_order_item,
            commands::orders::get_order_items,
            // ── App Preset & Trusted Nodes ──
            commands::preset::get_app_preset,
            commands::preset::set_app_preset,
            commands::preset::get_trusted_nodes,
            commands::preset::add_trusted_node,
            commands::preset::remove_trusted_node,
            // ── Flowers ──
            commands::flowers::get_flower_sorts,
            commands::flowers::create_flower_sort,
            commands::flowers::update_flower_sort,
            commands::flowers::delete_flower_sort,
            commands::flowers::adjust_flower_stock,
            commands::flowers::get_flower_constants,
            commands::flowers::set_flower_constants,
            commands::flowers::package_flowers,
            commands::flowers::create_pack_assignment,
            commands::flowers::get_pack_assignments,
            commands::flowers::get_packaging_log,
            commands::flowers::update_pack_status,
            // ── WebSocket P2P ──
            commands::ws::start_ws_server,
            commands::ws::ws_connect_peer,
            commands::ws::get_ws_status,
            // ── App Version ──
            commands::version::get_app_version,
            commands::version::get_version_info,
            // ── Greenhouse ──
            commands::greenhouse::save_flower_photo,
            commands::greenhouse::log_greenhouse_harvest,
            commands::greenhouse::get_harvest_log,
            // ── Orders extended ──
            commands::orders::update_order_extended,
            commands::orders::confirm_order_deadline,
            commands::orders::get_overdue_unconfirmed_orders,
            commands::orders::check_order_shortages,
            // ── App Settings ──
            commands::app_settings::get_setting,
            commands::app_settings::set_setting,
            commands::app_settings::get_all_settings,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let handle = window.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    match sync::backup::create_encrypted_backup(&handle).await {
                        Ok(path) => log::info!("Auto-backup created: {:?}", path),
                        Err(e) => log::error!("Auto-backup failed: {}", e),
                    }
                });
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
