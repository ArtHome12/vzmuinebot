/* ===============================================================================
Restaurant menu bot.
Main module. 21 May 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use std::{convert::Infallible, env, net::SocketAddr};
use customer::Customer;
use teloxide::{prelude::*, dispatching::{update_listeners::{self, StatefulListener},
   stop_token::AsyncStopToken}, types::User,
   dispatching::dialogue::InMemStorage,
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use native_tls::{TlsConnector};
use postgres_native_tls::MakeTlsConnector;
use warp::Filter;
use reqwest::{StatusCode, Url};
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};

mod database;
mod environment;
mod node;
mod states;
mod gear;
mod navigation;
mod basket;
mod customer;
mod orders;
mod callback;
mod ticket;
mod general;
mod registration;
mod search;
use crate::states::*;
use crate::database as db;

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

pub async fn webhook<'a>(bot: AutoSend<Bot>) -> impl update_listeners::UpdateListener<Infallible> {
   // Heroku auto defines a port value
   let teloxide_token = env::var("TELOXIDE_TOKEN").expect("TELOXIDE_TOKEN env variable missing");
   let port: u16 = env::var("PORT")
      .expect("PORT env variable missing")
      .parse()
      .expect("PORT value to be integer");
   // Heroku host example .: "heroku-ping-pong-bot.herokuapp.com"
   let host = env::var("HOST").expect("have HOST env variable");
   let path = format!("bot{}", teloxide_token);
   let url =  Url::parse(&format!("https://{}/{}", host, path))
   .unwrap();

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

   let (stop_token, stop_flag) = AsyncStopToken::new_pair();

   let addr = format!("0.0.0.0:{}", port).parse::<SocketAddr>().unwrap();
   let server = warp::serve(server);
   let (_addr, fut) = server.bind_with_graceful_shutdown(addr, stop_flag);

   // You might want to use serve.key_path/serve.cert_path methods here to
   // setup a self-signed TLS certificate.

   tokio::spawn(fut);
   let stream = UnboundedReceiverStream::new(rx);

   fn streamf<S, T>(state: &mut (S, T)) -> &mut S { &mut state.0 }
   
   StatefulListener::new((stream, stop_token), streamf, |state: &mut (_, AsyncStopToken)| state.1.clone())
}

async fn run() {
   log::info!("Starting...");

   let bot = Bot::from_env().auto_send();

   // Settings from environments
   let vars = environment::Vars::from_env(bot.clone()).await;
   match environment::VARS.set(vars) {
      Ok(_) => {environment::log("Bot restarted").await;},
      _ => log::info!("Something wrong with TELEGRAM_LOG_CHAT"),
   }

   // Open database
   let database_url = env::var("DATABASE_URL").expect("DATABASE_URL env variable missing");

   let connector = TlsConnector::builder()
   // .add_root_certificate(cert)
   .danger_accept_invalid_certs(true)
   .build().unwrap();
   let connector = MakeTlsConnector::new(connector);

   let pg_config = database_url.parse::<tokio_postgres::Config>().expect("DATABASE_URL env variable wrong");
   let mgr_config = ManagerConfig {recycling_method: RecyclingMethod::Fast};
   let mgr = Manager::from_config(pg_config, connector, mgr_config);
   let pool = Pool::builder(mgr).max_size(16).build().unwrap();

   // Test connection to database
   let test_pool = pool.clone();
   tokio::spawn(async move {
      if let Err(e) = test_pool.get().await {
         environment::log(&format!("Database connection error: {}", e)).await;
      }
   });

   // Save db clients pool
   match database::DB.set(pool) {
      Ok(_) => log::info!("Database connected"),
      _ => {
         log::info!("Something wrong with database");
         environment::log("Something wrong with database").await;
      }
   }

   // Check and create tables
   if database::is_tables_exist().await {
      log::info!("Table restaurants exist, open existing data");
   } else {
      log::info!("Table restaurants do not exist, create new tables: {}", database::is_success(database::create_tables().await));
   }

   // Create handlers
   let handler = Update::filter_message()
   .enter_dialogue::<Message, InMemStorage<State>, State>()
   .branch(
      // Filtering allow you to filter updates by some condition.
      dptree::filter(|msg: Message| {
         msg.chat.is_private() && msg.from().is_some()
      })
          // An endpoint is the last update handler.
          .endpoint(|msg: Message, bot: AutoSend<Bot>| async move {
              log::info!("Received a message from a private chat.");
              bot.send_message(msg.chat.id, "This is a private chat.").await?;
              respond(())
          }),
   );


   /* Dispatcher::new(bot.clone())
   .messages_handler(DialogueDispatcher::new(|DialogueWithCx { cx, dialogue }| async move {

      let res = handle_message(cx, dialogue.unwrap()).await;

      if let Err(e) = res {
         environment::log(&format!("main::dialog:{}", e)).await;
         DialogueStage::Exit
      } else {
         res.unwrap()
      }
   }))
   .callback_queries_handler(handle_callback_query)
   // .inline_queries_handler(handle_inline_query)
   .dispatch_with_listener(
      webhook(bot).await,
      LoggingErrorHandler::with_custom_text("An error from the update listener"),
   )
   .await; */
}

/* async fn handle_message(cx: UpdateWithCx<AutoSend<Bot>, Message>, dialogue: Dialogue) -> TransitionOut<Dialogue> {

   // Negative for chats, positive personal
   let chat_id = cx.update.chat_id();

   if chat_id > 0 {
      // Insert new user or update his last seen time
      let user = &cx.update.from();
      if user.is_some() {
         update_last_seen(user.unwrap())
         .await
         .map_err(|s| map_req_err(s))?;
      }

      // Collect info about update, if no text there may be image id or location
      let text = match cx.update.text() {
         Some(text) => String::from(text),
         None => {
            let picture = cx.update.photo();
            if let Some(sizes) = picture {
               sizes[0].file_id.clone()
            } else if let Some(_) = cx.update.location() {
               Customer::make_location(cx.update.id)
            } else {String::default()}
   
         }
      };

      if text == "" {
         cx.answer("Текстовое сообщение, пожалуйста!").await?;
      } else {
         // Handle message with FSM
         return dialogue.react(cx, text).await;
      }
   } else {
      // For chat messages react only command for printout group id (need for identify service chat)
      if let Some(input) = cx.update.text() {
         match input.get(..5).unwrap_or_default() {
            "/chat" => {
               let text = format!("Chat id={}", chat_id);
               cx.reply_to(&text).await?;
            }
            _ => (),
         }
      }
   }
   
   next(dialogue)
}

async fn handle_callback_query(rx: DispatcherHandlerRx<AutoSend<Bot>, CallbackQuery>) {
  UnboundedReceiverStream::new(rx)
  .for_each_concurrent(None, |cx| async move {

      // Update user last seen time and process query
      let user = &cx.update.from;
      let res = update_last_seen(user).await
      .and(callback::update(cx).await);

      if let Err(err) = res {
         environment::log(&format!("main::handle_callback_query:{}", err)).await;
      }
   })
  .await;
}

async fn update_last_seen(user: &User) -> Result<(), String> {
   let user_id = user.id;
   let successful = db::user_update_last_seen(user_id).await?;

   // If unsuccessful, then there is no such user
   if !successful {
      // Collect info about the new user and store in database
      let name = if let Some(last_name) = &user.last_name {
         format!("{} {}", user.first_name, last_name)
      } else {user.first_name.clone()};

      let contact = if let Some(username) = &user.username {
         format!(" @{}", username)
      } else {String::from("-")};

      db::user_insert(user_id, name, contact).await?;
   }
   Ok(())
} */

/* async fn handle_inline_query(rx: DispatcherHandlerRx<AutoSend<Bot>, InlineQuery>) {
   UnboundedReceiverStream::new(rx)
   .for_each_concurrent(None, |cx| async move {
      inline_handle_message(cx).await
   })
  .await;
}

pub async fn inline_handle_message(cx: UpdateWithCx<AutoSend<Bot>, InlineQuery>) {
   let id = cx.update.id;
   todo!();
} */