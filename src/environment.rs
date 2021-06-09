/* ===============================================================================
Restaurant menu bot.
Global vars, service chat. 18 July 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use chrono::{FixedOffset, NaiveDateTime, Utc,};
use once_cell::sync::{OnceCell};
use std::{env, };
use teloxide::{
   prelude::*,
   // types::{ChatId},
};

// Настройки
pub static VARS: OnceCell<Vars> = OnceCell::new();

// Хранит данные для работы логирования в чат
#[derive(Clone)]
pub struct ServiceChat {
   pub id: i64,
   pub bot: AutoSend<Bot>,
}

// Отправляет сообщение в телеграм группу для лога
impl ServiceChat {
   // Непосредственно отправляет сообщение
   async fn send(&self, text: &str, silence: bool, reply_to: Option<i32>) -> Option<i32> {
      send_to_chat(&self, text, silence, reply_to).await
   }
}

// Отправляет в служебный чат сообщение в молчаливом режиме
pub async fn log(text: &str) -> Option<i32> {
   if let Some(chat) = &VARS.get().unwrap().chat {
      chat.send(text, true, None).await
   } else {
      None
   }
}

pub async fn log_reply(text: &str, reply_to: Option<i32>) -> Option<i32> {
   if let Some(chat) = &VARS.get().unwrap().chat {
      chat.send(text, true, reply_to).await
   } else {
      None
   }
}

// Отправляет в служебный чат сообщение с уведомлением
pub async fn log_and_notify(text: &str) {
   if let Some(chat) = &VARS.get().unwrap().chat {
      chat.send(text, false, None).await;
   }
}

// Пересылает в служебный чат сообщение, возвращая идентификатор этого сообщения в служебном чате
/* pub async fn log_forward(from: i64, message_id: i32) {
   if let Some(chat) = &VARS.get().unwrap().chat {
      let res = chat.bot.forward_message(chat.id, from, message_id).await;
      match res {
         Ok(msg) => (),
         Err(e) => log::info!("Error log_forward(from={}): {}", from, e),
      }
   }
} */

// Отправляет сообщение без использования self
async fn send_to_chat(chat: &ServiceChat, text: &str, silence: bool, reply_to: Option<i32>) -> Option<i32> {
   // Формируем сообщение для откравки
   let mut res = chat.bot
   .send_message(chat.id, text)
   .disable_notification(silence);

   if let Some(reply_to) = reply_to {
      res = res.reply_to_message_id(reply_to);
   }

   match res.await {
      Ok(m) => Some(m.id),
      Err(err) => {
         log::info!("Error log({}): {}", text, err);
         None
      }
   }
}

// Для удобства обёртка - отправляет сообщение в чат, если он есть, и не делает ничего, если его нет
async fn int_log(chat: Option<ServiceChat>, text: &str) {
   if let Some(c) = chat {send_to_chat(&c, text, true, None).await;}
}


// Информация из переменных окружения
pub struct Vars {
   // Сервисный чат
   chat: Option<ServiceChat>,

   // Контактная информация администраторов бота
   admin_contact_info: String,

   // Идентификаторы админов, макс 3 штуки
   admin_id1: i64,
   admin_id2: i64,
   admin_id3: i64,

   // Единица измерения цены
   price_unit: String,

   // Часовой пояс
   time_zone: FixedOffset,

   // Ссылка для рекламы
   // link: String,
}

impl Vars {
   pub async fn from_env(service_bot: AutoSend<Bot>) -> Self {

      // Link to bot for advertise from its name
      /* let link = service_bot
      .get_me()
      .send()
      .await
      .map_err(|_| ())
      .and_then(|me| {
         match me.user.username {
            Some(name) => Ok(format!("http://t.me/{}?start=", name)),
            None => Err(()),
         }
      });
      let link = link.unwrap_or(String::from("Ошибка")); */

      // Служебный чат, чтобы иметь возможность выводить в него ошибки
      let chat = if let Ok(log_group_id_env) = env::var("LOG_GROUP_ID") {
         if let Ok(log_group_id) = log_group_id_env.parse::<i64>() {
            // Сохраняем id и копию экземпляра бота в глобальной переменной
            Some(ServiceChat {
               id: log_group_id,
               bot: service_bot,
            })
         } else {
            log::info!("Environment variable LOG_GROUP_ID must be integer");
            None
         }
      } else {
         log::info!("There is no environment variable LOG_GROUP_ID, no service chat");
         None
      };

      Vars {
         // Контактная информация администраторов бота
         admin_contact_info: {
            match env::var("CONTACT_INFO") {
               Ok(s) => {
                  log::info!("admin name is {}", s);
                  s
               }
               Err(e) => {
                  int_log(chat.clone(), &format!("Something wrong with CONTACT_INFO: {}", e)).await;
                  String::default()
               }
            }
         },

         // Идентификаторы админов, макс 3 штуки
         admin_id1: {
            match env::var("TELEGRAM_ADMIN_ID1") {
               Ok(s) => match s.parse::<i64>() {
                     Ok(n) => n,
                     Err(e) => {
                        int_log(chat.clone(), &format!("Something wrong with TELEGRAM_ADMIN_ID1: {}", e)).await;
                        0
                     }
               }
               Err(e) => {
                  int_log(chat.clone(), &format!("Something wrong with TELEGRAM_ADMIN_ID1: {}", e)).await;
                  0
               }
            }
         },

         admin_id2: {
            match env::var("TELEGRAM_ADMIN_ID2") {
               Ok(s) => if s.is_empty() {0} else {
                  match s.parse::<i64>() {
                     Ok(n) => n,
                     Err(e) => {
                        int_log(chat.clone(), &format!("Something wrong with TELEGRAM_ADMIN_ID2: {}", e)).await;
                        0
                     }
                  }
               }
               Err(_) => 0 // если переменная не задана, это нормально
            }
         },

         admin_id3: {
            match env::var("TELEGRAM_ADMIN_ID3") {
               Ok(s) => if s.is_empty() {0} else {
                  match s.parse::<i64>() {
                     Ok(n) => n,
                     Err(e) => {
                        int_log(chat.clone(), &format!("Something wrong with TELEGRAM_ADMIN_ID3: {}", e)).await;
                        0
                     }
                  }
               }
               Err(_) => 0 // если переменная не задана, это нормально
            }
         },

         // Единица измерения цены
         price_unit: {
            match env::var("PRICE_UNIT") {
               Ok(s) => s,
               Err(e) => {
                  int_log(chat.clone(), &format!("Something wrong with PRICE_UNIT: {}", e)).await;
                  String::default()
               }
            }
         },

         // Часовой пояс
         time_zone: {
            match env::var("TIME_ZONE") {
               Ok(s) => match s.parse::<i32>() {
                     Ok(n) => FixedOffset::east(n * 3600),
                     Err(e) => {
                        int_log(chat.clone(), &format!("Something wrong with TIME_ZONE: {}", e)).await;
                        FixedOffset::east(0)
                     }
               }
               Err(e) => {
                  int_log(chat.clone(), &format!("Something wrong with TIME_ZONE: {}", e)).await;
                  FixedOffset::east(0)
               }
            }
         },

         // link,

         // Служебный чат
         chat,
      }
   }
}

// Контактная информация администраторов бота
pub fn admin_contact_info() -> String {
   VARS.get().unwrap().admin_contact_info.clone()
}

// Возвращает текущее время с учётом часового пояса
pub fn current_date_time() -> NaiveDateTime {
   // Часовой пояс
   let our_timezone = VARS.get().unwrap().time_zone;

   // Текущее время
   Utc::now().with_timezone(&our_timezone).naive_local()
}

pub fn time_zone_info() -> String {
   // Часовой пояс
   let our_timezone = VARS.get().unwrap().time_zone.local_minus_utc() / 3600;
   if our_timezone > 0 {
      format!("UTC+{}", our_timezone)
   } else {
      format!("UTC{}", our_timezone)
   }
}

// Возвращает истину, если user_id принадлежит администратору
/* pub fn is_admin(user_id: Option<&teloxide::types::User>) -> bool {
   match user_id {
      Some(user) => is_admin_id(user.id),
      None => false,
   }
} */

// Возвращает истину, если user_id принадлежит администратору
pub fn is_admin_id(user_id: i64) -> bool {
   let vars = VARS.get().unwrap();
   user_id == vars.admin_id1 || user_id == vars.admin_id2 || user_id == vars.admin_id3
}

// Форматирование цены с единицей измерения
pub fn price_with_unit(price: usize) -> String {
   format!("{}{}", price,  VARS.get().unwrap().price_unit)
}

// Картинка по-умолчанию для использования в качестве заглушки в режиме с инлайн-кнопками
/* pub fn def_image() -> String {
   VARS.get().unwrap().def_image_id.clone()
} */

// Ссылка для рекламы
/* pub fn link() -> String {
   VARS.get().unwrap().link.clone()
} */