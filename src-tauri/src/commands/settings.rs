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

/// How many downloads run their network phase at once.
///
/// Three rather than the previous two: the network phase is cheap to parallelise,
/// unlike the encode phase it feeds.
pub const DEFAULT_NETWORK_CONCURRENCY: u32 = 3;
/// How many re-encodes run at once, across both the download and transcoding
/// pipelines.
///
/// One, and deliberately not derived from the core count: libx264 already scales
/// across every available core, so a second concurrent encode halves each one
/// rather than adding throughput.
pub const DEFAULT_CPU_CONCURRENCY: u32 = 1;

/// Accepted range for [`Settings::max_network_concurrency`].
const NETWORK_CONCURRENCY_RANGE: std::ops::RangeInclusive<u32> = 1..=8;
/// Accepted range for [`Settings::max_cpu_concurrency`].
///
/// Capped at two, and the second slot exists to keep the app responsive rather
/// than to go faster — see [`DEFAULT_CPU_CONCURRENCY`].
const CPU_CONCURRENCY_RANGE: std::ops::RangeInclusive<u32> = 1..=2;

/// Read a stored concurrency limit.
///
/// `None` means the stored value is unusable — not a number, or outside `range` —
/// and the caller keeps its default. Zero is outside every range on purpose: a
/// limit of zero would stop every download and every encode without reporting
/// anything, which is far worse than ignoring a bad value.
fn parse_concurrency(value: &str, range: &std::ops::RangeInclusive<u32>) -> Option<u32> {
    value
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|limit| range.contains(limit))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub download_path: String,
    pub auto_organize: bool,
    pub transcoding_preset: String,
    pub detect_clipboard: bool,
    pub theme: String,
    pub max_network_concurrency: u32,
    pub max_cpu_concurrency: u32,
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
            max_network_concurrency: DEFAULT_NETWORK_CONCURRENCY,
            max_cpu_concurrency: DEFAULT_CPU_CONCURRENCY,
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
                // Same contract as `theme`: an unusable value leaves the default
                // standing for this run and is not corrected in the database.
                // Each limit is checked against its own range, so a value that is
                // valid for one is not thereby accepted for the other.
                "max_network_concurrency" => {
                    if let Some(limit) = parse_concurrency(&value, &NETWORK_CONCURRENCY_RANGE) {
                        self.max_network_concurrency = limit;
                    }
                }
                "max_cpu_concurrency" => {
                    if let Some(limit) = parse_concurrency(&value, &CPU_CONCURRENCY_RANGE) {
                        self.max_cpu_concurrency = limit;
                    }
                }
                _ => {}
            }
        }
        self
    }
}

/// Read the effective settings.
///
/// Extracted from `get_settings` so other commands can read a setting without
/// going through the IPC layer — the CPU budget's capacity comes from here.
pub async fn load_settings(db_instances: &DbInstances) -> Result<Settings, String> {
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

/// The configured CPU concurrency, or the default when the settings cannot be
/// read.
///
/// A download or transcode must not fail because the database is not loaded yet;
/// falling back to the default limit is the conservative outcome — it is the
/// smaller value, so it never widens the budget by accident.
pub async fn cpu_concurrency_or_default(db_instances: &DbInstances) -> u32 {
    load_settings(db_instances)
        .await
        .map(|settings| settings.max_cpu_concurrency)
        .unwrap_or(DEFAULT_CPU_CONCURRENCY)
}

#[tauri::command]
pub async fn get_settings(
    db_instances: State<'_, DbInstances>,
) -> Result<Settings, String> {
    load_settings(&db_instances).await
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

    // Concurrency limits. Both are numeric, so unlike `theme` they cannot be
    // validated with an allowlist; each has its own accepted range instead.

    #[test]
    fn concurrency_defaults_are_three_network_and_one_cpu() {
        let settings = Settings::default();
        assert_eq!(settings.max_network_concurrency, 3);
        // One, not a value derived from the core count: libx264 already scales
        // across every core, so a second concurrent encode halves each rather
        // than adding throughput.
        assert_eq!(settings.max_cpu_concurrency, 1);
    }

    /// The decision table from the settings-management spec, verbatim.
    ///
    /// Each row is (key, stored value, expected reported value).
    const STORED_CONCURRENCY_VALUES: [(&str, &str, u32); 8] = [
        ("max_network_concurrency", "4", 4),
        ("max_network_concurrency", "1", 1),
        ("max_network_concurrency", "8", 8),
        ("max_network_concurrency", "0", 3),
        ("max_network_concurrency", "9", 3),
        ("max_network_concurrency", "abc", 3),
        ("max_cpu_concurrency", "2", 2),
        ("max_cpu_concurrency", "3", 1),
    ];

    #[test]
    fn stored_concurrency_values_are_parsed_and_range_checked() {
        for (key, stored, expected) in STORED_CONCURRENCY_VALUES {
            let merged =
                Settings::default().merge(vec![(key.to_string(), stored.to_string())]);
            let actual = match key {
                "max_network_concurrency" => merged.max_network_concurrency,
                "max_cpu_concurrency" => merged.max_cpu_concurrency,
                other => panic!("unexpected key in the table: {}", other),
            };
            assert_eq!(
                actual, expected,
                "{} stored as {:?} must be reported as {}",
                key, stored, expected
            );
        }
    }

    #[test]
    fn out_of_range_cpu_concurrency_falls_back_to_the_default() {
        // The scenario stated in the spec: 8 is a plausible value for the network
        // limit but outside the CPU limit's range, so it must not be accepted for
        // the CPU limit just because it parses as a number.
        let merged = Settings::default()
            .merge(vec![("max_cpu_concurrency".to_string(), "8".to_string())]);
        assert_eq!(merged.max_cpu_concurrency, 1);
    }

    #[test]
    fn zero_concurrency_never_reaches_the_caller() {
        // A limit of zero would mean no download ever starts and nothing is ever
        // encoded — a silent deadlock rather than a visible error. Both ranges
        // start at 1 so a hand-edited or truncated value cannot produce it.
        for key in ["max_network_concurrency", "max_cpu_concurrency"] {
            let merged = Settings::default().merge(vec![(key.to_string(), "0".to_string())]);
            assert!(
                merged.max_network_concurrency >= 1 && merged.max_cpu_concurrency >= 1,
                "{} stored as 0 must not yield a zero limit",
                key
            );
        }
    }

    #[test]
    fn a_rejected_concurrency_value_leaves_the_other_limit_alone() {
        // The two limits are both unsigned integers, so a merge that mixed them
        // up would be invisible to the type system. Rejecting one must not change
        // the other.
        let merged = Settings::default().merge(vec![
            ("max_network_concurrency".to_string(), "9".to_string()),
            ("max_cpu_concurrency".to_string(), "2".to_string()),
        ]);
        assert_eq!(merged.max_network_concurrency, 3, "rejected, so default");
        assert_eq!(merged.max_cpu_concurrency, 2, "accepted independently");
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
