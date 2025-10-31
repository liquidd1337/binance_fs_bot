use teloxide::prelude::*;
use teloxide::types::ChatId;

pub struct TgBot {
    bot: Bot,
    chat_id: ChatId,
}

impl TgBot {
    /// Создаёт Telegram-бота с токеном и ID чата
    pub fn new(token: &str, chat_id: i64) -> Self {
        Self {
            bot: Bot::new(token.to_string()),
            chat_id: ChatId(chat_id),
        }
    }

    /// Отправляет сообщение в Telegram
    pub async fn send_message(&self, text: &str) {
        if let Err(err) = self.bot.send_message(self.chat_id, text).await {
            eprintln!("❌ Ошибка отправки в Telegram: {}", err);
        }
    }
}
