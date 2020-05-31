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
};

use std::{convert::Infallible, env, net::SocketAddr, sync::Arc};
use tokio::sync::mpsc;
use warp::Filter;
use reqwest::StatusCode;
use chrono;

mod database;
mod commands;


#[derive(SmartDefault)]
enum Dialogue {
    #[default]
    Start,
    UserMode,
    CatererMode,
    EditRestTitle,
    EditRestInfo,
    EditMainGroup,
    EditGroup(i32),
    AddGroup,
}

// ============================================================================
// [Control a dialogue]
// ============================================================================

type Cx<State> = DialogueDispatcherHandlerCx<Message, State>;
type Res = ResponseResult<DialogueStage<Dialogue>>;

async fn start(cx: Cx<()>) -> Res {
    // Отображаем приветственное сообщение и меню с кнопками.
    cx.answer("Пожалуйста, выберите, какие заведения показать в основном меню снизу. Меню можно скрыть и работать по ссылкам.")
        .reply_markup(commands::User::main_menu_markup())
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
            match commands::User::from(command) {
                commands::User::Water |
                commands::User::Food |
                commands::User::Alcohol | 
                commands::User::Entertainment => {
                    // Отобразим все рестораны, у которых есть в меню выбранная категория.
                    let rest_list = database::restaurant_by_category_from_db(command.to_string()).await;
                    cx.answer(format!("Рестораны с меню в категории {}{}", command, rest_list))
                        .send().await?;
                }
                commands::User::OpenedNow => {
                    use chrono::{Utc, FixedOffset};
                    let our_timezone = FixedOffset::east(7 * 3600);
                    let now = Utc::now().with_timezone(&our_timezone).format("%H:%M");
                    cx.answer(format!("Рестораны, открытые сейчас ({})\nКоманда в разработке", now)).send().await?;
                }
                commands::User::RestaurantMenuInCategory(cat_id, rest_id) => {
                    // Отобразим категорию меню ресторана.rest_id
                    let menu_list = database::dishes_by_restaurant_and_category_from_db(cat_id.to_string(), rest_id.to_string()).await;
                    cx.answer(format!("Меню в категории {} ресторана {}{}", cat_id, rest_id, menu_list)).send().await?;
                }
                commands::User::RestaurantOpenedCategories(rest_id) => {
                    cx.answer(format!("Доступные категории ресторана {}", rest_id)).send().await?;
                }
                commands::User::DishInfo(dish_id) => {
                    // Отобразим информацию о выбранном блюде.
                    let dish = database::dish(dish_id.to_string()).await;
                    match dish {
                        None => {
                        }
                        Some(dish_info) => {
                            cx.answer_photo(dish_info.img)
                            .caption(format!("Цена {} тыс. ₫\n{}", dish_info.price, dish_info.desc))
                            .send()
                            .await?;
                            //cx.answer(format!("Цена {} тыс. ₫\n{}", dish_info.price, dish_info.desc)).send().await?;
                        }
                    }
                }
                commands::User::CatererMode => {
                    if let Some(user) = cx.update.from() {
                        if database::is_rest_owner(user.id).await {
                            // Запрос к БД
                            let rest_info = database::rest_info(user.id).await;

                            // Отображаем информацию о ресторане и добавляем кнопки меню
                            cx.answer(format!("{}\n\nUser Id={}{}", commands::Caterer::WELCOME_MSG, user.id, rest_info))
                            .reply_markup(commands::Caterer::main_menu_markup())
                                .send()
                                .await?;
                            return next(Dialogue::CatererMode);
                        } else {
                            cx.answer(format!("Для доступа в режим рестораторов обратитесь к @vzbalmashova и сообщите ей свой Id={}", user.id))
                            .send().await?;
                        }
                    }
                }
                commands::User::Repeat => {
                    cx.answer("Команда в разработке").send().await?;
                }
                commands::User::UnknownCommand => {
                    cx.answer(format!("Неизвестная команда {}", command)).send().await?;
                }
            }
        }
    }

    // Остаёмся в пользовательском режиме.
    next(Dialogue::UserMode)
}

async fn caterer_mode(cx: Cx<()>) -> Res {
    // Код пользователя - код ресторана
    let rest_id = cx.update.from().unwrap().id;

    // Разбираем команду.
    match cx.update.text() {
        None => {
            cx.answer("Текстовое сообщение, пожалуйста!").send().await?;
            next(Dialogue::CatererMode)
        }
        Some(command) => {
            match commands::Caterer::from(command) {
                // Показать основное меню
                commands::Caterer::CatererMain => {
                    let rest_info = database::rest_info(rest_id).await;
                    cx.answer(format!("{}", rest_info))
                    .reply_markup(commands::Caterer::main_menu_markup())
                    .send()
                    .await?;

                    // Остаёмся в режиме ресторатора
                    next(Dialogue::CatererMode)
                }

                // Выйти из режима ресторатора
                commands::Caterer::CatererExit => {
                    return start(cx).await
                }

                // Изменение названия ресторана
                commands::Caterer::EditRestTitle => {
                    cx.answer(format!("Введите название (/ для отмены)"))
                    .reply_markup(commands::Caterer::slash_markup())
                    .send()
                    .await?;
                    next(Dialogue::EditRestTitle)
                }
                commands::Caterer::EditRestInfo => {
                    cx.answer(format!("Введите описание (адрес, контакты)"))
                    .reply_markup(commands::Caterer::slash_markup())
                    .send()
                    .await?;
                    next(Dialogue::EditRestInfo)
                }
                // Переключение активности ресторана
                commands::Caterer::ToggleRestPause => {
                    // Без запроса доп.данных переключаем активность
                    database::rest_toggle(rest_id).await;
                    next(Dialogue::CatererMode)
                }
                commands::Caterer::EditMainGroup => {
                    cx.answer(format!("Введите название группы"))
                    .reply_markup(commands::Caterer::slash_markup())
                    .send()
                    .await?;
                    next(Dialogue::EditMainGroup)
                }
                commands::Caterer::EditGroup(group_id) => {
                    cx.answer(format!("Введите название группы"))
                    .reply_markup(commands::Caterer::slash_markup())
                    .send()
                    .await?;
                    next(Dialogue::EditGroup(group_id))
                }
                commands::Caterer::AddGroup => {
                    cx.answer(format!("Введите название группы"))
                    .reply_markup(commands::Caterer::slash_markup())
                    .send()
                    .await?;
                    next(Dialogue::AddGroup)
                }

                commands::Caterer::UnknownCommand => {
                    cx.answer(format!("Неизвестная команда {}", command)).send().await?;
                    next(Dialogue::CatererMode)
                }
/*                _ => {
                    cx.answer(format!("В разработке")).send().await?;
                    next(Dialogue::CatererMode)
                }*/
            }
        }
    }
}

async fn edit_rest_title_mode(cx: Cx<()>) -> Res {
    if let Some(text) = cx.update.text() {
        // Удалим из строки слеши
        let s = commands::remove_slash(text).await;

        // Если строка не пустая, продолжим
        if !s.is_empty() {
            // Код пользователя - код ресторана
            let rest_id = cx.update.from().unwrap().id;
        
            database::rest_edit_title(rest_id, s).await;

            // Снова покажем главное меню
            let rest_info = database::rest_info(rest_id).await;
            cx.answer(format!("{}", rest_info))
            .reply_markup(commands::Caterer::main_menu_markup())
            .send()
            .await?;
        }
}
    next(Dialogue::CatererMode)
}

async fn edit_rest_info_mode(cx: Cx<()>) -> Res {
    if let Some(text) = cx.update.text() {
        // Удалим из строки слеши
        let s = commands::remove_slash(text).await;

        // Если строка не пустая, продолжим
        if !s.is_empty() {
            // Код пользователя - код ресторана
            let rest_id = cx.update.from().unwrap().id;
            database::rest_edit_info(rest_id, s).await;

            // Снова покажем главное меню
            let rest_info = database::rest_info(rest_id).await;
            cx.answer(format!("{}", rest_info))
            .reply_markup(commands::Caterer::main_menu_markup())
            .send()
            .await?;
        }
    }
    next(Dialogue::CatererMode)
}

async fn edit_rest_main_group_mode(cx: Cx<()>) -> Res {
    if let Some(text) = cx.update.text() {
        // Удалим из строки слеши
        let s = commands::remove_slash(text).await;

        // Если строка не пустая, продолжим
        if !s.is_empty() {
            // Код пользователя - код ресторана
            let rest_id = cx.update.from().unwrap().id;
            database::rest_edit_group(rest_id, 1, s).await;

            // Снова покажем главное меню
            let rest_info = database::rest_info(rest_id).await;
            cx.answer(format!("{}", rest_info))
            .reply_markup(commands::Caterer::main_menu_markup())
            .send()
            .await?;
        }
    }
    next(Dialogue::CatererMode)
}

async fn edit_rest_group_mode(cx: Cx<i32>) -> Res {
    if let Some(text) = cx.update.text() {
        // Удалим из строки слеши
        let s = commands::remove_slash(text).await;

        // Если строка не пустая, продолжим
        if !s.is_empty() {
            // Код пользователя - код ресторана
            let rest_id = cx.update.from().unwrap().id;
            let group_id = cx.dialogue;
            database::rest_edit_group(rest_id, group_id, s).await;

            // Снова покажем главное меню
            let rest_info = database::rest_info(rest_id).await;
            cx.answer(format!("{}", rest_info))
            .reply_markup(commands::Caterer::main_menu_markup())
            .send()
            .await?;
        }
    }
    next(Dialogue::CatererMode)
}

async fn add_rest_group(cx: Cx<()>) -> Res {
    if let Some(text) = cx.update.text() {
         // Удалим из строки слеши
         let s = commands::remove_slash(text).await;

         // Если строка не пустая, продолжим
         if !s.is_empty() {
            // Код пользователя - код ресторана
            let rest_id = cx.update.from().unwrap().id;
            database::rest_add_group(rest_id, s).await;

            // Снова покажем главное меню
            let rest_info = database::rest_info(rest_id).await;
            cx.answer(format!("{}", rest_info))
            .reply_markup(commands::Caterer::main_menu_markup())
            .send()
            .await?;
         }
    }
    next(Dialogue::CatererMode)
}


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
        Dialogue::CatererMode => {
            caterer_mode(DialogueDispatcherHandlerCx::new(bot, update, ()))
                .await
        }
        Dialogue::EditRestTitle => {
            edit_rest_title_mode(DialogueDispatcherHandlerCx::new(bot, update, ()))
                .await
        }
        Dialogue::EditRestInfo => {
            edit_rest_info_mode(DialogueDispatcherHandlerCx::new(bot, update, ()))
                .await
        }
        Dialogue::EditMainGroup => {
            edit_rest_main_group_mode(DialogueDispatcherHandlerCx::new(bot, update, ()))
                .await
        }
        Dialogue::EditGroup(s) => {
            edit_rest_group_mode(DialogueDispatcherHandlerCx::new(bot, update, s))
                .await
        }
        Dialogue::AddGroup => {
            add_rest_group(DialogueDispatcherHandlerCx::new(bot, update, ()))
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


