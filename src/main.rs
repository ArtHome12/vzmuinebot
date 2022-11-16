/* ===============================================================================
Restaurant menu bot.
Main module. 21 May 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020-2022 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use std::{env, fmt::Debug, sync::Arc};
use futures::future::BoxFuture;

use teloxide::{prelude::*, 
   dispatching::{
      update_listeners::{webhooks},
      dialogue::InMemStorage,
   },
   error_handlers::ErrorHandler,
};
use native_tls::{TlsConnector};
use postgres_native_tls::MakeTlsConnector;
use reqwest::{Url};
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use crate::states::*;

mod database;
mod environment;
mod node;
mod states;
mod gear;
mod navigation;
mod cart;
mod customer;
mod orders;
mod callback;
mod ticket;
mod general;
mod registration;
mod search;
mod loc;

// ============================================================================
// [Run!]
// ============================================================================
#[tokio::main]
async fn main() {
   run().await;
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
   pretty_env_logger::init();
   log::info!("Starting...");

   let bot = Bot::from_env();

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
      log::info!("Table nodes exist, open existing data");
   } else {
      log::info!("Table nodes do not exist, creating new tables");

      let res = database::create_tables().await;
      match res {
         Ok(_) => log::info!("tables created"),
         Err(e) => log::error!("main::run(): {}", e),
      }
   }

   // Data for localization
   let loc = crate::loc::Locale::new("en");
   if loc::LOC.set(loc).is_err() {
      log::error!("main::run() loc set error")
   }

   let teloxide_token = env::var("TELOXIDE_TOKEN").expect("TELOXIDE_TOKEN env variable missing");

   // Heroku auto defines a port value
   let port: u16 = env::var("PORT")
      .expect("PORT env variable missing")
      .parse()
      .expect("PORT value to be integer");

   // Heroku host example .: "heroku-ping-pong-bot.herokuapp.com"
   let host = env::var("HOST").expect("have HOST env variable");
   let path = format!("bot{}", teloxide_token);
   let url =  Url::parse(&format!("https://{}/{}", host, path))
      .unwrap();

   let addr = ([0, 0, 0, 0], port).into();

   let update_listener = webhooks::axum(bot.clone(), webhooks::Options::new(addr, url))
      .await
      .expect("Couldn't setup webhook");

   Dispatcher::builder(bot.clone(), states::schema())
   .dependencies(dptree::deps![InMemStorage::<State>::new()])
   // .default_handler(|upd| async move {
   //    environment::log(&format!("main::Unhandled update: {:?}", upd)).await;
   // })
   // If the dispatcher fails for some reason, execute this handler.
   .error_handler(Arc::new(MyErrorHandler{}))
   .build()
   .dispatch_with_listener(
      update_listener,
      LoggingErrorHandler::with_custom_text("main::An error from the update listener"),
   )
   .await;
}


