/* ===============================================================================
Restaurant menu bot.
Main module. 21 May 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020-2022 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use std::{convert::Infallible, env, net::SocketAddr, fmt::Debug, sync::Arc};
use futures::future::BoxFuture;
use teloxide::{prelude::*, 
   dispatching::{
      update_listeners::{self, StatefulListener},
      stop_token::AsyncStopToken,
      dialogue::InMemStorage,
   },
   error_handlers::ErrorHandler,
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

   tokio::spawn(fut);
   let stream = UnboundedReceiverStream::new(rx);

   fn streamf<S, T>(state: &mut (S, T)) -> &mut S { &mut state.0 }
   
   StatefulListener::new((stream, stop_token), streamf, |state: &mut (_, AsyncStopToken)| state.1.clone())
}

struct MyErrorHandler {}
impl<E> ErrorHandler<E> for MyErrorHandler
where
    E: Debug,
{
    fn handle_error(self: Arc<Self>, error: E) -> BoxFuture<'static, ()> {
      let text = format!("main::handle_error: {:?}", error);
      log::error!("{}", text);

      let fut = async move {
         if environment::log(&text).await.is_none() {
            log::info!("main::Unable to send message to the service chat");
         };
      };

      Box::pin(fut)
   }
}


async fn run() {
   // pretty_env_logger::init();
   let mut builder = pretty_env_logger::formatted_builder();
   builder.target(env_logger::Target::Stdout);
   builder.init();

   log::info!("Starting...");

   let bot = Bot::from_env().auto_send();

   // Settings from environments
   let vars = environment::Vars::from_env(bot.clone()).await;
   match environment::VARS.set(vars) {
      Ok(_) => {environment::log("Bot restarted").await;},
      _ => log::info!("Something wrong with TELEGRAM_LdOG_CHAT"),
   }

   // Open database
   let database_url = env::var("DATABASE_URL").expect("DATABASE_URL env variable missing");

   let connector = TlsConnector::builder()
   // .add_root_certificate(cert)
   .danger_accept_invalid_certs(true)
   .build()
   .unwrap();
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

   Dispatcher::builder(bot.clone(), states::schema())
   .dependencies(dptree::deps![InMemStorage::<State>::new()])
   .default_handler(|upd| async move {
      environment::log(&format!("main::Unhandled update: {:?}", upd)).await;
   })
   // If the dispatcher fails for some reason, execute this handler.
   .error_handler(Arc::new(MyErrorHandler{}))
   .build()
   .dispatch_with_listener(
      webhook(bot).await,
      LoggingErrorHandler::with_custom_text("main::An error from the update listener"),
   )
   .await;
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
*/
