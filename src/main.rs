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
    #[enumeration(rename = "Работает сейчас")]
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
                KeyboardButton::new("Работает сейчас"),
            ])
        .one_time_keyboard(true)
        .resize_keyboard(true)
    }
}


// ============================================================================
// [Favourite music kinds]
// ============================================================================

/*#[derive(Copy, Clone, Display, FromStr)]
enum FoodCategory {
    Breakfast, 
    Lunch, 
    Dinner, 
    Dessert,
    Other,
}

impl FoodCategory {
    fn markup() -> ReplyKeyboardMarkup {
        ReplyKeyboardMarkup::default().append_row(vec![
            KeyboardButton::new("Breakfast"),
            KeyboardButton::new("Lunch"),
            KeyboardButton::new("Dinner"),
            KeyboardButton::new("Dessert"),
            KeyboardButton::new("Other"),
        ])
        .one_time_keyboard(true)
        .resize_keyboard(true)
    }
}
*/
// ============================================================================
// [A type-safe finite automaton]
// ============================================================================

// Выбираем категорию еды, либо "Работающие сейчас" либо "Режим для ресторатора"
/*#[derive(Clone)]
struct ReceiveMainMenuState {
    main_menu_state: FoodCategory,
}*/

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


/*
#[derive(Clone)]
struct ReceivePriceState {
    food_name: String,
}

#[derive(Clone)]
struct ReceiveFoodCategoryState {
    data: ReceivePriceState,
    food_price: u8,
}

#[derive(Display)]
#[display(
    "Название блюда: {data.data.food_name}, цена: {data.food_price} тыс. донгов, категория \
     : {food_category}"
)]
*/
#[derive(Display)]
#[display(
    "В главном меню выбрано: {main_menu_state}"
)]
struct ExitState {
    main_menu_state: MainMenu,
}


#[derive(SmartDefault)]
enum Dialogue {
    #[default]
    Start,
    ReceiveMainMenu,
//    ReceiveReastaurantByCategory(ReceiveRestaurantByCategoryState),
//    ReceiveReastaurantByNow(ReceiveRestaurantByNowState),
//    ReceiveRestauratorName(ReceiveRestauratorNameState),
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
    cx.answer("Пожалуйста, выберите, какие заведения показать. Если вы ресторатор, то жмите /addOwnMenu")
        .reply_markup(MainMenu::markup())
        .send()
        .await?;
    next(Dialogue::ReceiveMainMenu)
}

async fn main_menu(cx: Cx<()>) -> Res {
    match cx.update.text().unwrap().parse() {
        Ok(main_menu_state) => {
            cx.answer(format!(
                "Отлично. {}",
                ExitState {
                    main_menu_state
                }
            ))
            //.reply_markup(main_menu_markup())
            .send()
            .await?;
            exit()
        }
        Err(_) => {
            cx.answer("Пожалуйста, выберите вариант с кнопки!").send().await?;
            next(Dialogue::ReceiveMainMenu)
        }
    }
}



/*
async fn food_name(cx: Cx<()>) -> Res {
    match cx.update.text() {
        None => {
            cx.answer("Текстовое сообщение, пожалуйста!").send().await?;
            next(Dialogue::ReceiveFoodName)
        }
        Some(food_name) => {
            cx.answer("Чудесное название! Какова цена в тыс. донгов?").send().await?;
            next(Dialogue::ReceiveFoodPrice(ReceivePriceState {
                food_name: food_name.to_owned(),
            }))
        }
    }
}

async fn food_price(cx: Cx<ReceivePriceState>) -> Res {
    match cx.update.text().unwrap().parse() {
        Ok(food_price) => {
            cx.answer("Хорошо. К какой категории оно относится:")
                .reply_markup(FoodCategory::markup())
                .send()
                .await?;
            next(Dialogue::ReceiveFoodCategory(ReceiveFoodCategoryState {
                data: cx.dialogue,
                food_price,
            }))
        }
        Err(_) => {
            cx.answer("Число, пожалуйста!").send().await?;
            next(Dialogue::ReceiveFoodPrice(cx.dialogue))
        }
    }
}

async fn food_category(cx: Cx<ReceiveFoodCategoryState>) -> Res {
    match cx.update.text().unwrap().parse() {
        Ok(food_category) => {
            cx.answer(format!(
                "Отлично. {}",
                ExitState {
                    data: cx.dialogue.clone(),
                    food_category
                }
            ))
            //.reply_markup(main_menu_markup())
            .send()
            .await?;
            exit()
        }
        Err(_) => {
            cx.answer("Пожалуйста, выберите вариант с кнопки!").send().await?;
            next(Dialogue::ReceiveFoodCategory(cx.dialogue))
        }
    }
}*/

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
 /*       Dialogue::ReceiveFoodName => {
            food_name(DialogueDispatcherHandlerCx::new(bot, update, ())).await
        }
        Dialogue::ReceiveFoodPrice(s) => {
            food_price(DialogueDispatcherHandlerCx::new(bot, update, s)).await
        }
        Dialogue::ReceiveFoodCategory(s) => {
            food_category(DialogueDispatcherHandlerCx::new(bot, update, s))
                .await
        }*/
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


