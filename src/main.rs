/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Главный модуль. 21 May 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

#![allow(clippy::trivial_regex)]

#[macro_use]
extern crate smart_default;

use teloxide::{
    dispatching::update_listeners, 
    prelude::*, 
    types::{KeyboardButton, ReplyKeyboardMarkup},
};

use std::{convert::Infallible, env, net::SocketAddr, sync::Arc};
use tokio::sync::mpsc;
use warp::Filter;
use reqwest::StatusCode;
use chrono;

mod database;

// ============================================================================
// [Main menu]
// ============================================================================
#[derive(Copy, Clone)]
enum Commands {
    // Команды главного меню
    Breakfast, 
    Lunch, 
    Dinner, 
    Dessert,
    OpenedNow,
    Repeat,
    RestoratorMode,
    UnknownCommand,
    // Показать список блюд в указанной категории ресторана /rest#___ cat_id, rest_id, 
    RestaurantMenuInCategory(u32, u32),
    // Показать информацию о блюде /dish___ dish_id
    DishInfo(u32),
    // Показать список доступных сейчас категорий меню ресторана /menu___ rest_id
    RestaurantOpenedCategories(u32),
}

impl Commands {
    fn from(input: &str) -> Commands {
        match input {
            // Сначала проверим на цельные команды.
            "Завтрак" => Commands::Breakfast,
            "Обед" => Commands::Lunch,
            "Ужин" => Commands::Dinner,
            "Кофе" => Commands::Dessert,
            "Работают сейчас" => Commands::OpenedNow,
            "Повтор" => Commands::Repeat,
            "Добавить меню" => Commands::RestoratorMode,
            _ => {
                // Ищем среди команд с цифровыми суффиксами, если строка достаточной длины.
                // Сначала Извлекаем возможное тело команды, потом разбираем команду и аргументы.
                match input.get(..5).unwrap_or_default() {
                    "/rest" => {
                        // Длина строки должна быть достаточной для двух аргументов.
/*                        if len < 6 {
                            return Commands::UnknownCommand;
                        }*/

                        // Извлекаем аргументы (сначала подстроку, потом число).
                        let arg1 = input.get(5..6).unwrap_or_default().parse().unwrap_or_default();
                        let arg2 = input.get(6..).unwrap_or_default().parse().unwrap_or_default();

                        // Возвращаем команду.
                        Commands::RestaurantMenuInCategory(arg1, arg2)
                    }
                    "/dish" => Commands::DishInfo(input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
                    "/menu" => Commands::RestaurantOpenedCategories(input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
                    _ => Commands::UnknownCommand,
                }
            }
        }
    }

    fn main_menu_markup() -> ReplyKeyboardMarkup {
        ReplyKeyboardMarkup::default()
            .append_row(vec![
                KeyboardButton::new("Завтрак"),
                KeyboardButton::new("Обед"),
                KeyboardButton::new("Ужин"),
                KeyboardButton::new("Кофе"),
            ])
            .append_row(vec![
                KeyboardButton::new("Работают сейчас"),
                KeyboardButton::new("Повтор"),
                KeyboardButton::new("Добавить"),
            ])
            .resize_keyboard(true)
    }
}

// ============================================================================
// [A type-safe finite automaton]
// ============================================================================
/*
// Если выбрана категория еды, то надо показать список ресторанов, в которых есть
// еда данной категории
#[derive(Clone)]
struct ReceiveRestaurantByCategoryState {
    main_menu_state: MainMenu,
}

// Если выбрана категория "Работающие сейчас", то надо показать список ресторанов,
// работающих в текущее время.
#[derive(Clone)]
struct ReceiveRestaurantByNowState {
    main_menu_state: MainMenu,
}

// Если выбрана категория "Режим для рестораторов", то переходим в него.
#[derive(Clone)]
struct ReceiveRestauratorNameState {
    main_menu_state: MainMenu,
}


#[derive(Display)]
#[display(
    "В главном меню выбрано: {main_menu_state}, {some_text}"
)]
struct ExitState {
    main_menu_state: MainMenu,
    some_text: String,
}
*/

#[derive(SmartDefault)]
enum Dialogue {
    #[default]
    Start,
    UserMode,
    RestaurateurMode,
}

// ============================================================================
// [Control a dialogue]
// ============================================================================

type Cx<State> = DialogueDispatcherHandlerCx<Message, State>;
type Res = ResponseResult<DialogueStage<Dialogue>>;

async fn start(cx: Cx<()>) -> Res {
    // Отображаем приветственное сообщение и меню с кнопками.
    cx.answer("Пожалуйста, выберите, какие заведения показать в основном меню снизу. Меню можно скрыть и работать по ссылкам.")
        .reply_markup(Commands::main_menu_markup())
        .send()
        .await?;
    
    // Переходим в режим получения выбранного пункта в главном меню.
    next(Dialogue::UserMode)
}

async fn user_mode(cx: Cx<()>) -> Res {
    // Разбираем команду.
    match cx.update.text() {
        None => {
            cx.answer("Текстовое сообщение, пожалуйста!").send().await?;
        }
        Some(command) => {
            match Commands::from(command) {
                Commands::Breakfast |
                Commands::Lunch |
                Commands::Dinner | 
                Commands::Dessert => {
                    // Отобразим все рестораны, у которых есть в меню выбранная категория.
                    let rest_list = database::restaurant_by_category_from_db(command.to_string()).await;
                    cx.answer(format!("Рестораны с меню в категории {}{}", command, rest_list))
                        .send().await?;
                }
                Commands::OpenedNow => {
                    use chrono::{Utc};
                    let now = Utc::now().format("%H:%M");
                    cx.answer(format!("Рестораны, открытые сейчас ({})", now)).send().await?;
                }
                Commands::RestaurantMenuInCategory(cat_id, rest_id) => {
                    // Отобразим категорию меню ресторана.rest_id
                    let menu_list = database::dishes_by_restaurant_and_category_from_db(cat_id.to_string(), rest_id.to_string()).await;
                    cx.answer(format!("Меню в категории {} ресторана {}{}", cat_id, rest_id, menu_list)).send().await?;
                }
                Commands::RestaurantOpenedCategories(rest_id) => {
                    cx.answer(format!("Доступные категории ресторана {}", rest_id)).send().await?;
                }
                Commands::DishInfo(dish_id) => {
                    // Отобразим информацию о выбранном блюде.
                    let dish = database::dish(dish_id.to_string()).await;
                    match dish {
                        None => {
                        }
                        Some(dish_info) => {
                            cx.answer_photo(dish_info.img).send().await?;
                            cx.answer(format!("Цена {} тыс. ₫\n{}", dish_info.price, dish_info.desc)).send().await?;
                        }
                    }
                }
                Commands::RestoratorMode => {
                    cx.answer("Переходим в режим для владельцев ресторанов").send().await?;
                    return next(Dialogue::RestaurateurMode);
                }
                Commands::Repeat => {
                    cx.answer("Повтор").send().await?;
                    return next(Dialogue::RestaurateurMode);
                }
                Commands::UnknownCommand => {
                    cx.answer(format!("Неизвестная команда {}", command)).send().await?;
                }
            }
        }
    }

    // Остаёмся в пользовательском режиме.
    next(Dialogue::UserMode)
}

async fn restorator_mode(cx: Cx<()>) -> Res {
    cx.answer(format!("Для доступа в режим рестораторов обратитесь к @vzbalmashova")).send().await?;
    next(Dialogue::Start)
}

/*
async fn main_menu(cx: Cx<()>) -> Res {
    // Преобразовываем выбор пользователя в элемент MainMenu.
    match cx.update.text().unwrap().parse::<MainMenu>() {
        Ok(main_menu_state) => {
            match main_menu_state {
                // Если пользователь выбрал вариант с указанием категории меню (завтрак, обед и т.д.).
                MainMenu::Breakfast | 
                MainMenu::Lunch | 
                MainMenu::Dinner |
                MainMenu::Dessert => {
                    // Отобразим все рестораны, у которых есть в меню выбранная категория.
                    let rest_list = database::restaurant_by_category_from_db(main_menu_state.to_string()).await;
                    cx.answer(format!("Список ресторанов с блюдами категории {}{}\n\
                    Возврат в главное меню /main", main_menu_state, rest_list)).send().await?;

                    next(Dialogue::ReceiveRestaurantByCategory(ReceiveRestaurantByCategoryState {
                        main_menu_state : main_menu_state.to_owned(),
                    }))
                }

                // Если пользователь хочет увидеть рестораны, работающие сейчас.
                MainMenu::OpenedNow => {
                    let rest_list = String::from(restaurant_opened_now_from_db().await);
                    cx.answer(format!("Список ресторанов, работающих сейчас{}\n\
                    Возврат в главное меню /main", rest_list)).send().await?;

                    next(Dialogue::ReceiveRestaurantByNow(ReceiveRestaurantByNowState {
                        main_menu_state : main_menu_state.to_owned(),
                    }))
                }

                // Если пользователь хочет перейти в режим управления меню.
                MainMenu::RestoratorMode => {
                    cx.answer(format!("Для доступа в режим рестораторов обратитесь к @vzbalmashova")).send().await?;
                    next(Dialogue::RestoratorMode)
                }
            }
        }
        Err(_) => {
            cx.answer("Пожалуйста, выберите вариант с кнопки!").send().await?;
            next(Dialogue::ReceiveMainMenu)
        }
    }
}

async fn restaurant_by_category(cx: Cx<ReceiveRestaurantByCategoryState>) -> Res {
    match cx.update.text() {
        None => {
            cx.answer("Ссылку на ресторан или /main, пожалуйста!").send().await?;
            next(Dialogue::ReceiveRestaurantByCategory(cx.dialogue))
        }
        Some(rest_name) => {
            match rest_name {
                "/main" => {
                    start(cx.with_new_dialogue(())).await
                }
                _ => {
                    // Отобразим меню выбранного ресторана
                    let dishes_list = database::dishes_by_restaurant_and_category_from_db(rest_name.to_string(), String::default()).await;
                    cx.answer(format!("Меню ресторана с блюдами выбранной категории{}\n\
                    Возврат в главное меню /main", dishes_list)).send().await?;
                    next(Dialogue::ReceiveDish)
                }
            }
        }
    }
}

async fn restaurant_by_now(cx: Cx<ReceiveRestaurantByNowState>) -> Res {
    match cx.update.text() {
        None => {
            cx.answer("Название категории, пожалуйста!").send().await?;
            next(Dialogue::ReceiveRestaurantByNow(cx.dialogue))
        }
        Some(full_name) => {
            cx.answer(format!(
                "Отлично. {}",
                ExitState {
                    main_menu_state: cx.dialogue.main_menu_state.clone(),
                    some_text: full_name.to_string(),
                }
            ))
            .send()
            .await?;
            exit()
        }
    }
}

async fn show_dish(cx: Cx<()>) -> Res {
    // Получаем информацию о блюде.
    let dish = database::dish(String::default()).await;
    
    // Отправляем информацию о блюде.
    match dish {
        None => {
        }
        Some(dish_info) => {
            cx.answer_photo(dish_info.img).send().await?;
            cx.answer(format!("Цена {} тыс. ₫\n{}", dish_info.price, dish_info.desc)).send().await?;
        }
    }

    // Переходим в главное меню.
    start(cx.with_new_dialogue(())).await
}*/

async fn handle_message(cx: Cx<Dialogue>) -> Res {
    let DialogueDispatcherHandlerCx { bot, update, dialogue } = cx;

    // You need handle the error instead of panicking in real-world code, maybe
    // send diagnostics to a development chat.
    match dialogue {
        Dialogue::Start => {
            start(DialogueDispatcherHandlerCx::new(bot, update, ())).await
        }
        Dialogue::UserMode => {
            user_mode(DialogueDispatcherHandlerCx::new(bot, update, ())).await
        }
/*        Dialogue::ReceiveMainMenu => {
            main_menu(DialogueDispatcherHandlerCx::new(bot, update, ()))
                .await
        }
        Dialogue::ReceiveRestaurantByCategory(s) => {
            restaurant_by_category(DialogueDispatcherHandlerCx::new(bot, update, s))
                .await
        }
        Dialogue::ReceiveRestaurantByNow(s) => {
            restaurant_by_now(DialogueDispatcherHandlerCx::new(bot, update, s))
                .await
        }
        Dialogue::ReceiveDish => {
            show_dish(DialogueDispatcherHandlerCx::new(bot, update, ()))
                .await
        }*/
        Dialogue::RestaurateurMode => {
            restorator_mode(DialogueDispatcherHandlerCx::new(bot, update, ()))
                .await
        }
    }
}


// ============================================================================
// [Run!]
// ============================================================================
#[tokio::main]
async fn main() {
    run().await;
}

async fn handle_rejection(error: warp::Rejection) -> Result<impl warp::Reply, Infallible> {
    log::error!("Cannot process the request due to: {:?}", error);
    Ok(StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn webhook<'a>(bot: Arc<Bot>) -> impl update_listeners::UpdateListener<Infallible> {
    // Heroku defines auto defines a port value
    let teloxide_token = env::var("TELOXIDE_TOKEN").expect("TELOXIDE_TOKEN env variable missing");
    let port: u16 = env::var("PORT")
        .expect("PORT env variable missing")
        .parse()
        .expect("PORT value to be integer");
    // Heroku host example .: "heroku-ping-pong-bot.herokuapp.com"
    let host = env::var("HOST").expect("have HOST env variable");
    let path = format!("bot{}", teloxide_token);
    let url = format!("https://{}/{}", host, path);

    bot.set_webhook(url)
        .send()
        .await
        .expect("Cannot setup a webhook");
    
    let (tx, rx) = mpsc::unbounded_channel();

    let server = warp::post()
        .and(warp::path(path))
        .and(warp::body::json())
        .map(move |json: serde_json::Value| {
            let try_parse = match serde_json::from_str(&json.to_string()) {
                Ok(update) => Ok(update),
                Err(error) => {
                    log::error!(
                        "Cannot parse an update.\nError: {:?}\nValue: {}\n\
                       This is a bug in teloxide, please open an issue here: \
                       https://github.com/teloxide/teloxide/issues.",
                        error,
                        json
                    );
                    Err(error)
                }
            };
            if let Ok(update) = try_parse {
                tx.send(Ok(update))
                    .expect("Cannot send an incoming update from the webhook")
            }

            StatusCode::OK
        })
        .recover(handle_rejection);

    let serve = warp::serve(server);

    let address = format!("0.0.0.0:{}", port);
    tokio::spawn(serve.run(address.parse::<SocketAddr>().unwrap()));
    rx
}

async fn run() {
    teloxide::enable_logging!();
    log::info!("Starting vzmuinebot!");

    log::info!("Database connected");
    
    let bot = Bot::from_env();

    Dispatcher::new(Arc::clone(&bot))
        .messages_handler(DialogueDispatcher::new(|cx| async move {
            handle_message(cx).await.expect("Something wrong with the bot!")
        }))
        .dispatch_with_listener(
            webhook(bot).await,
            LoggingErrorHandler::with_custom_text("An error from the update listener"),
        )
        .await;
}


