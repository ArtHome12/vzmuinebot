/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Модуль для связи с СУБД. 28 May 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use once_cell::sync::{OnceCell};
use deadpool_postgres::{Pool, Client};
use std::collections::HashMap;
use std::sync::RwLock;

use crate::settings;

// Пул клиентов БД
pub static DB: OnceCell<Pool> = OnceCell::new();

// Картинки по-умолчанию для категорий блюд
type CatImageList = HashMap<i32, String>;
pub static CI: OnceCell<RwLock<CatImageList>> = OnceCell::new();

// ============================================================================
// [Restaurants table]
// ============================================================================


// Проверяет существование таблиц
pub async fn is_tables_exist() -> bool {
   // Получаем клиента БД
   let client = db_client().await;
   if client.is_none() {return false;}

   // Выполняем запрос
   let rows = client.unwrap()
   .query("SELECT table_name FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_NAME='restaurants'", &[]).await;

   // Проверяем результат
   match rows {
      Ok(data) => !data.is_empty(),
      _ => false,
   }
}

// Создаёт новые таблицы
pub async fn create_tables() -> bool {
   // Получаем клиента БД
   let client = db_client().await;
   if client.is_none() {return false;}

   // Таблица с данными о ресторанах
   let query = client.unwrap()
   .batch_execute("CREATE TABLE restaurants (
         PRIMARY KEY (user_id),
         user_id        INTEGER        NOT NULL,
         title          VARCHAR(100)   NOT NULL,
         info           VARCHAR(512)   NOT NULL,
         active         BOOLEAN        NOT NULL,
         enabled        BOOLEAN        NOT NULL,
         rest_num       SERIAL,
         image_id       VARCHAR(512),
         opening_time   TIME           NOT NULL,    
         closing_time   TIME           NOT NULL);

      CREATE TABLE groups (
         PRIMARY KEY (rest_num, group_num),
         rest_num       INTEGER        NOT NULL,
         group_num      INTEGER        NOT NULL,
         title          VARCHAR(100)   NOT NULL,
         info           VARCHAR(512)   NOT NULL,
         active         BOOLEAN        NOT NULL,
         cat_id         INTEGER        NOT NULL,
         opening_time   TIME           NOT NULL,    
         closing_time   TIME           NOT NULL);

      CREATE TABLE dishes (
         PRIMARY KEY (rest_num, group_num, dish_num),
         rest_num       INTEGER        NOT NULL,
         dish_num       INTEGER        NOT NULL,
         title          VARCHAR(100)   NOT NULL,
         info           VARCHAR(512)   NOT NULL,
         active         BOOLEAN        NOT NULL,
         group_num      INTEGER        NOT NULL,
         price          INTEGER        NOT NULL,
         image_id       VARCHAR(512));

      CREATE TABLE users (
         PRIMARY KEY (user_id),
         user_id        INTEGER        NOT NULL,
         user_name      VARCHAR(100)   NOT NULL,
         contact        VARCHAR(100)   NOT NULL,
         address        VARCHAR(100)   NOT NULL,
         last_seen      TIMESTAMP      NOT NULL,
         compact        BOOLEAN        NOT NULL,
         pickup         BOOLEAN        NOT NULL);

      CREATE TABLE orders (
         PRIMARY KEY (user_id, rest_num, group_num, dish_num),
         user_id        INTEGER        NOT NULL,
         rest_num       INTEGER        NOT NULL,
         group_num      INTEGER        NOT NULL,
         dish_num       INTEGER        NOT NULL,
         amount         INTEGER        NOT NULL);

      CREATE TABLE category (
         PRIMARY KEY (cat_id),
         cat_id         INTEGER        NOT NULL,
         image_id       VARCHAR(512));

      CREATE TABLE tickets (
         PRIMARY KEY (ticket_id),
         ticket_id      SERIAL         NOT NULL,
         eater_id       INTEGER        NOT NULL,
         caterer_id     INTEGER        NOT NULL,
         eater_msg_id   INTEGER        NOT NULL,
         caterer_msg_id INTEGER        NOT NULL,
         stage          INTEGER        NOT NULL,
         eater_status_msg_id     INTEGER,
         caterer_status_msg_id   INTEGER);")
   .await;
      
   match query {
      Ok(_) => true,
      Err(e) => {
         settings::log(&format!("Error create_tables: {}", e)).await;
         false
       }
   }
}

// Успешно-неуспешно
pub fn is_success(flag : bool) -> &'static str {
   if flag {
      "успешно"
  } else {
      "ошибка"
  }
}


// Обёртка, возвращает пул клиентов
async fn db_client() -> Option<Client> {
   match DB.get().unwrap().get().await {
      Ok(client) => Some(client),
      Err(e) => {
         settings::log(&format!("No db client: {}", e)).await;
         None
      }
   }
}

// Инициализирует структуру с картинками для категорий
pub async fn cat_image_init() {
   // Внесём значения по-умолчанию, а потом попытаемся прочесть их их базы
   let mut hash: CatImageList = HashMap::new();
   hash.insert(1, settings::default_photo_id());
   hash.insert(2, settings::default_photo_id());
   hash.insert(3, settings::default_photo_id());
   hash.insert(4, settings::default_photo_id());

   // Получаем клиента БД
   let client = db_client().await;
   if client.is_some() {
      // Выполняем запрос
      let query = client.unwrap()
      .query("SELECT cat_id, image_id FROM category", &[])
      .await;
      match query {
         Ok(rows) => {
            for row in rows {
               let s: Option<String> = row.get(1);
               if s.is_some() {hash.insert(row.get(0), s.unwrap());}
            }
         }
         Err(e) => {
            settings::log(&format!("Error db::cat_image_init: {}", e)).await;
         }
      };
   };

   // Сохраняем данные
   if let Err(_) = CI.set(RwLock::new(hash)) {
      settings::log(&format!("Error db::cat_image_init2")).await;
   }
}
