use teloxide::prelude::*;
use teloxide::utils::command::BotCommand;
use serde::{Deserialize, Serialize};
use log::{info, error};
use std::env;
use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::Mutex;
use once_cell::sync::Lazy;

// Глобальное хранилище состояний пользователей
static USER_STATES: Lazy<Mutex<HashMap<i64, UserState>>> = Lazy::new(|| Mutex::new(HashMap::new()));

// Структура для хранения состояния пользователя
#[derive(Clone)]
struct UserState {
    topic: Option<String>,
    page: usize,
    news: Vec<NewsItem>,
}

// Структура для представления новости
#[derive(Serialize, Deserialize, Clone)]
struct NewsItem {
    title: String,
    description: String,
    url: String,
    url_to_image: Option<String>,
}

// Перечисление для команд бота
#[derive(BotCommand, Clone)]
#[command(rename = "lowercase", description = "Доступные команды:")]
enum Command {
    #[command(description = "начать работу с ботом")]
    Start,
    #[command(description = "получить справку")]
    Help,
}

// Основная асинхронная функция запуска бота
#[tokio::main]
async fn main() {
    // Настройка логирования
    env_logger::init();
    
    // Загрузка переменных окружения из .env файла
    dotenv::dotenv().ok();
    
    // Получение токена бота из переменной окружения
    let bot_token = env::var("TELEGRAM_BOT_TOKEN")
        .expect("TELEGRAM_BOT_TOKEN не найден в переменных окружения");
    
    // Создание экземпляра бота
    let bot = Bot::new(bot_token);
    
    // Вывод информации о запуске
    info!("Запуск бота...");
    
    // Запуск бота с обработкой обновлений
    teloxide::repl(bot, handler).await;
}

// Асинхронная функция обработки сообщений
async fn handler(cx: UpdateWithCx<AutoSend<Bot>, Message>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Извлечение бота и сообщения из контекста
    let bot = &cx.bot;
    let msg = &cx.update;
    
    // Получение ID пользователя
    let user_id = msg.chat_id();
    
    // Обработка команд
    if let Some(text) = msg.text() {
        if text.starts_with('/') {
            // Парсинг команды
            match teloxide::utils::command::parse_command(text, &cx.bot.me().await?.username) {
                Ok((command, _)) => {
                    match command.parse::<Command>() {
                        Ok(Command::Start) => {
                            // Очистка состояния пользователя
                            {
                                let mut states = USER_STATES.lock().await;
                                states.remove(&user_id);
                            }
                            // Отправка приветственного сообщения
                            bot.send_message(user_id, "Здравствуйте, какие новости вы хотите узнать?")
                                .await?;
                            return Ok(());
                        }
                        Ok(Command::Help) => {
                            // Отправка справки
                            let help_text = "Этот бот позволяет получать новости по интересующей вас теме.\n\n\
                                           /start - перезапустить бота\n\
                                           /help - показать это сообщение\n\n\
                                           Просто введите тему (например, \"спорт\", \"технологии\", \"политика\"), и я найду для вас новости.";
                            bot.send_message(user_id, help_text)
                                .await?;
                            return Ok(());
                        }
                        Err(_) => {
                            bot.send_message(user_id, "Неизвестная команда. Используйте /help для получения списка команд.")
                                .await?;
                            return Ok(());
                        }
                    }
                }
                Err(_) => {
                    // Если это не команда, обрабатываем как тему для новостей
                    let topic = text.trim().to_string();
                    get_and_send_news(bot, user_id, &topic, 0).await?;
                    return Ok(());
                }
            }
        } else if text.trim().eq_ignore_ascii_case("следующая") || text.trim().eq_ignore_ascii_case("next") {
            // Обработка команды "следующая"
            let mut states = USER_STATES.lock().await;
            if let Some(state) = states.get_mut(&user_id) {
                if !state.news.is_empty() {
                    state.page = (state.page + 1) % state.news.len();
                    send_single_news(bot, user_id, &state.news[state.page], state.page + 1, state.news.len()).await?;
                } else {
                    bot.send_message(user_id, "Сначала введите тему для поиска новостей.")
                        .await?;
                }
            } else {
                bot.send_message(user_id, "Сначала введите тему для поиска новостей.")
                    .await?;
            }
            return Ok(());
        } else if text.trim().eq_ignore_ascii_case("предыдущая") || text.trim().eq_ignore_ascii_case("previous") {
            // Обработка команды "предыдущая"
            let mut states = USER_STATES.lock().await;
            if let Some(state) = states.get_mut(&user_id) {
                if !state.news.is_empty() {
                    state.page = if state.page == 0 { state.news.len() - 1 } else { state.page - 1 };
                    send_single_news(bot, user_id, &state.news[state.page], state.page + 1, state.news.len()).await?;
                } else {
                    bot.send_message(user_id, "Сначала введите тему для поиска новостей.")
                        .await?;
                }
            } else {
                bot.send_message(user_id, "Сначала введите тему для поиска новостей.")
                    .await?;
            }
            return Ok(());
        } else {
            // Обработка текста как темы для новостей
            let topic = text.trim().to_string();
            get_and_send_news(bot, user_id, &topic, 0).await?;
            return Ok(());
        }
    }
    
    // Если сообщение не текстовое
    bot.send_message(user_id, "Пожалуйста, введите текстовую команду или тему для поиска новостей.")
        .await?;
    
    Ok(())
}

// Асинхронная функция получения и отправки новостей
async fn get_and_send_news(
    bot: &AutoSend<Bot>,
    chat_id: i64,
    topic: &str,
    page: usize,
) -> Result<()> {
    // Получение API ключа из переменных окружения
    let api_key = env::var("NEWS_API_KEY")
        .expect("NEWS_API_KEY не найден в переменных окружения");
    
    // Формирование URL для запроса к NewsAPI
    let url = format!(
        "https://newsapi.org/v2/everything?q={}&sortBy=publishedAt&apiKey={}&pageSize=5&page={}",
        topic,
        api_key,
        page + 1
    );
    
    // Выполнение HTTP запроса
    let response = reqwest::get(&url).await?;
    
    // Проверка статуса ответа
    if !response.status().is_success() {
        bot.send_message(chat_id, "Ошибка при получении новостей. Попробуйте позже.")
            .await?;
        return Ok(());
    }
    
    // Десериализация JSON ответа
    let news_response: NewsApiResponse = response.json().await?;
    
    // Проверка, есть ли новости
    if news_response.articles.is_empty() {
        bot.send_message(chat_id, "Новостей по вашему запросу не найдено.")
            .await?;
        return Ok(());
    }
    
    // Сохранение новостей в состоянии пользователя
    {
        let mut states = USER_STATES.lock().await;
        let state = states.entry(chat_id).or_insert(UserState {
            topic: Some(topic.to_string()),
            page: 0,
            news: Vec::new(),
        });
        state.topic = Some(topic.to_string());
        state.page = 0;
        state.news = news_response.articles.clone();
    }
    
    // Отправка первой новости и инструкции по навигации
    if !news_response.articles.is_empty() {
        send_single_news(bot, chat_id, &news_response.articles[0], 1, news_response.articles.len()).await?;
        
        // Отправка инструкции по навигации
        bot.send_message(chat_id, "Используйте \"следующая\" или \"предыдущая\" для навигации между новостями.")
            .await?;
    } else {
        bot.send_message(chat_id, "Новостей по вашему запросу не найдено.")
            .await?;
    }
    
    Ok(())
}

// Асинхронная функция отправки одной новости
async fn send_single_news(
    bot: &AutoSend<Bot>,
    chat_id: i64,
    article: &NewsItem,
    current_index: usize,
    total_count: usize,
) -> Result<()> {
    let message = format!("*{}*\n\n{}\n\nСтраница {} из {}", 
                         article.title, 
                         article.description,
                         current_index,
                         total_count);
    
    // Если есть изображение, отправляем его
    if let Some(image_url) = &article.url_to_image {
        bot.send_photo(chat_id, image_url)
            .caption(&message)
            .parse_mode(teloxide::types::ParseMode::Markdown)
            .await?;
    } else {
        bot.send_message(chat_id, &message)
            .parse_mode(teloxide::types::ParseMode::Markdown)
            .await?;
    }
    
    // Отправка ссылки на новость
    bot.send_message(chat_id, format!("[Читать полную новость]({})", article.url))
        .parse_mode(teloxide::types::ParseMode::Markdown)
        .disable_web_page_preview(false)
        .await?;
    
    Ok(())
}

// Структура для десериализации ответа от NewsAPI
#[derive(Serialize, Deserialize)]
struct NewsApiResponse {
    articles: Vec<NewsItem>,
}

// Вспомогательная функция для загрузки изображений в Telegraph (для отправки в Telegram)
// В целях упрощения, будем использовать прямые ссылки на изображения
async fn upload_image_to_telegraph(image_url: &str) -> Result<String> {
    // Для простоты возвращаем оригинальный URL изображения
    // В реальном приложении можно реализовать загрузку на Telegraph или другой сервис
    Ok(image_url.to_string())
}