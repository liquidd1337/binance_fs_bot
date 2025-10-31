mod tg_bot;

use reqwest::Client;
use serde::Deserialize;
use std::{collections::HashMap, time::Duration};
use tokio::time::sleep;
//use chrono::Local;
//use toml::*;
use std::fs;
use tg_bot::TgBot;

#[derive(Debug, Deserialize)]
struct ExchangeInfo {
    symbols: Vec<SymbolInfo>,
}

#[derive(Debug, Deserialize)]
struct SymbolInfo {
    symbol: String,
    #[serde(default)]
    contractType: String, // у спота нет, поэтому игнорируем
}

#[derive(Debug, Deserialize)]
struct Config {
    diff_threshold: f64,
    update_interval: u64,
    telegram_token: String,
    chat_id: i64,
}

#[derive(Debug, Deserialize)]
struct Blacklist {
    blacklist: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PriceResponse {
    symbol: String,
    price: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Парсим конфиг
    let config_text = fs::read_to_string("config.toml")
        .expect("❌ Не удалось прочитать config.toml — убедись, что файл существует");
    let config: Config = toml::from_str(&config_text)
        .expect("❌ Ошибка парсинга config.toml — проверь формат");

    println!(
        "⚙️  Конфигурация: порог {:.2}% | интервал {} сек.\n",
        config.diff_threshold, config.update_interval
    );

    let blacklist_text = fs::read_to_string("blacklist.toml")
        .unwrap_or_else(|_| {
            println!("⚠️  Не найден blacklist.toml, продолжаю без него.");
            String::from("blacklist = []")
        });
    let blacklist: Blacklist = toml::from_str(&blacklist_text)
        .expect("❌ Ошибка парсинга blacklist.toml");

    let client = Client::new();
    let tg_bot = TgBot::new(&config.telegram_token, config.chat_id);

    //Получаем список бессрочных фьючерсов
    println!("Загружаю список фьючерсов...");
    let futures_info: ExchangeInfo = client
        .get("https://fapi.binance.com/fapi/v1/exchangeInfo")
        .send()
        .await?
        .json()
        .await?;

    //Получаем список спотовых пар
    println!("Загружаю список спотовых пар...");
    let spot_info: ExchangeInfo = client
        .get("https://api.binance.com/api/v3/exchangeInfo")
        .send()
        .await?
        .json()
        .await?;

    //Собираем список символов бессрочных фьючерсов, у которых есть спотовая пара
    let spot_symbols: HashMap<_, _> =
        spot_info.symbols.iter().map(|s| (s.symbol.clone(), true)).collect();

    let valid_symbols: Vec<String> = futures_info
        .symbols
        .into_iter()
        .filter(|f| f.contractType == "PERPETUAL" && spot_symbols.contains_key(&f.symbol))
        .map(|f| f.symbol)
        .collect();

     println!("✅ Найдено {} совпадающих пар", valid_symbols.len());
    tg_bot
        .send_message(&format!(
            "🚀 Мониторинг запущен\nПорог: {:.2}%\nИнтервал: {} сек.\nОтслеживаемых пар: {}",
            config.diff_threshold, config.update_interval, valid_symbols.len()
        ))
        .await;

    loop {
        //Получаем все цены одним запросом
        let futures_prices: Vec<PriceResponse> = client
            .get("https://fapi.binance.com/fapi/v1/ticker/price")
            .send()
            .await?
            .json()
            .await?;

        let spot_prices: Vec<PriceResponse> = client
            .get("https://api.binance.com/api/v3/ticker/price")
            .send()
            .await?
            .json()
            .await?;

        // Преобразуем в словари
        let fut_map: HashMap<_, _> = futures_prices
            .into_iter()
            .map(|p| (p.symbol, p.price))
            .collect();

        let spot_map: HashMap<_, _> = spot_prices
            .into_iter()
            .map(|p| (p.symbol, p.price))
            .collect();

        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let mut message = format!("📊 {}\nПары с разницей > {:.2}%:\n", now, config.diff_threshold);

        //Проверяем все пары
        let mut found = false;

        for symbol in &valid_symbols {
            //Скипаем пары, указанные в blacklist.toml
            if blacklist.blacklist.contains(symbol) {
                continue;
            }

            if let (Some(fut_str), Some(spot_str)) = (fut_map.get(symbol), spot_map.get(symbol)) {
                let fut_price: f64 = fut_str.parse().unwrap_or(0.0);
                let spot_price: f64 = spot_str.parse().unwrap_or(0.0);

                if fut_price > 0.0 {
                    let diff_pct = ((spot_price - fut_price) / fut_price) * 100.0;
                    if diff_pct > config.diff_threshold {
                        found = true;
                        message.push_str(&format!(
                            "{} | F: {:.4} | S: {:.4} | Δ: {:+.2}%\n",
                            symbol, fut_price, spot_price, diff_pct
                        ));
                    }
                }
            }
        }

        if found {
            println!("{}", message);
            tg_bot.send_message(&message).await;
        } else {
            continue;
        }

        //Ждём интервал
        println!("\nОжидание {} секунд перед следующим опросом...\n", config.update_interval);
        sleep(Duration::from_secs(config.update_interval)).await;
    }
}
