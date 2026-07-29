use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tauri::State;
use tauri_plugin_sql::{DbInstances, DbPool};
use sqlx::Row;

/// The only theme modes the frontend knows how to resolve. `update_setting` is a
/// generic key-value writer with no per-key validation, so anything may end up in
/// the database; treating this list as an allowlist on the way out keeps an
/// unrecognised value from reaching the `data-theme` contract, which admits only
/// "light" or "dark".
const VALID_THEMES: [&str; 3] = ["system", "light", "dark"];

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub download_path: String,
    pub auto_organize: bool,
    pub transcoding_preset: String,
    pub detect_clipboard: bool,
    pub theme: String,
}

impl Default for Settings {
    fn default() -> Self {
        let download_path = dirs::download_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "".to_string());

        Self {
            download_path,
            auto_organize: false,
            transcoding_preset: "Balanced".to_string(),
            detect_clipboard: true,
            theme: "system".to_string(),
        }
    }
}

impl Settings {
    pub fn merge(mut self, db_values: Vec<(String, String)>) -> Self {
        for (key, value) in db_values {
            match key.as_str() {
                "download_path" => self.download_path = value,
                "auto_organize" => {
                    if let Ok(b) = value.parse::<bool>() {
                        self.auto_organize = b;
                    }
                }
                "transcoding_preset" => self.transcoding_preset = value,
                "detect_clipboard" => {
                    if let Ok(b) = value.parse::<bool>() {
                        self.detect_clipboard = b;
                    }
                }
                // An unrecognised value is left alone rather than corrected in
                // place: the default stands for this run, and nothing is written
                // back to the database.
                "theme" => {
                    if VALID_THEMES.contains(&value.as_str()) {
                        self.theme = value;
                    }
                }
                _ => {}
            }
        }
        self
    }
}

#[tauri::command]
pub async fn get_settings(
    db_instances: State<'_, DbInstances>,
) -> Result<Settings, String> {
    let instances = db_instances.0.read().await;
    let db = instances
        .get("sqlite:vidbridge.db")
        .ok_or("Database not loaded")?;

    let mut db_values = Vec::new();
    let DbPool::Sqlite(pool) = db;
    let rows = sqlx::query("SELECT key, value FROM settings")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    for row in rows {
        let k: String = row.try_get("key").unwrap_or_default();
        let v: String = row.try_get("value").unwrap_or_default();
        db_values.push((k, v));
    }

    Ok(Settings::default().merge(db_values))
}

#[tauri::command]
pub async fn update_setting(
    key: String,
    value: JsonValue,
    db_instances: State<'_, DbInstances>,
) -> Result<(), String> {
    let instances = db_instances.0.read().await;
    let db = instances
        .get("sqlite:vidbridge.db")
        .ok_or("Database not loaded")?;

    let value_str = match value {
        JsonValue::String(s) => s,
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => n.to_string(),
        _ => value.to_string(),
    };

    let DbPool::Sqlite(pool) = db;
    sqlx::query("INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(key)
        .bind(value_str)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert_eq!(settings.auto_organize, false);
        assert_eq!(settings.transcoding_preset, "Balanced");
        assert_eq!(settings.detect_clipboard, true);
        // download_path comes from the OS and can legitimately be empty on a CI
        // runner that provides no Downloads directory, so it is not asserted here.
    }

    #[test]
    fn test_settings_merge() {
        let default_settings = Settings::default();
        let db_values = vec![
            ("auto_organize".to_string(), "true".to_string()),
        ];
        let merged = default_settings.clone().merge(db_values);
        
        // Overridden by db
        assert_eq!(merged.auto_organize, true);
        // Kept default
        assert_eq!(merged.transcoding_preset, "Balanced");
        assert_eq!(merged.download_path, default_settings.download_path);
    }

    #[test]
    fn test_theme_defaults_to_system() {
        // "system" keeps the pre-existing behaviour of following the OS colour
        // scheme, so upgrading an installation must not change what users see.
        assert_eq!(Settings::default().theme, "system");
    }

    #[test]
    fn test_theme_accepts_the_three_valid_modes() {
        for value in ["system", "light", "dark"] {
            let merged =
                Settings::default().merge(vec![("theme".to_string(), value.to_string())]);
            assert_eq!(merged.theme, value, "theme {:?} must be accepted", value);
        }
    }

    #[test]
    fn test_theme_falls_back_to_system_for_unrecognized_values() {
        // A hand-edited database, or a mode a future version removed, must
        // degrade to the default instead of reaching the DOM contract, which
        // only ever allows "light" or "dark".
        for value in ["", "sepia"] {
            let merged =
                Settings::default().merge(vec![("theme".to_string(), value.to_string())]);
            assert_eq!(
                merged.theme, "system",
                "unrecognized theme {:?} must fall back to system",
                value
            );
        }
    }
}
