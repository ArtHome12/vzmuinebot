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
   types::{CallbackQuery, InlineQuery},
};

use std::{convert::Infallible, env, net::SocketAddr, sync::Arc};
use tokio::sync::mpsc;
use tokio_postgres::{NoTls};
use warp::Filter;
use reqwest::StatusCode;
use chrono::{FixedOffset};

mod database;
mod commands;
mod eater;
mod caterer;
mod cat_group;
mod dish;
mod eat_rest;
mod eat_group;
mod eat_dish;
mod eat_rest_now;
mod eat_group_now;
mod callback;
mod basket;
mod inline;
mod language;

use commands as cmd;

// ============================================================================
// [Control a dialogue]
// ============================================================================
async fn handle_message(cx: cmd::Cx<cmd::Dialogue>) -> cmd::Res {
   let DialogueDispatcherHandlerCx { bot, update, dialogue } = cx;

   // Для различения, в личку или в группу пишут
   let chat_id = update.chat_id();

   // Обрабатываем команду, если она пришла в личку
   if chat_id > 0 {
      match dialogue {
         cmd::Dialogue::Start => {
            eater::start(DialogueDispatcherHandlerCx::new(bot, update, ()), true).await
         }
         cmd::Dialogue::UserMode(compact_mode) => {
            eater::user_mode(DialogueDispatcherHandlerCx::new(bot, update, compact_mode)).await
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

         cmd::Dialogue::EatRestSelectionMode(compact_mode, cat_id) => {
            eat_rest::handle_selection_mode(DialogueDispatcherHandlerCx::new(bot, update, (compact_mode, cat_id)))
                  .await
         }
         cmd::Dialogue::EatRestGroupSelectionMode(compact_mode, cat_id, rest_id) => {
            eat_group::handle_selection_mode(DialogueDispatcherHandlerCx::new(bot, update, (compact_mode, cat_id, rest_id)))
                  .await
         }
         cmd::Dialogue::EatRestGroupDishSelectionMode(compact_mode, cat_id, rest_id, group_id) => {
            eat_dish::handle_selection_mode(DialogueDispatcherHandlerCx::new(bot, update, (compact_mode, cat_id, rest_id, group_id)))
                  .await
         }
         cmd::Dialogue::EatRestNowSelectionMode(compact_mode) => {
            eat_rest_now::handle_selection_mode(DialogueDispatcherHandlerCx::new(bot, update, compact_mode))
                  .await
         }
         cmd::Dialogue::EatRestGroupNowSelectionMode(compact_mode, rest_id) => {
            eat_group_now::handle_selection_mode(DialogueDispatcherHandlerCx::new(bot, update, (compact_mode, rest_id)))
                  .await
         }
         cmd::Dialogue::BasketMode(user_id) => {
            basket::handle_selection_mode(DialogueDispatcherHandlerCx::new(bot, update, user_id))
                  .await
         }
      } 
   } else {
      // Для сообщений не в личке обрабатываем только команду вывода id группы
      if let Some(input) = update.text() {
         match input.get(..5).unwrap_or_default() {
            "/chat" => eater::send_text(&DialogueDispatcherHandlerCx::new(bot, update, false), &format!("Chat id={}", chat_id)).await,
            _ => (),
         }
      }
      exit()
   }
}


async fn handle_callback_query(rx: DispatcherHandlerRx<CallbackQuery>) {
   rx.for_each_concurrent(None, |cx| async move {
      callback::handle_message(cx).await
   })
  .await;
}

async fn handle_inline_query(rx: DispatcherHandlerRx<InlineQuery>) {
   rx.for_each_concurrent(None, |cx| async move {
      inline::handle_message(cx).await
   })
  .await;
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
   log::info!("Starting...");


   let bot = Bot::from_env();

   // GroupId группы/чата для вывода лога
   if let Ok(log_group_id_env) = env::var("LOG_GROUP_ID") {
      if let Ok(log_group_id) = log_group_id_env.parse::<i64>() {
         // Сохраняем id и копию экземпляра бота в глобальной переменной
         let log = database::ServiceChat {
            id: log_group_id,
            bot: Arc::clone(&bot),
         };
         match database::TELEGRAM_LOG_CHAT.set(log) {
            Ok(_) => database::log_and_notify("Bot restarted").await,
            _ => log::info!("Something wrong with TELEGRAM_LOG_CHAT"),
         }
      } else {
         log::info!("Environment variable LOG_GROUP_ID must be integer")
      }
   } else {
      log::info!("There is no environment variable LOG_GROUP_ID, no service chat")
   }
   
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
         database::log(&format!("Database connection error: {}", e)).await;
      }
   });

   // Сохраним доступ к БД
   match database::DB.set(client) {
      Ok(_) => log::info!("Database connected"),
      _ => {
         log::info!("Something wrong with database");
         database::log("Something wrong with database").await;
      }
   }

   // Учётные данные админа бота - контактное имя
   let admin_name = env::var("CONTACT_INFO").expect("CONTACT_INFO env variable missing");
   match database::CONTACT_INFO.set(admin_name) {
      Ok(_) => log::info!("admin name is {}", database::CONTACT_INFO.get().unwrap()),
      _ => log::info!("Something wrong with admin name"),
   }

   // Учётные данные админа бота до трёх человек - user id
   let admin_id1: i32 = env::var("TELEGRAM_ADMIN_ID1")
      .expect("TELEGRAM_ADMIN_ID1 env variable missing")
      .parse()
      .expect("TELEGRAM_ADMIN_ID value to be integer");

   let admin_id2 = if let Ok(env_var) = env::var("TELEGRAM_ADMIN_ID2") {
      env_var.parse::<i32>().unwrap_or_default()
   } else {
      0
   };
   let admin_id3 = if let Ok(env_var) = env::var("TELEGRAM_ADMIN_ID3") {
      env_var.parse::<i32>().unwrap_or_default()
   } else {
      0
   };
   match database::TELEGRAM_ADMIN_ID.set((admin_id1, admin_id2, admin_id3)) {
      Ok(_) => log::info!("admins id is {}, {}, {}", admin_id1, admin_id2, admin_id3),
      _ => log::info!("Something wrong with admins id"),
   }

   // Единица измерения цены
   if let Ok(price_unit) = env::var("PRICE_UNIT") {
      if let Err(_) = database::PRICE_UNIT.set(price_unit) {
         let s = "Something wrong with PRICE_UNIT";
         log::info!("{}", s);
         database::log(s).await;
      }
   } else {
      let s = "There is no environment variable PRICE_UNIT";
      log::info!("{}", s);
      database::log(s).await;
   }

   // Часовой пояс
   if let Ok(time_zone_str) = env::var("TIME_ZONE") {
      // Зона как число
      let time_zone_num = time_zone_str.parse::<i32>().unwrap_or_default();

      // Создадим нужный объект
      let time_zone = FixedOffset::east(time_zone_num * 3600);

      if let Err(_) = database::TIME_ZONE.set(time_zone) {
         let s = "Something wrong with TIME_ZONE";
         log::info!("{}", s);
         database::log(s).await;
      }
   } else {
      let s = "There is no environment variable TIME_ZONE";
      log::info!("{}", s);
      database::log(s).await;
   }


   // Проверим существование таблиц и если их нет, создадим
   //
   if database::is_tables_exist().await {
      log::info!("Table restaurants exist, open existing data");
   } else {
      log::info!("Table restaurants do not exist, create new tables: {}", database::is_success(database::create_tables().await));
   }
   
   Dispatcher::new(Arc::clone(&bot))
   .messages_handler(DialogueDispatcher::new(|cx| async move {
      handle_message(cx).await.expect("Something wrong with the bot!")
   }))
   .callback_queries_handler(handle_callback_query)
   .inline_queries_handler(handle_inline_query)
   .dispatch_with_listener(
      webhook(bot).await,
      LoggingErrorHandler::with_custom_text("An error from the update listener"),
   )
   .await;
}


