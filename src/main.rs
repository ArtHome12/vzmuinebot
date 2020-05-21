/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
21 May 2020.
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


use parse_display::{Display, FromStr};




// ============================================================================
// [Favourite music kinds]
// ============================================================================

#[derive(Copy, Clone, Display, FromStr)]
enum FavouriteMusic {
    Rock,
    Metal,
    Pop,
    Other,
}

impl FavouriteMusic {
    fn markup() -> ReplyKeyboardMarkup {
        ReplyKeyboardMarkup::default().append_row(vec![
            KeyboardButton::new("Rock"),
            KeyboardButton::new("Metal"),
            KeyboardButton::new("Pop"),
            KeyboardButton::new("Other"),
        ])
    }
}

// ============================================================================
// [A type-safe finite automaton]
// ============================================================================

#[derive(Clone)]
struct ReceiveAgeState {
    full_name: String,
}

#[derive(Clone)]
struct ReceiveFavouriteMusicState {
    data: ReceiveAgeState,
    age: u8,
}

#[derive(Display)]
#[display(
    "Your full name: {data.data.full_name}, your age: {data.age}, your \
     favourite music: {favourite_music}"
)]
struct ExitState {
    data: ReceiveFavouriteMusicState,
    favourite_music: FavouriteMusic,
}

#[derive(SmartDefault)]
enum Dialogue {
    #[default]
    Start,
    ReceiveFullName,
    ReceiveAge(ReceiveAgeState),
    ReceiveFavouriteMusic(ReceiveFavouriteMusicState),
}

// ============================================================================
// [Control a dialogue]
// ============================================================================

type Cx<State> = DialogueDispatcherHandlerCx<Message, State>;
type Res = ResponseResult<DialogueStage<Dialogue>>;

async fn start(cx: Cx<()>) -> Res {
    cx.answer("Let's start! First, what's your full name?").send().await?;
    next(Dialogue::ReceiveFullName)
}

async fn full_name(cx: Cx<()>) -> Res {
    match cx.update.text() {
        None => {
            cx.answer("Please, send me a text message!").send().await?;
            next(Dialogue::ReceiveFullName)
        }
        Some(full_name) => {
            cx.answer("What a wonderful name! Your age?").send().await?;
            next(Dialogue::ReceiveAge(ReceiveAgeState {
                full_name: full_name.to_owned(),
            }))
        }
    }
}

async fn age(cx: Cx<ReceiveAgeState>) -> Res {
    match cx.update.text().unwrap().parse() {
        Ok(age) => {
            cx.answer("Good. Now choose your favourite music:")
                .reply_markup(FavouriteMusic::markup())
                .send()
                .await?;
            next(Dialogue::ReceiveFavouriteMusic(ReceiveFavouriteMusicState {
                data: cx.dialogue,
                age,
            }))
        }
        Err(_) => {
            cx.answer("Oh, please, enter a number!").send().await?;
            next(Dialogue::ReceiveAge(cx.dialogue))
        }
    }
}

async fn favourite_music(cx: Cx<ReceiveFavouriteMusicState>) -> Res {
    match cx.update.text().unwrap().parse() {
        Ok(favourite_music) => {
            cx.answer(format!(
                "Fine. {}",
                ExitState {
                    data: cx.dialogue.clone(),
                    favourite_music
                }
            ))
            .send()
            .await?;
            exit()
        }
        Err(_) => {
            cx.answer("Oh, please, enter from the keyboard!").send().await?;
            next(Dialogue::ReceiveFavouriteMusic(cx.dialogue))
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
        Dialogue::ReceiveFullName => {
            full_name(DialogueDispatcherHandlerCx::new(bot, update, ())).await
        }
        Dialogue::ReceiveAge(s) => {
            age(DialogueDispatcherHandlerCx::new(bot, update, s)).await
        }
        Dialogue::ReceiveFavouriteMusic(s) => {
            favourite_music(DialogueDispatcherHandlerCx::new(bot, update, s))
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


