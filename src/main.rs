use teloxide::{
    prelude::*,
    utils::command::BotCommands,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, Message, ParseMode},
};
use serde::{Deserialize, Serialize};
use reqwest;
use dotenv::dotenv;
use std::env;

// Структура для хранения информации о пользователе
#[derive(Debug)]
struct UserState {
    topic: String,
    news: Vec<Article>,
    current_index: usize,
}

// Глобальное хранилище состояний пользователей
use std::collections::HashMap;
use once_cell::sync::Lazy;
use tokio::sync::Mutex;

static USER_STATES: Lazy<Mutex<HashMap<i64, UserState>>> = Lazy::new(|| Mutex::new(HashMap::new()));

// Определение команд бота
#[derive(BotCommands, Clone)]
#[command(rename = "lowercase", description = "Доступные команды:")]
enum Command {
    #[command(description = "начать работу с ботом")]
    Start,
    #[command(description = "получить справку")]
    Help,
}

// Структура для хранения информации о статье
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Article {
    title: String,
    description: String,
    url: String,
    url_to_image: Option<String>,
}

// Структура для ответа от NewsAPI
#[derive(Serialize, Deserialize, Debug)]
struct NewsResponse {
    articles: Vec<Article>,
}

// Асинхронная функция для получения новостей из NewsAPI
async fn get_news(topic: &str) -> Result<Vec<Article>, Box<dyn std::error::Error + Send + Sync>> {
    // Получаем API-ключ из переменной окружения
    let api_key = env::var("NEWS_API_KEY").map_err(|_| "Не найден NEWS_API_KEY в .env файле")?;
    
    // Формируем URL для запроса
    let url = format!(
        "https://newsapi.org/v2/everything?q={}&sortBy=publishedAt&apiKey={}",
        topic, api_key
    );

    // Выполняем HTTP-запрос
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    
    // Проверяем статус ответа
    if response.status().is_success() {
        // Десериализуем JSON-ответ
        let news_response: NewsResponse = response.json().await?;
        Ok(news_response.articles)
    } else {
        // Возвращаем ошибку при неуспешном ответе
        Err(format!("Ошибка API: {}", response.status()).into())
    }
}

// Асинхронная функция для отправки карточки новости пользователю
async fn send_news_card(
    bot: &Bot,
    msg: &Message,
    article: &Article,
    current_index: usize,
    total_count: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Создаем клавиатуру с кнопками навигации
    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("Предыдущая", "previous"),
        InlineKeyboardButton::callback("Следующая", "next"),
    ]]);

    // Формируем текст сообщения с заголовком и описанием
    let text = format!(
        "*{}*\n\n{}\n\nИсточник: [Ссылка на новость]({})\n\n({}/{})",
        article.title,
        article.description.as_ref().unwrap_or(&"Без описания".to_string()),
        article.url,
        current_index + 1,
        total_count
    );

    // Отправляем сообщение с изображением, если оно доступно
    if let Some(image_url) = &article.url_to_image {
        bot.send_photo(msg.chat.id, image_url)
            .caption(text)
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(keyboard)
            .await?;
    } else {
        // Если изображение недоступно, отправляем только текст
        bot.send_message(msg.chat.id, text)
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(keyboard)
            .await?;
    }

    Ok(())
}

// Основная асинхронная функция обработки сообщений
async fn handler(cx: UpdateWithCx<Message>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Получаем текст сообщения от пользователя
    let text = cx.update.text().unwrap_or_default().trim().to_lowercase();

    // Обрабатываем команды
    if text.starts_with('/') {
        match Command::parse(&text, "newsbot") {
            Ok(Command::Start) => {
                // Отправляем приветственное сообщение
                cx.bot
                    .send_message(cx.update.chat.id, "Здравствуйте, какие новости вы хотите узнать?")
                    .await?;
            }
            Ok(Command::Help) => {
                // Отправляем справочное сообщение
                cx.bot
                    .send_message(
                        cx.update.chat.id,
                        "Бот для получения новостей по теме.\n\nДоступные команды:\n/start - начать работу\n/help - справка\n\nВведите тему для поиска новостей.",
                    )
                    .await?;
            }
            Err(_) => {
                // Отправляем сообщение об ошибке при неправильной команде
                cx.bot
                    .send_message(cx.update.chat.id, "Неизвестная команда. Используйте /help для получения списка команд.")
                    .await?;
            }
        }
    } else {
        // Если пользователь ввел тему (не команду)
        // Получаем API-ключ из переменной окружения
        let api_key = env::var("NEWS_API_KEY").unwrap_or_default();
        
        if api_key.is_empty() {
            // Уведомляем пользователя, если не установлен API-ключ
            cx.bot
                .send_message(
                    cx.update.chat.id,
                    "Для работы бота необходимо установить NEWS_API_KEY в .env файле.",
                )
                .await?;
            return Ok(());
        }

        // Отправляем сообщение о поиске новостей
        cx.bot
            .send_message(cx.update.chat.id, "Поиск новостей...")
            .await?;

        // Получаем новости по указанной теме
        match get_news(&text).await {
            Ok(articles) => {
                if !articles.is_empty() {
                    // Сохраняем состояние пользователя (тему, новости, индекс)
                    {
                        let mut states = USER_STATES.lock().await;
                        states.insert(
                            cx.update.chat.id.0,
                            UserState {
                                topic: text.clone(),
                                news: articles,
                                current_index: 0,
                            },
                        );
                    }

                    // Отправляем первую новость
                    let user_state = USER_STATES.lock().await.get(&cx.update.chat.id.0).unwrap().clone();
                    send_news_card(
                        &cx.bot,
                        &cx.update,
                        &user_state.news[0],
                        user_state.current_index,
                        user_state.news.len(),
                    )
                    .await?;
                } else {
                    // Уведомляем пользователя, если новости не найдены
                    cx.bot
                        .send_message(cx.update.chat.id, "Новостей по вашему запросу не найдено.")
                        .await?;
                }
            }
            Err(e) => {
                // Уведомляем пользователя об ошибке при получении новостей
                eprintln!("Ошибка при получении новостей: {}", e);
                cx.bot
                    .send_message(cx.update.chat.id, "Ошибка при получении новостей. Попробуйте позже.")
                    .await?;
            }
        }
    }

    Ok(())
}

// Асинхронная функция обработки callback-запросов (навигация по новостям)
async fn callback_handler(cx: UpdateWithCx<teloxide::types::CallbackQuery>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Получаем ID пользователя
    let user_id = cx.update.message.as_ref().map(|msg| msg.chat.id.0).unwrap_or_else(|| cx.update.from().map(|user| user.id.0).unwrap_or(0));
    
    // Получаем действие из callback-запроса
    let action = &cx.update.data.as_ref().unwrap_or(&String::new()).to_lowercase();

    // Получаем состояние пользователя
    let mut states = USER_STATES.lock().await;
    if let Some(user_state) = states.get_mut(&user_id) {
        match action.as_str() {
            "next" => {
                // Переход к следующей новости
                if user_state.current_index + 1 < user_state.news.len() {
                    user_state.current_index += 1;
                } else {
                    // Если достигли конца, возвращаемся к первой новости
                    user_state.current_index = 0;
                }
            }
            "previous" => {
                // Переход к предыдущей новости
                if user_state.current_index > 0 {
                    user_state.current_index -= 1;
                } else {
                    // Если на первой новости, переходим к последней
                    user_state.current_index = user_state.news.len().saturating_sub(1);
                }
            }
            _ => {}
        }

        // Отправляем текущую новость пользователю
        let current_article = &user_state.news[user_state.current_index];
        if let Some(message) = &cx.update.message {
            send_news_card(
                &cx.bot,
                message,
                current_article,
                user_state.current_index,
                user_state.news.len(),
            )
            .await?;
        }
    }

    // Отвечаем на callback-запрос
    cx.bot.answer_callback_query(&cx.update.id).await?;
    Ok(())
}

// Асинхронная основная функция запуска бота
#[tokio::main]
async fn main() {
    // Загружаем переменные окружения из файла .env
    dotenv().ok();
    
    // Включаем логирование
    teloxide::enable_logging!();
    
    // Получаем токен бота из переменной окружения
    let bot = Bot::from_env();

    log::info!("Бот запущен!");

    // Запускаем бота с обработчиками сообщений и callback-запросов
    teloxide::repl(bot, |update| async move {
        match update {
            teloxide::UpdateKind::Message(msg) => {
                handler(UpdateWithCx::new(msg.bot.clone(), msg)).await
            }
            teloxide::UpdateKind::CallbackQuery(callback) => {
                callback_handler(UpdateWithCx::new(callback.bot.clone(), callback)).await
            }
            _ => Ok(()),
        }
    }).await;
}