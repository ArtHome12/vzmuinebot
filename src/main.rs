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
use enum_utils;

use parse_display::{Display};


// ============================================================================
// [Main menu]
// ============================================================================
#[derive(Copy, Clone, Display, enum_utils::FromStr)]
enum MainMenu {
    #[enumeration(rename = "Завтрак")]
    Breakfast, 
    #[enumeration(rename = "Обед")]
    Lunch, 
    #[enumeration(rename = "Ужин")]
    Dinner, 
    #[enumeration(rename = "Кофе/десерты")]
    Dessert,
    #[enumeration(rename = "Работают сейчас")]
    OpenedNow,
    #[enumeration(rename = "/addOwnMenu")]
    RestoratorMode,
}

impl MainMenu {
    fn markup() -> ReplyKeyboardMarkup {
        ReplyKeyboardMarkup::default()
            .append_row(vec![
                KeyboardButton::new("Завтрак"),
                KeyboardButton::new("Обед"),
                KeyboardButton::new("Ужин"),
            ])
            .append_row(vec![
                KeyboardButton::new("Кофе/десерты"),
                KeyboardButton::new("Работают сейчас"),
            ])
            .one_time_keyboard(true)
            .resize_keyboard(true)
    }

/*    fn exit_markup() -> ReplyKeyboardMarkup {
        ReplyKeyboardMarkup::default()
            .append_row(vec![KeyboardButton::new("Продолжить")])
            .one_time_keyboard(true)
            .resize_keyboard(true)
    }*/
}


// ============================================================================
// [A type-safe finite automaton]
// ============================================================================

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


#[derive(SmartDefault)]
enum Dialogue {
    #[default]
    Start,
    ReceiveMainMenu,
    ReceiveReastaurantByCategory(ReceiveRestaurantByCategoryState),
    ReceiveReastaurantByNow(ReceiveRestaurantByNowState),
    RestoratorMode,
    /*ReceiveFoodName,
    ReceiveFoodPrice(ReceivePriceState),
    ReceiveFoodCategory(ReceiveFoodCategoryState),*/
}

// ============================================================================
// [Control a dialogue]
// ============================================================================

type Cx<State> = DialogueDispatcherHandlerCx<Message, State>;
type Res = ResponseResult<DialogueStage<Dialogue>>;

async fn start(cx: Cx<()>) -> Res {
    // Отображаем приветственное сообщение и меню с кнопками.
    cx.answer("Пожалуйста, выберите, какие заведения показать. Если вы ресторатор, то жмите /addOwnMenu")
        .reply_markup(MainMenu::markup())
        .send()
        .await?;
    
    // Переходим в режим получения выбранного пункта в главном меню.
    next(Dialogue::ReceiveMainMenu)
}

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
                    let rest_list = String::from(restaurant_by_category_from_db().await);
                    cx.answer(format!("Список ресторанов с блюдами выбранной категории{}\n\
                    Возврат в главное меню /main", rest_list)).send().await?;

                    next(Dialogue::ReceiveReastaurantByCategory(ReceiveRestaurantByCategoryState {
                        main_menu_state : main_menu_state.to_owned(),
                    }))
                }

                // Если пользователь хочет увидеть рестораны, работающие сейчас.
                MainMenu::OpenedNow => {
                    let rest_list = String::from(restaurant_opened_now_from_db().await);
                    cx.answer(format!("Список ресторанов, работающих сейчас{}\n\
                    Возврат в главное меню /main", rest_list)).send().await?;

                    next(Dialogue::ReceiveReastaurantByNow(ReceiveRestaurantByNowState {
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
            next(Dialogue::ReceiveReastaurantByCategory(cx.dialogue))
        }
        Some(rest_name) => {
            match rest_name {
                "/main" => next(Dialogue::Start)
/*                "/main" => {
                    // Отобразим кнопку для возврата.
                    cx.answer("В начало")
                        .reply_markup(MainMenu::exit_markup())
                        .send()
                        .await?;
                     exit()
                }*/
                _ => {
                    // Отобразим меню выбранного ресторана
                    let dishes_list = String::from(dishes_by_restaurant_and_category_from_db().await);
                    cx.answer(format!("Меню ресторана с блюдами выбранной категории{}\n\
                    Возврат в главное меню /start", dishes_list)).send().await?;
                    //next(Dialogue::ReceiveMainMenu)
                    exit()
                }
            }
        }
    }
}

async fn restaurant_by_now(cx: Cx<ReceiveRestaurantByNowState>) -> Res {
    match cx.update.text() {
        None => {
            cx.answer("Название категории, пожалуйста!").send().await?;
            next(Dialogue::ReceiveReastaurantByNow(cx.dialogue))
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

async fn handle_message(cx: Cx<Dialogue>) -> Res {
    let DialogueDispatcherHandlerCx { bot, update, dialogue } = cx;

    // You need handle the error instead of panicking in real-world code, maybe
    // send diagnostics to a development chat.
    match dialogue {
        Dialogue::Start => {
            start(DialogueDispatcherHandlerCx::new(bot, update, ())).await
        }
        Dialogue::ReceiveMainMenu => {
            main_menu(DialogueDispatcherHandlerCx::new(bot, update, ()))
                .await
        }
        Dialogue::ReceiveReastaurantByCategory(s) => {
            restaurant_by_category(DialogueDispatcherHandlerCx::new(bot, update, s))
                .await
        }
        Dialogue::ReceiveReastaurantByNow(s) => {
            restaurant_by_now(DialogueDispatcherHandlerCx::new(bot, update, s))
                .await
        }
        Dialogue::RestoratorMode => {
            main_menu(DialogueDispatcherHandlerCx::new(bot, update, ()))
                .await
        }
    }
}

// ============================================================================
// [Database routines!]
// ============================================================================
async fn restaurant_by_category_from_db() -> String {
    String::from("
        Ёлки-палки /rest01
        Крошка-картошка /rest02
        Плакучая ива /rest03
        Националь /rest04
        Хинкал /rest05"
    )
}

async fn restaurant_opened_now_from_db() -> String {
    String::from("
        Ёлки-палки /rest01
        Крошка-картошка /rest02"
    )
}

async fn dishes_by_restaurant_and_category_from_db() -> String {
    String::from("
        Борщ /rest0101
        Картофельное пюре /rest0102
        Мясо по-французски /rest0103
        Шарлотка /rest0104
        Чай /rest0105"
    )
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


