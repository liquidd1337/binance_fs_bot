use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

/// Храним пары, по которым уже было уведомление
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CheckList {
    pub checked: HashMap<String, u64>,
}

impl CheckList {
    /// Загружаем список из файла `check.toml`
    pub fn load() -> Self {
        if let Ok(text) = fs::read_to_string("check.toml") {
            toml::from_str(&text).unwrap_or_default()
        } else {
            CheckList::default()
        }
    }

    /// Сохраняем список в файл `check.toml`
    pub fn save(&self) {
        if let Ok(toml_str) = toml::to_string_pretty(self) {
            if let Err(err) = fs::write("check.toml", toml_str) {
                eprintln!("⚠️ Ошибка записи check.toml: {}", err);
            }
        }
    }

    /// Добавляем пару с текущим временем
    pub fn add(&mut self, symbol: String) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.checked.insert(symbol, now);
    }

    /// Проверяем, можно ли уведомлять по паре
    pub fn should_notify(&self, symbol: &str) -> bool {
        !self.checked.contains_key(symbol)
    }

    /// Удаляем старые записи старше `hold_time_secs`
    pub fn cleanup(&mut self, hold_time_secs: u64) -> Vec<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut removed = Vec::new();
        self.checked.retain(|symbol, &mut ts| {
            let keep = now - ts < hold_time_secs;
            if !keep {
                removed.push(symbol.clone());
            }
            keep
        });

        removed
    }
}
