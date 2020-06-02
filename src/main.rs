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

mod database;
mod commands;
mod eater;
mod caterer;
mod cat_group;

use commands as cmd;

// ============================================================================
// [Control a dialogue]
// ============================================================================
async fn handle_message(cx: cmd::Cx<cmd::Dialogue>) -> cmd::Res {
    let DialogueDispatcherHandlerCx { bot, update, dialogue } = cx;

    // You need handle the error instead of panicking in real-world code, maybe
    // send diagnostics to a development chat.
    match dialogue {
        cmd::Dialogue::Start => {
            eater::start(DialogueDispatcherHandlerCx::new(bot, update, ())).await
        }
        cmd::Dialogue::UserMode => {
            eater::user_mode(DialogueDispatcherHandlerCx::new(bot, update, ())).await
        }
        cmd::Dialogue::CatererMode => {
            caterer::caterer_mode(DialogueDispatcherHandlerCx::new(bot, update, ()))
                .await
        }
        cmd::Dialogue::CatEditRestTitle(rest_id) => {
            caterer::edit_rest_title_mode(DialogueDispatcherHandlerCx::new(bot, update, rest_id))
                .await
        }
        cmd::Dialogue::CatEditRestInfo(rest_id) => {
            caterer::edit_rest_info_mode(DialogueDispatcherHandlerCx::new(bot, update, rest_id))
                .await
        }
        cmd::Dialogue::CatEditGroup(rest_id, s) => {
            cat_group::edit_rest_group_mode(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, s)))
                .await
        }
        cmd::Dialogue::CatAddGroup(rest_id) => {
            caterer::add_rest_group(DialogueDispatcherHandlerCx::new(bot, update, rest_id))
                .await
        }
        cmd::Dialogue::CatEditGroupTitle(rest_id, group_id) => {
            cat_group::edit_title_mode(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, group_id)))
                .await
        }
        cmd::Dialogue::CatEditGroupInfo(rest_id, group_id) => {
            cat_group::edit_info_mode(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, group_id)))
                .await
        }
        cmd::Dialogue::CatEditGroupCategory(rest_id, group_id) => {
            cat_group::edit_category_mode(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, group_id)))
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

    // Откроем БД
    /*let restaurant = database::Restaurant{
        //id: 0,
        title: String::from("Хинкал"),
        info: String::from("Наш адрес 00NDC, доставка @nick, +84123"),
        active: true,
    };

    use std::sync::Mutex;

    if database::REST_DB.set(Mutex::new(restaurant)).is_err() {
        return;
    }*/


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


