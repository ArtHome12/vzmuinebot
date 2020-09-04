/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Главный модуль. 21 May 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

//#![type_length_limit="46255638"]
#![allow(clippy::trivial_regex)]

#[macro_use]
extern crate smart_default;

use teloxide::{
   dispatching::update_listeners, 
   prelude::*, 
   types::{CallbackQuery, InlineQuery, ChatId, },
};

use std::{convert::Infallible, env, net::SocketAddr, sync::Arc};
use tokio::sync::mpsc;
use tokio_postgres::{NoTls};
use warp::Filter;
use reqwest::StatusCode;
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};

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
mod settings;
mod gear;

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
         cmd::Dialogue::UserMode => {
            eater::handle_commands(DialogueDispatcherHandlerCx::new(bot, update, ())).await
         }
         cmd::Dialogue::UserModeEditCatImage(image_id) => {
            eater::edit_cat_image(DialogueDispatcherHandlerCx::new(bot, update, image_id)).await
         }
         cmd::Dialogue::CatererMode(rest_id) => {
            caterer::handle_commands(DialogueDispatcherHandlerCx::new(bot, update, rest_id))
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
            cat_group::handle_commands(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, s)))
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
            dish::handle_commands(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, group_id, dish_id)))
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

         cmd::Dialogue::EatRestSelectionMode(cat_id) => {
            eat_rest::handle_commands(DialogueDispatcherHandlerCx::new(bot, update, cat_id))
                  .await
         }
         cmd::Dialogue::EatRestGroupSelectionMode(cat_id, rest_id) => {
            eat_group::handle_commands(DialogueDispatcherHandlerCx::new(bot, update, (cat_id, rest_id)))
                  .await
         }
         cmd::Dialogue::EatRestGroupDishSelectionMode(cat_id, rest_id, group_id) => {
            eat_dish::handle_commands(DialogueDispatcherHandlerCx::new(bot, update, (cat_id, rest_id, group_id)))
                  .await
         }
         cmd::Dialogue::EatRestNowSelectionMode => {
            eat_rest_now::handle_commands(DialogueDispatcherHandlerCx::new(bot, update, ()))
                  .await
         }
         cmd::Dialogue::EatRestGroupNowSelectionMode(rest_id) => {
            eat_group_now::handle_commands(DialogueDispatcherHandlerCx::new(bot, update, rest_id))
                  .await
         }
         cmd::Dialogue::BasketMode(user_id) => {
            basket::handle_commands(DialogueDispatcherHandlerCx::new(bot, update, user_id))
                  .await
         }
         cmd::Dialogue::BasketEditName(user_id) => {
            basket::edit_name_mode(DialogueDispatcherHandlerCx::new(bot, update, user_id))
                  .await
         }
         cmd::Dialogue::BasketEditContact(user_id) => {
            basket::edit_contact_mode(DialogueDispatcherHandlerCx::new(bot, update, user_id))
                  .await
         }
         cmd::Dialogue::BasketEditAddress(user_id) => {
            basket::edit_address_mode(DialogueDispatcherHandlerCx::new(bot, update, user_id))
                  .await
         }
         cmd::Dialogue::MessageToCaterer(user_id, caterer_id, origin) => {
            edit_message_to_caterer_mode(DialogueDispatcherHandlerCx::new(bot, update, (user_id, caterer_id, origin)))
                  .await
         }
         cmd::Dialogue::GearMode => {
            gear::handle_commands(DialogueDispatcherHandlerCx::new(bot, update, ()))
                  .await
         }
      } 
} else {
      // Для сообщений не в личке обрабатываем только команду вывода id группы
      if let Some(input) = update.text() {
         match input.get(..5).unwrap_or_default() {
            "/chat" => cmd::send_text_without_markup(&DialogueDispatcherHandlerCx::new(bot, update, ()), &format!("Chat id={}", chat_id)).await,
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

   // Настройки из переменных окружения
   let vars = settings::Vars::from_env(&bot).await;
   match settings::VARS.set(vars) {
      Ok(_) => settings::log_and_notify("Bot restarted").await,
      _ => log::info!("Something wrong with TELEGRAM_LOG_CHAT"),
   }

   // Откроем БД
   let database_url = env::var("DATABASE_URL").expect("DATABASE_URL env variable missing");
   let pg_config = database_url.parse::<tokio_postgres::Config>().expect("DATABASE_URL env variable wrong");
   let mgr_config = ManagerConfig {recycling_method: RecyclingMethod::Fast};
   let mgr = Manager::from_config(pg_config, NoTls, mgr_config);
   let pool = Pool::new(mgr, 16);

   // Протестируем соединение
   let test_pool = pool.clone();
   tokio::spawn(async move {
      if let Err(e) = test_pool.get().await {
         settings::log(&format!("Database connection error: {}", e)).await;
      }
   });

   // Сохраним доступ к БД
   match database::DB.set(pool) {
      Ok(_) => log::info!("Database connected"),
      _ => {
         log::info!("Something wrong with database");
         settings::log("Something wrong with database").await;
      }
   }

   // Проверим существование таблиц и если их нет, создадим
   if database::is_tables_exist().await {
      log::info!("Table restaurants exist, open existing data");
   } else {
      log::info!("Table restaurants do not exist, create new tables: {}", database::is_success(database::create_tables().await));
   }
   
   // Инициализируем структуру с картинками для категорий
   database::cat_image_init().await;
   
   Dispatcher::new(Arc::clone(&bot))
   .messages_handler(DialogueDispatcher::new(|cx| async move {
      let res = handle_message(cx).await;
      if let Err(e) = res {
         settings::log(&format!("main:{}", e)).await;
         DialogueStage::Exit
      } else {
         res.unwrap()
      }
   }))
   .callback_queries_handler(handle_callback_query)
   .inline_queries_handler(handle_inline_query)
   .dispatch_with_listener(
      webhook(bot).await,
      LoggingErrorHandler::with_custom_text("An error from the update listener"),
   )
   .await;
}

// Отправить сообщение
pub async fn send_message(bot: &Arc<Bot>, chat_id: ChatId, s: &str) -> bool {
   if let Err(e) = bot.send_message(chat_id, s).send().await {
      settings::log(&format!("Ошибка {}", e)).await;
      false
   } else {true}
}

// Отправить сообщение ресторатору
pub async fn edit_message_to_caterer_mode(cx: cmd::Cx<(i32, i32, Box<cmd::DialogueState>)>) -> cmd::Res {
   // Извлечём параметры
   let (user_id, caterer_id, boxed_origin) = cx.dialogue;

   if let Some(text) = cx.update.text() {
      // Удалим из строки слеши
      let s = cmd::remove_slash(text).await;

      // Если строка не пустая, продолжим
      let text = if !s.is_empty() {

         // Адресат сообщения
         let to = ChatId::Id(i64::from(caterer_id));
      
         // Текст для отправки
         let user_name = if let Some(u) = cx.update.from() {&u.first_name} else {""};
         let s = format!("Сообщение от {}\n{}\n Для ответа нажмите ссылку /snd{}", user_name, s, user_id);

         // Отправляем сообщение и сообщаем результат
         if send_message(&cx.bot, to, &s).await {String::from("Сообщение отправлено")}
         else {String::from("Ошибка отправки сообщения")}
      } else {
         String::from("Отмена отправки сообщения")
      };

      // Уведомим о результате
      let new_cx = DialogueDispatcherHandlerCx::new(cx.bot.clone(), cx.update.clone(), ());
      new_cx.answer(text)
      .reply_markup(boxed_origin.m)
      .disable_notification(true)
      .send()
      .await?;
   }

   // Возвращаемся в предыдущий режим c обновлением кнопок
   next(boxed_origin.d)
}

