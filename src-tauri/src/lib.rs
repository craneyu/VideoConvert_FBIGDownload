mod commands;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations(
                    "sqlite:vidbridge.db",
                    vec![
                        tauri_plugin_sql::Migration {
                            version: 1,
                            description: "create download_history table",
                            sql: "CREATE TABLE download_history (
                            id INTEGER PRIMARY KEY AUTOINCREMENT,
                            url TEXT NOT NULL,
                            title TEXT,
                            status TEXT NOT NULL,
                            file_path TEXT,
                            source TEXT,
                            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                        );",
                            kind: tauri_plugin_sql::MigrationKind::Up,
                        },
                        tauri_plugin_sql::Migration {
                            version: 2,
                            description: "create settings table",
                            sql: "CREATE TABLE settings (
                                key TEXT PRIMARY KEY,
                                value TEXT NOT NULL
                            );",
                            kind: tauri_plugin_sql::MigrationKind::Up,
                        },
                        // Migration 1 defaulted created_at to CURRENT_TIMESTAMP, which
                        // SQLite evaluates in UTC, while the UI renders the value as if
                        // it were local time. Migration 1 cannot be edited — installs
                        // that already ran it never re-run it — so the default change
                        // and the backfill are applied here instead.
                        //
                        // Both MUST stay in this single migration version. Changing the
                        // default without backfilling (or vice versa) leaves UTC and
                        // local rows mixed in a column that carries no timezone, and
                        // there is then no way to tell which rows were converted.
                        //
                        // SQLite cannot alter a column default in place, hence the
                        // rebuild. Rows are copied before the old table is dropped.
                        // datetime(created_at, 'localtime') reads the stored value as
                        // UTC and converts it using the host's zone, so this is correct
                        // outside UTC+8 as well.
                        //
                        // The four statements below rely on two guarantees verified in
                        // sqlx 0.8.6, which backs this plugin:
                        //   - Executor::execute delegates to execute_many, so every
                        //     statement in this string runs, not just the first.
                        //   - The migrator wraps the script and its version bookkeeping
                        //     in one transaction, so a failure part-way rolls the whole
                        //     thing back and version 3 is not recorded. There is no
                        //     half-applied state to recover from.
                        tauri_plugin_sql::Migration {
                            version: 3,
                            description: "store download_history.created_at in local time",
                            sql: "CREATE TABLE download_history_local (
                                id INTEGER PRIMARY KEY AUTOINCREMENT,
                                url TEXT NOT NULL,
                                title TEXT,
                                status TEXT NOT NULL,
                                file_path TEXT,
                                source TEXT,
                                created_at DATETIME DEFAULT (datetime('now', 'localtime'))
                            );
                            INSERT INTO download_history_local
                                (id, url, title, status, file_path, source, created_at)
                                SELECT id, url, title, status, file_path, source,
                                       datetime(created_at, 'localtime')
                                FROM download_history;
                            DROP TABLE download_history;
                            ALTER TABLE download_history_local RENAME TO download_history;",
                            kind: tauri_plugin_sql::MigrationKind::Up,
                        }
                    ],
                )
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &settings_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "settings" => {
                        // For now, just show the main window as a placeholder for settings
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                window.hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::download::fetch_video_info,
            commands::download::download_video,
            commands::download::open_folder,
            commands::transcode::transcode_video,
            commands::settings::get_settings,
            commands::settings::update_setting,
            commands::utils::check_dependencies,
            commands::utils::install_dependencies,
            commands::utils::read_clipboard_text
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
