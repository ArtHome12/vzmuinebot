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
use tokio_postgres::{NoTls};
use warp::Filter;
use reqwest::StatusCode;

mod database;
mod commands;
mod eater;
mod caterer;
mod cat_group;
mod dish;

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
         eater::start(DialogueDispatcherHandlerCx::new(bot, update, ()), true).await
      }
      cmd::Dialogue::UserMode => {
         eater::user_mode(DialogueDispatcherHandlerCx::new(bot, update, ())).await
      }
      cmd::Dialogue::CatererMode(rest_id) => {
         caterer::caterer_mode(DialogueDispatcherHandlerCx::new(bot, update, rest_id))
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
      cmd::Dialogue::CatEditRestImage(rest_id) => {
         caterer::edit_rest_image_mode(DialogueDispatcherHandlerCx::new(bot, update, rest_id))
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
      cmd::Dialogue::CatEditGroupTime(rest_id, group_id) => {
         cat_group::edit_time_mode(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, group_id)))
               .await
      }
      cmd::Dialogue::CatAddDish(rest_id, group_id) => {
         cat_group::add_dish_mode(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, group_id)))
               .await
      }
      cmd::Dialogue::CatEditDish(rest_id, group_id, dish_id) => {
         dish::edit_dish_mode(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, group_id, dish_id)))
               .await
      }
      cmd::Dialogue::CatEditDishTitle(rest_id, group_id, dish_id) => {
         dish::edit_title_mode(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, group_id, dish_id)))
               .await
      }
      cmd::Dialogue::CatEditDishInfo(rest_id, group_id, dish_id) => {
         dish::edit_info_mode(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, group_id, dish_id)))
               .await
      }
      cmd::Dialogue::CatEditDishGroup(rest_id, group_id, dish_id) => {
         dish::edit_dish_group_mode(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, group_id, dish_id)))
               .await
      }
      cmd::Dialogue::CatEditDishPrice(rest_id, group_id, dish_id) => {
         dish::edit_price_mode(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, group_id, dish_id)))
               .await
      }
      cmd::Dialogue::CatEditDishImage(rest_id, group_id, dish_id) => {
         dish::edit_image_mode(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, group_id, dish_id)))
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

    // Логин к БД
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL env variable missing");    
    // Откроем БД
    let (client, connection) =
        tokio_postgres::connect(&database_url, NoTls).await
            .expect("Cannot connect to database");

    // The connection object performs the actual communication with the database,
    // so spawn it off to run on its own.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    // Сохраним доступ к БД
    match database::DB.set(client) {
        Ok(_) => log::info!("Database connected"),
        _ => log::info!("Something wrong with database"),
    }

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


