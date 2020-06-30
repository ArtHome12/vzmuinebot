/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Модуль для связи с СУБД. 28 May 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use chrono::{NaiveTime, FixedOffset, NaiveDateTime, Timelike, Utc};
use once_cell::sync::{OnceCell};
use text_io::try_scan;
use teloxide::{
   prelude::*,
   types::{User, ChatId},
};
use std::collections::BTreeMap;

//use tokio_postgres::{Row, Error};
// extern crate runtime_fmt;

use crate::language as lang;

// Клиент БД
pub static DB: OnceCell<tokio_postgres::Client> = OnceCell::new();

// Телеграм ник администратора бота для связи
pub static CONTACT_INFO: OnceCell<String> = OnceCell::new();
pub static TELEGRAM_ADMIN_ID: OnceCell<(i32, i32, i32)> = OnceCell::new();

// Телеграм ник группы для вывода лога
pub static TELEGRAM_LOG_CHAT: OnceCell<ServiceChat> = OnceCell::new();

// Единица измерения цены
pub static PRICE_UNIT: OnceCell<String> = OnceCell::new();

// Часовой пояс
pub static TIME_ZONE: OnceCell<FixedOffset> = OnceCell::new();

// Картинка по-умолчанию
pub static DEFAULT_IMAGE_ID: OnceCell<String> = OnceCell::new();

// Бот для отправки сообщений между ресторатором и едоком
pub static BOT: OnceCell<std::sync::Arc<Bot>> = OnceCell::new();


// ============================================================================
// [User]
// ============================================================================

// Возвращает список ресторанов с активными группами в данной категории
//
pub type RestaurantList = BTreeMap<i32, String>;
pub async fn restaurant_by_category(cat_id: i32) -> RestaurantList {
   // Выполняем запрос
   let rows = DB.get().unwrap()
      .query("SELECT r.rest_num, r.title FROM restaurants AS r INNER JOIN (SELECT DISTINCT rest_num FROM groups WHERE cat_id=$1::INTEGER AND active = TRUE) g ON r.rest_num = g.rest_num 
      WHERE r.active = TRUE", &[&cat_id])
      .await;

   // Возвращаем результат
   match rows {
      Ok(data) => data.into_iter().map(|row| (row.get(0), row.get(1))).collect(),
      Err(e) => {
         // Сообщаем об ошибке и возвращаем пустой список
         log(&format!("Error restaurant_by_category: {}", e)).await;
         BTreeMap::<i32, String>::new()
      }
   }
}

// Возвращает список ресторанов с активными группами в указанное время
pub async fn restaurant_by_now(time: NaiveTime) -> RestaurantList {
   // Выполняем запрос
   let rows = DB.get().unwrap()
      .query("SELECT r.rest_num, r.title FROM restaurants AS r INNER JOIN (SELECT DISTINCT rest_num FROM groups WHERE active = TRUE AND 
         ($1::TIME BETWEEN opening_time AND closing_time) OR (opening_time > closing_time AND $1::TIME > opening_time)) g ON r.rest_num = g.rest_num WHERE r.active = TRUE", &[&time])
      .await;

   // Возвращаем результат
   match rows {
      Ok(data) => data.into_iter().map(|row| (row.get(0), row.get(1))).collect(),
      Err(e) => {
         // Сообщаем об ошибке и возвращаем пустой список
         log(&format!("Error restaurant_by_now: {}", e)).await;
         BTreeMap::<i32, String>::new()
      }
   }
}

// Возвращает описание, фото и список групп выбранного ресторана и категории
pub struct GroupListWithRestaurantInfo {
   pub info: String, 
   pub image_id: Option<String>,
   pub groups: BTreeMap<i32, String>
}

async fn subselect_groups(rest_num: i32, cat_id: i32) -> BTreeMap<i32, String> {
   // Выполняем запрос групп
   let rows = DB.get().unwrap()
      .query("SELECT group_num, title, opening_time, closing_time FROM groups WHERE rest_num=$1::INTEGER AND cat_id=$2::INTEGER AND active = TRUE", &[&rest_num, &cat_id])
      .await;

   // Возвращаем результат
   match rows {
      Ok(data) => data.into_iter().map(|row| -> (i32, String) {
         let group_num: i32 = row.get(0);
         let title: String = row.get(1);
         let opening_time: NaiveTime = row.get(2);
         let closing_time: NaiveTime = row.get(3);

         // Если время указано без минут, то выводим только часы
         let opening = if opening_time.minute() == 0 { opening_time.format("%H") } else { opening_time.format("%H:%M") };
         let closing = if closing_time.minute() == 0 { closing_time.format("%H") } else { closing_time.format("%H:%M") };

         // Возвращаем хешстроку
         (group_num, format!("   {} ({}-{})", title, opening, closing))
      }).collect(),
      Err(e) => {
         // Сообщаем об ошибке и возвращаем пустой список
         log(&format!("Error subselect_groups: {}", e)).await;
         BTreeMap::<i32, String>::new()
      }
   }
}

pub async fn groups_by_restaurant_and_category(rest_num: i32, cat_id: i32) -> Option<GroupListWithRestaurantInfo> {
   // Выполняем запрос информации о ресторане
   let rows = DB.get().unwrap()
      .query("SELECT title, info, image_id FROM restaurants WHERE rest_num=$1::INTEGER", &[&rest_num])
      .await;

   match rows {
      Ok(data) => {
         if !data.is_empty() {
            // Параметры ресторана
            let title: String = data[0].get(0);
            let info: String = data[0].get(1);
            let res = GroupListWithRestaurantInfo {
               // info: format!("Заведение: {}\nОписание: {}\nПодходящие разделы меню для {}", title, info, id_to_category(cat_id)),
               info: format!("Заведение: {}\nОписание: {}", title, info),
               image_id: data[0].get(2),
               groups: subselect_groups(rest_num, cat_id).await,
            };

            // Окончательный результат
            Some(res)
         } else {
            None
         }
      }
      Err(e) => {
         // Сообщим об ошибке
         log(&format!("Error groups_by_restaurant_and_category: {}", e)).await;
         None
      }
   }
}

async fn subselect_groups_now(rest_num: i32, time: NaiveTime) -> BTreeMap<i32, String> {
   // Выполняем запрос групп
   let rows = DB.get().unwrap()
      .query("SELECT group_num, title, opening_time, closing_time FROM groups WHERE rest_num=$1::INTEGER AND active = TRUE AND $2::TIME BETWEEN opening_time AND closing_time", &[&rest_num, &time])
      .await;

   // Возвращаем результат
   match rows {
      Ok(data) => data.into_iter().map(|row| -> (i32, String) {
         let group_num: i32 = row.get(0);
         let title: String = row.get(1);
         let opening_time: NaiveTime = row.get(2);
         let closing_time: NaiveTime = row.get(3);

         // Если время указано без минут, то выводим только часы
         let opening = if opening_time.minute() == 0 { opening_time.format("%H") } else { opening_time.format("%H:%M") };
         let closing = if closing_time.minute() == 0 { closing_time.format("%H") } else { closing_time.format("%H:%M") };

         // Возвращаем хешстроку
         (group_num, format!("   {} ({}-{})", title, opening, closing))
      }).collect(),
      Err(e) => {
         // Сообщаем об ошибке и возвращаем пустой список
         log(&format!("Error subselect_groups_now: {}", e)).await;
         BTreeMap::<i32, String>::new()
      }
   }
}

// Возвращает описание, список групп выбранного ресторана, работающего в указанное время и фото, если есть
pub async fn groups_by_restaurant_now(rest_num: i32) -> Option<GroupListWithRestaurantInfo> {
   // Текущее время
   let our_timezone = TIME_ZONE.get().unwrap();
   let time = Utc::now().with_timezone(our_timezone).naive_local().time();

   // Выполняем запрос информации о ресторане
   let rows = DB.get().unwrap()
      .query("SELECT title, info, image_id FROM restaurants WHERE rest_num=$1::INTEGER", &[&rest_num])
      .await;

   match rows {
      Ok(data) => {
         if !data.is_empty() {
            // Параметры ресторана
            let title: String = data[0].get(0);
            let info: String = data[0].get(1);
            let res = GroupListWithRestaurantInfo {
               info: format!("Заведение: {}\nОписание: {}", title, info),
               image_id: data[0].get(2),
               groups: subselect_groups_now(rest_num, time).await,
            };

            // Окончательный результат
            Some(res)
         } else {None}
      }
      Err(e) => {
         // Сообщим об ошибке
         log(&format!("Error groups_by_restaurant_now: {}", e)).await;
         None
      }
   }
}

// Возвращает список блюд выбранного ресторана и группы
pub struct DishListWithGroupInfo {
   pub info: String, 
   pub dishes: BTreeMap<i32, String>
}

async fn subselect_dishes(rest_num: i32, group_num: i32) -> BTreeMap<i32, String> {
   // Выполняем запрос списка блюд
   let rows = DB.get().unwrap()
      .query("SELECT dish_num, title, price FROM dishes WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND active = TRUE ORDER BY dish_num", &[&rest_num, &group_num])
      .await;

   // Проверяем результат
   match rows {
      Ok(data) => data.into_iter().map(|row| -> (i32, String) {
         let dish_num: i32 = row.get(0);
         let title: String = row.get(1);
         let price: i32 = row.get(2);

         // Возвращаем хешстроку
         (dish_num, format!("   {} {}", title, price_with_unit(price)))
      }).collect(),
      Err(e) => {
         // Сообщаем об ошибке и возвращаем пустой список
         log(&format!("Error restaurant_by_category: {}", e)).await;
         BTreeMap::<i32, String>::new()
      }
   }
}

// Возвращает список блюд указанного ресторана и группы
pub async fn dishes_by_restaurant_and_group(rest_num: i32, group_num: i32) -> Option<DishListWithGroupInfo> {
   // Выполняем запрос информации о группе
   let rows = DB.get().unwrap()
      .query("SELECT info FROM groups WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER", &[&rest_num, &group_num])
      .await;

   match rows {
      Ok(data) => {
         if !data.is_empty() {
            // Информация о группе
            let res = DishListWithGroupInfo{
               info: data[0].get(0),
               dishes: subselect_dishes(rest_num, group_num).await,
            };

            // Окончательный результат
            Some(res)
         } else {
            None
         }
      }
      _ => None,
   }
}

// Возвращает категорию указанной группы
pub async fn category_by_restaurant_and_group(rest_num: i32, group_num: i32) -> i32 {
   // Выполняем запрос информации о группе
   let rows = DB.get().unwrap()
      .query("SELECT cat_id FROM groups WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER", &[&rest_num, &group_num])
      .await;

   match rows {
      Ok(data) => {
         if !data.is_empty() { data[0].get(0) }
         else { 0 }
      }
      _ => 0,
   }
}

// Возвращает информацию о блюде - картинку, цену и описание.
pub async fn eater_dish_info(rest_num: i32, group_num: i32, dish_num: i32) -> Option<(String, Option<String>)> {
   // Выполняем запрос
   let rows = DB.get().unwrap()
      .query("SELECT title, info, price, image_id FROM dishes WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num=$3::INTEGER AND active = TRUE", &[&rest_num, &group_num, &dish_num])
      .await;

   // Проверяем результат
    match rows {
      Ok(data) => {
          if !data.is_empty() {
              // Параметры ресторана
              let title: String = data[0].get(0);
              let info: String = data[0].get(1);
              let price: i32 = data[0].get(2);
              let image_id: Option<String> = data[0].get(3);
              
              // Если описание слишком короткое, не выводим его
              let info_str = if info.len() < 3 {
                 String::default()
              } else {
                 format!("Информация: {}\n", info)
              };
              
              Some((String::from(format!("Название: {}\n{}Цена: {}", title, info_str, price_with_unit(price))), image_id))
          } else {
            None
          }
      }
      _ => None,
   }
}


// Регистрация или разблокировка ресторатора
pub async fn register_caterer(user_id: i32) -> bool {
   // Попробуем разблокировать пользователя, тогда получим 1 в качестве обновлённых записей
   let query = DB.get().unwrap()
   .execute("UPDATE restaurants SET enabled = TRUE WHERE user_id=$1::INTEGER", &[&user_id])
   .await;

   if let Ok(res) = query {
      if res > 0 {
         return true;
      }
   }
   
   // Создаём новый ресторан
   let query = DB.get().unwrap()
   .execute("INSERT INTO restaurants (user_id, title, info, active, enabled) VALUES ($1::INTEGER, 'Му-му', 'Наш адрес 00NDC, доставка @nick, +84123', FALSE, TRUE)", &[&user_id])
   .await;
   
   match query {
      Ok(_) => true,
      _ => false,
   }
}


// Приостановка доступа ресторатора
pub async fn hold_caterer(user_id: i32) -> Result<(), ()> {
   // Блокируем пользователя
   let query = DB.get().unwrap()
   .execute("UPDATE restaurants SET enabled = FALSE, active = FALSE WHERE user_id=$1::INTEGER", &[&user_id])
   .await;

   // Если обновили 0 строк, значит такого пользователя не было зарегистрировано
   match query {
      Ok(1) => Ok(()),
      _ => Err(()),
   }
}


// Возвращает список ресторанов
pub async fn restaurant_list() -> String {
   // Выполняем запрос информации о ресторане
   let rows = DB.get().unwrap()
      .query("SELECT rest_num, user_id, title, enabled FROM restaurants ORDER BY rest_num", &[])
      .await;

   match rows {
      Ok(data) => {
         // Строка для возврата результата
         let mut res = String::default();

         for record in data {
            let rest_num: i32 = record.get(0);
            let user_id: i32 = record.get(1);
            let title: String = record.get(2);
            let enabled: bool = record.get(3);
            res.push_str(&format!("{} '{}', {} {}{}\n", 
                rest_num, title, enabled_to_str(enabled), enabled_to_cmd(enabled), user_id
            ));
        }
        res
      }
      _ => String::from(lang::t("ru", lang::Res::DatabaseEmpty)),
   }
}


// Возвращает список ресторанов с командой для входа
pub async fn restaurant_list_sudo() -> String {
   // Выполняем запрос информации о ресторане
   let rows = DB.get().unwrap()
      .query("SELECT rest_num, user_id, title FROM restaurants ORDER BY rest_num", &[])
      .await;

   match rows {
      Ok(data) => {
         // Строка для возврата результата
         let mut res = String::default();

         for record in data {
            let rest_num: i32 = record.get(0);
            let user_id: i32 = record.get(1);
            let title: String = record.get(2);
            res.push_str(&format!("{} '{}' /sudo{}\n", 
                user_id, title, rest_num
            ));
        }
        res
      }
      _ => String::from(lang::t("ru", lang::Res::DatabaseEmpty)),
   }
}


// Возвращает настройку пользователя и временную отметку последнего входа
pub async fn user_compact_interface(user: Option<&User>, dt: NaiveDateTime) -> bool {
   if let Some(u) = user {
      // Выполняем запрос на обновление
      let query = DB.get().unwrap()
      .execute("UPDATE users SET last_seen = $1::TIMESTAMP WHERE user_id=$2::INTEGER", &[&dt, &u.id])
      .await;

      // Если обновили 0 записей, вставим новую
      if let Ok(num) = query {
         if num == 0 {
            // Информация о пользователе
            let name = if let Some(last_name) = &u.last_name {
               format!("{} {}", u.first_name, last_name)
            } else {u.first_name.clone()};

            let contact = if let Some(username) = &u.username {
               format!(" @{}", username)
            } else {String::from("-")};

            let query = DB.get().unwrap()
            .execute("INSERT INTO users (user_id, user_name, contact, address, last_seen, compact, pickup) VALUES ($1::INTEGER, $2::VARCHAR(100), $3::VARCHAR(100), '-', $4::TIMESTAMP, FALSE, TRUE)"
               , &[&u.id, &name, &contact, &dt])
            .await;

            if let Err(e) = query {
               log(&format!("Error insert last seen record for {}\n{}", name, e)).await;
            }
         } else {
            // Раз обновление было успешным, прочитаем настройку
            let rows = DB.get().unwrap()
            .query("SELECT compact FROM users WHERE user_id=$1::INTEGER", &[&u.id])
            .await;
      
            match rows {
               Ok(data) => {
                  if !data.is_empty() {
                     return data[0].get(0);
                  }
               }
               Err(e) => log(&format!("Error reading interface settings: {}", e)).await,
            }
         }
      }
   }
         
   // Возвращаем значение по-умолчанию
   false
}

// Переключает режим интерфейса
pub async fn user_toggle_interface(user: Option<&User>) {
   if let Some(u) = user {
      let query = DB.get().unwrap()
      .execute("UPDATE users SET compact = NOT compact WHERE user_id=$1::INTEGER", &[&u.id])
      .await;

      // Если произошлa ошибка, сообщим о ней
      if let Err(e) = query {
         log(&format!("Error toggle interface settings: {}", e)).await;
      }
   }
}

// Информация о пользователе для корзины
pub struct UserBasketInfo {
   pub name: String, 
   pub contact: String, 
   pub address: String, 
   pub pickup: bool,
}

impl UserBasketInfo {
   // Возвращает либо сам адрес либо надпись, что задана точка
   pub fn address_label(&self) -> String {
      // Если адрес начинается с ключевого слова, значит там id сообщения с локацией
      if "Location" == self.address.get(..8).unwrap_or_default() {String::from("на карте")} else {self.address.clone()}
   }

   // Возвращает id сообщения с локацией, если имеется
   pub fn address_message_id(&self) -> Option<i32> {
      if "Location" == self.address.get(..8).unwrap_or_default() {
         // Пытаемся получить продолжение строки
         if let Some(s) = self.address.get(8..) {
            // Пытаемся преобразовать в число.
            if let Ok(res) = s.parse::<i32>() {Some(res)} else {None}
         } else {None}
      } else {None}
   } 
}

pub async fn user_basket_info(user_id: i32) -> Option<UserBasketInfo> {
   let query = DB.get().unwrap()
   .query("SELECT user_name, contact, address, pickup from users WHERE user_id=$1::INTEGER", &[&user_id])
   .await;

   match query {
      Ok(data) => {
         if !data.is_empty() {
            return Some(UserBasketInfo {
               name: data[0].get(0),
               contact: data[0].get(1),
               address: data[0].get(2),
               pickup: data[0].get(3),
            });
         }
      }
      // Если произошл ошибка, сообщим о ней
      Err(e) => log(&format!("Error toggle interface settings: {}", e)).await,
   }
   
   // Если попали сюда, значит была ошибка
   None
}

// ============================================================================
// [Misc]
// ============================================================================
// Для отображения статуса
//
fn active_to_str(active : bool) -> &'static str {
   if active {
       "показывается"
   } else {
       "скрыт"
   }
}

fn enabled_to_str(enabled : bool) -> &'static str {
   if enabled {
       "доступен"
   } else {
       "в бане"
   }
}

fn enabled_to_cmd(enabled : bool) -> &'static str {
   if enabled {
       "/hold"
   } else {
       "/regi"
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

// Используется при редактировании категории группы
//
pub fn id_to_category(cat_id : i32) -> &'static str {
   match cat_id {
      1 => "Соки воды",
      2 => "Еда",
      3 => "Алкоголь",
      4 => "Развлечения",
      _ => "Неизвестная категория",
   }
} 

pub fn category_to_id(category: &str) -> i32 {
   match category {
      "Соки воды" => 1,
      "Еда" => 2,
      "Алкоголь" => 3,
      "Развлечения" => 4,
      _ => 0,
   }
}

// Режим интерфейса
//
pub fn interface_mode(is_compact: bool) -> String {
   if is_compact {
      String::from("со ссылками")
   } else {
      String::from("с кнопками")
   }
}

// Возвращает истину, если user_id принадлежит администратору
//
pub fn is_admin(user_id: Option<&teloxide::types::User>) -> bool {
   match user_id { 
      Some(user) => {
         let (admin1, admin2, admin3) = *TELEGRAM_ADMIN_ID.get().unwrap();
         let test = user.id;
         test == admin1 || test == admin2 || test == admin3
      }
      None => false,
   }
}

// Проверяет существование таблиц
//
pub async fn is_tables_exist() -> bool {
   // Выполняем запрос
   let rows = DB.get().unwrap()
      .query("SELECT * FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_NAME='restaurants'", &[]).await;

   // Проверяем результат
   match rows {
      Ok(data) => !data.is_empty(),
      _ => false,
   }
}

// Создаёт новые таблицы
pub async fn create_tables() -> bool {
   // Таблица с данными о ресторанах
   let query = DB.get().unwrap()
   .execute("CREATE TABLE restaurants (
      PRIMARY KEY (user_id),
      user_id     INTEGER       NOT NULL,
      title       VARCHAR(100)  NOT NULL,
      info        VARCHAR(255)  NOT NULL,
      active      BOOLEAN       NOT NULL,
      enabled     BOOLEAN       NOT NULL,
      rest_num    SERIAL,
      image_id    VARCHAR(255))", &[])
   .await;
   
   match query {
      Ok(_) => {
         // Таблица с данными о группах
         let query = DB.get().unwrap()
         .execute("CREATE TABLE groups (
            PRIMARY KEY (rest_num, group_num),
            rest_num        INTEGER         NOT NULL,
            group_num       INTEGER         NOT NULL,
            title           VARCHAR(100)    NOT NULL,
            info            VARCHAR(255)    NOT NULL,
            active          BOOLEAN         NOT NULL,
            cat_id          INTEGER         NOT NULL,
            opening_time    TIME            NOT NULL,    
            closing_time    TIME            NOT NULL)", &[])
         .await;
         
         match query {
            Ok(_) => {
               // Таблица с данными о блюдах
               let query = DB.get().unwrap()
               .execute("CREATE TABLE dishes (
                  PRIMARY KEY (rest_num, group_num, dish_num),
                  rest_num       INTEGER        NOT NULL,
                  dish_num       INTEGER        NOT NULL,
                  title          VARCHAR(100)   NOT NULL,
                  info           VARCHAR(255)   NOT NULL,
                  active         BOOLEAN        NOT NULL,
                  group_num      INTEGER        NOT NULL,
                  price          INTEGER        NOT NULL,
                  image_id       VARCHAR(255))", &[])
               .await;
               
               match query {
                  Ok(_) => {
                     // Таблица с данными о едоках
                     let query = DB.get().unwrap()
                     .execute("CREATE TABLE users (
                        PRIMARY KEY (user_id),
                        user_id     INTEGER        NOT NULL,
                        user_name   VARCHAR(100)   NOT NULL,
                        contact     VARCHAR(100)   NOT NULL,
                        address     VARCHAR(100)   NOT NULL,
                        last_seen   TIMESTAMP      NOT NULL,
                        compact     BOOLEAN        NOT NULL,
                        pickup      BOOLEAN        NOT NULL)", &[])
                     .await;
                     
                     match query {
                        Ok(_) => {
                           // Таблица с данными о заказах
                           let query = DB.get().unwrap()
                           .execute("CREATE TABLE orders (
                              PRIMARY KEY (user_id, rest_num, group_num, dish_num),
                              user_id     INTEGER        NOT NULL,
                              rest_num    INTEGER        NOT NULL,
                              group_num   INTEGER        NOT NULL,
                              dish_num    INTEGER        NOT NULL,
                              amount      INTEGER        NOT NULL)", &[])
                           .await;
                           
                           match query {
                              Ok(_) => {
                                 true
                              }
                              _ => false,
                           }
                        }
                        _ => false,
                     }
                  }
                  _ => false,
               }
            }
            _ => false,
         }
      }
      _ => false,
   }
}

// Формирование ключа блюда на основе аргументов
pub fn make_key_3_int(first: i32, second: i32, third: i32) -> String {
   format!("{}_{}_{}", first, second, third)
}

// Разбор строки на три числа, например ключа блюда на аргументы
pub fn parse_key_3_int(text: &str) -> Result<(i32, i32, i32), Box<dyn std::error::Error>> {
   let first: i32;
   let second: i32;
   let third: i32;

   try_scan!(text.bytes() => "{}_{}_{}", first, second, third);

   Ok((first, second, third))
}

// Хранит данные для работы логирования в чат
pub struct ServiceChat {
   pub id: i64,
   pub bot: std::sync::Arc<Bot>,
}

// Отправляет сообщение в телеграм группу для лога
//
impl ServiceChat {
   // Непосредственно отправляет сообщение
   async fn send(&self, text: &str, silence: bool) {
      if let Err(err) = self.bot.send_message(self.id, text).disable_notification(silence).send().await {
         log::info!("Error log({}): {}", text, err);
      }
   }
}

// Отправляет в служебный чат сообщение в молчаливом режиме
pub async fn log(text: &str) {
   if let Some(chat) = TELEGRAM_LOG_CHAT.get() {
      chat.send(text, true).await;
   }
}

// Отправляет в служебный чат сообщение с уведомлением
pub async fn log_and_notify(text: &str) {
   if let Some(chat) = TELEGRAM_LOG_CHAT.get() {
      chat.send(text, false).await;
   }
}

// Пересылает в служебный чат сообщение
pub async fn log_forward(from_chat: ChatId, message_id: i32) {
   if let Some(chat) = TELEGRAM_LOG_CHAT.get() {
      if let Err(e) = chat.bot.forward_message(chat.id, from_chat, message_id).send().await {
         log::info!("Error log_forward(): {}", e);
      }
   }
}

// Формирование информации о пользователе для лога
//
fn user_info_optional_part(user: &User) -> String {
   // Строка для возврата результата
   let mut s = String::default();

   if let Some(last_name) = &user.last_name {
      s.push_str(&format!(" {}", last_name));
   }
   if let Some(username) = &user.username {
      s.push_str(&format!(" @{}", username));
   }
   if let Some(language_code) = &user.language_code {
      s.push_str(&format!(" lang={}", language_code));
   }
   s
}

pub fn user_info(user: Option<&User>, detail: bool) -> String {
   if let Some(u) = user {
      let mut s = format!("{}:{}", u.id, u.first_name);

      // Эту информацию выводим только для полного описания
      if detail {
         s.push_str(&user_info_optional_part(u));
      }
      s
   } else {
      String::from("None user info")
   }
}

// Форматирование цены с единицей измерения
pub fn price_with_unit(price: i32) -> String {
   let unit = match PRICE_UNIT.get() {
      Some(data) => data,
      None => "",
   };

   format!("{}{}", price, unit)
}

pub fn default_photo_id() -> String { 
   match DEFAULT_IMAGE_ID.get() {
      Some(image) => image.clone(),
      None => String::from(""),
   }
}

// Изменение имени пользователя
pub async fn basket_edit_name(user_id: i32, s: String) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE users SET user_name = $1::VARCHAR(100) WHERE user_id=$2::INTEGER", &[&s, &user_id])
   .await;
   match query {
       Ok(_) => true,
       Err(e) => {
         log(&format!("Error db::basket_edit_name: {}", e)).await;
         false
       }
   }
}

// Изменение имени пользователя
pub async fn basket_edit_contact(user_id: i32, s: String) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE users SET contact = $1::VARCHAR(100) WHERE user_id=$2::INTEGER", &[&s, &user_id])
   .await;
   match query {
       Ok(_) => true,
       Err(e) => {
         log(&format!("Error db::basket_edit_contact: {}", e)).await;
         false
       }
   }
}

// Изменение имени пользователя
pub async fn basket_edit_address(user_id: i32, s: String) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE users SET address = $1::VARCHAR(100) WHERE user_id=$2::INTEGER", &[&s, &user_id])
   .await;
   match query {
       Ok(_) => true,
       Err(e) => {
         log(&format!("Error db::basket_edit_address: {}", e)).await;
         false
       }
   }
}

// Изменение имени пользователя
pub async fn basket_toggle_pickup(user_id: i32) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE users SET pickup = NOT pickup WHERE user_id=$1::INTEGER", &[&user_id])
   .await;
   match query {
       Ok(_) => true,
       Err(e) => {
         log(&format!("Error db::basket_toggle_pickup: {}", e)).await;
         false
       }
   }
}

// ============================================================================
// [Caterer]
// ============================================================================

// Возвращает номер ресторана, если пользователю разрешён доступ в режим ресторатора
//
pub async fn rest_num(user : Option<&teloxide::types::User>) -> Result<i32, ()> {
   // Проверяем, передан ли пользователь.
   let u = user.ok_or(())?;

   // Выполняем запрос
   let rows = DB.get().unwrap()
      .query("SELECT rest_num FROM restaurants WHERE user_id=$1::INTEGER AND enabled = TRUE", &[&u.id])
      .await;

   // Возвращаем номер ресторана, если такой есть.
   match rows {
      Ok(data) => {
         if data.is_empty() {
            Err(()) 
         } else {
            Ok(data[0].get(0))
         }
      }
      _ => Err(()),
   }
}


// Возвращает строку с информацией о ресторане
//
pub async fn rest_info(rest_num: i32) -> Option<(String, Option<String>)> {
   // Выполняем запрос
   let rows = DB.get().unwrap()
      .query("SELECT title, info, active, image_id FROM restaurants WHERE rest_num=$1::INTEGER", &[&rest_num])
      .await;

   // Проверяем результат
   match rows {
      Ok(data) => {
         if !data.is_empty() {
               // Параметры ресторана
               let title: String = data[0].get(0);
               let info: String = data[0].get(1);
               let active: bool = data[0].get(2);
               let image_id: Option<String> = data[0].get(3);
               let groups: String = group_titles(rest_num).await;
               Some((
                  String::from(format!("Название: {} /EditTitle\nОписание: {} /EditInfo\nСтатус: {} /Toggle\nЗагрузить фото /EditImg\nГруппы и время работы (добавить новую /AddGroup):\n{}",
                     title, info, active_to_str(active), groups)
                  ), image_id
               ))
         } else {
               None
         }
      }
      _ => None,
   }
}

pub async fn rest_edit_title(rest_num: i32, new_str: String) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE restaurants SET title = $1::VARCHAR(100) WHERE rest_num=$2::INTEGER", &[&new_str, &rest_num])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}

pub async fn rest_edit_info(rest_num: i32, new_str: String) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE restaurants SET info = $1::VARCHAR(255) WHERE rest_num=$2::INTEGER", &[&new_str, &rest_num])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}

pub async fn rest_toggle(rest_num: i32) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE restaurants SET active = NOT active WHERE rest_num=$1::INTEGER", &[&rest_num])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}

// Изменение фото ресторана
//
pub async fn rest_edit_image(rest_num: i32, image_id: &String) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE restaurants SET image_id = $1::VARCHAR(255) WHERE rest_num=$2::INTEGER", &[&image_id, &rest_num])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}

// Изменяет владельца ресторана
//
pub async fn transfer_ownership(rest_num: i32, new_user_id: i32) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE restaurants SET user_id = $1::INTEGER WHERE rest_num=$2::INTEGER", &[&new_user_id, &rest_num])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}


// ============================================================================
// [Group]
// ============================================================================

// Возвращает строки с краткой информацией о группах
//
async fn group_titles(rest_num: i32) -> String {
    // Выполняем запрос
    let rows = DB.get().unwrap()
        .query("SELECT group_num, title, opening_time, closing_time FROM groups WHERE rest_num=$1::INTEGER", &[&rest_num])
        .await;

    // Строка для возврата результата
    let mut res = String::default();

    // Проверяем результат
    if let Ok(data) = rows {
        for record in data {
            let group_num: i32 = record.get(0);
            let title: String = record.get(1);
            let opening_time: NaiveTime = record.get(2);
            let closing_time: NaiveTime = record.get(3);
            res.push_str(&format!("   {} {}-{} /EdGr{}\n", 
                title, opening_time.format("%H:%M"), closing_time.format("%H:%M"), group_num
            ));
        }
    }
    res
}


// Возвращает информацию о группе
//
pub async fn group_info(rest_num: i32, group_num: i32) -> Option<String> {
   // Выполняем запрос
   let rows = DB.get().unwrap()
   .query("SELECT title, info, active, cat_id, opening_time, closing_time FROM groups WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER", &[&rest_num, &group_num])
   .await;

   // Проверяем результат
   match rows {
      Ok(data) => {
         if !data.is_empty() {
               // Параметры ресторана
               let title: String = data[0].get(0);
               let info: String = data[0].get(1);
               let active: bool = data[0].get(2);
               let cat_id: i32 = data[0].get(3);
               let opening_time: NaiveTime = data[0].get(4);
               let closing_time: NaiveTime = data[0].get(5);
               let dishes: String = dish_titles(rest_num, group_num).await;
               Some(
                  String::from(format!("Название: {} /EditTitle\nДоп.инфо: {} /EditInfo\nКатегория: {} /EditCat\nСтатус: {} /Toggle\nВремя: {}-{} /EditTime\nУдалить группу /Remove\nНовое блюдо /AddDish\n{}",
                  title, info, id_to_category(cat_id), active_to_str(active), opening_time.format("%H:%M"), closing_time.format("%H:%M"), dishes))
               )
         } else {
               None
         }
      }
      _ => None,
   }
}

// Добавляет новую группу
//
pub async fn rest_add_group(rest_num: i32, new_str: String) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("INSERT INTO groups (rest_num, group_num, title, info, active, cat_id, opening_time, closing_time) 
   VALUES (
      $1::INTEGER, 
      (SELECT COUNT(*) FROM groups WHERE rest_num=$2::INTEGER) + 1,
      $3::VARCHAR(100),
      'Блюда подаются на тарелке',
      TRUE,
      2,
      '07:00',
      '23:00'
   )", &[&rest_num, &rest_num, &new_str])
   .await;
   match query {
      Ok(_) => true,
      _ => false,
   }
}

// Изменяет название группы
//
pub async fn rest_group_edit_title(rest_num: i32, group_num: i32, new_str: String) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE groups SET title = $1::VARCHAR(100) WHERE rest_num=$2::INTEGER AND group_num=$3::INTEGER", &[&new_str, &rest_num, &group_num])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}

// Изменяет описание группы
//
pub async fn rest_group_edit_info(rest_num: i32, group_num: i32, new_str: String) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE groups SET info = $1::VARCHAR(255) WHERE rest_num=$2::INTEGER AND group_num=$3::INTEGER", &[&new_str, &rest_num, &group_num])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}

// Переключает доступность группы
//
pub async fn rest_group_toggle(rest_num: i32, group_num: i32) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE groups SET active = NOT active WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER", &[&rest_num, &group_num])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}

// Изменяет категорию группы
//
pub async fn rest_group_edit_category(rest_num: i32, group_num: i32, new_cat : i32) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE groups SET cat_id = $1::INTEGER WHERE rest_num=$2::INTEGER AND group_num=$3::INTEGER", &[&new_cat, &rest_num, &group_num])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}

// Изменяет время доступности группы
//
pub async fn rest_group_edit_time(rest_num: i32, group_num: i32, opening_time: NaiveTime, closing_time: NaiveTime) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE groups SET opening_time = $1::TIME, closing_time = $2::TIME WHERE rest_num=$3::INTEGER AND group_num=$4::INTEGER", &[&opening_time, &closing_time, &rest_num, &group_num])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}

// Удаляет группу и изменяет порядковый номер оставшихся групп, в т.ч. и у блюд
//
pub async fn rest_group_remove(rest_num: i32, group_num: i32) -> bool {
   // Проверим, что у группы нет блюд
   let rows = DB.get().unwrap()
   .query("SELECT * FROM dishes WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER", &[&rest_num, &group_num])
   .await;
   if let Ok(res) = rows {
      if !res.is_empty() {
         return false;
      }
   } else {
      return false;
   }

    // Выполняем запрос. Должно быть начало транзакции, потом коммит, но transaction требует mut
   let query = DB.get().unwrap()
   .execute("DELETE FROM groups WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER", &[&rest_num, &group_num])
   .await;
   match query {
      Ok(_) => {
         // Номера групп перенумеровываем для исключения дырки
         let query = DB.get().unwrap()
         .execute("UPDATE groups SET group_num = group_num - 1 WHERE rest_num=$1::INTEGER AND group_num>$2::INTEGER", &[&rest_num, &group_num])
         .await;
         match query {
            Ok(_) => {
               // Перенумеровываем группы у блюд
               let query = DB.get().unwrap()
               .execute("UPDATE dishes SET group_num = group_num - 1 WHERE rest_num=$1::INTEGER AND group_num>$2::INTEGER", &[&rest_num, &group_num])
               .await;
               match query {
                  Ok(_) => true,
                  _ => false,
               }
            }
            _     => false,
         }
      }
      _ => false,
   }
}


// ============================================================================
// [Dish]
// ============================================================================

// Возвращает строки с краткой информацией о блюдах
//
async fn dish_titles(rest_num: i32, group_num: i32) -> String {
    // Выполняем запрос
    let rows = DB.get().unwrap()
        .query("SELECT dish_num, title, price FROM dishes WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER ORDER BY dish_num", &[&rest_num, &group_num])
        .await;

    // Строка для возврата результата
    let mut res = String::default();

    // Проверяем результат
    if let Ok(data) = rows {
        for record in data {
            let dish_num: i32 = record.get(0);
            let title: String = record.get(1);
            let price: i32 = record.get(2);
            res.push_str(&format!("   {} {} /EdDi{}\n", 
                title, price_with_unit(price), dish_num
            ));
        }
    }
    res
}


// Возвращает информацию о блюде
//
pub async fn dish_info(rest_num: i32, group_num: i32, dish_num: i32) -> Option<(String, Option<String>)> {
     // Выполняем запрос
     let rows = DB.get().unwrap()
     .query("SELECT title, info, active, group_num, price, image_id FROM dishes WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num=$3::INTEGER", &[&rest_num, &group_num, &dish_num])
     .await;

    // Проверяем результат
    match rows {
        Ok(data) => {
            if !data.is_empty() {
                // Параметры ресторана
                let title: String = data[0].get(0);
                let info: String = data[0].get(1);
                let active: bool = data[0].get(2);
                let group_num: i32 = data[0].get(3);
                let price: i32 = data[0].get(4);
                let image_id: Option<String> = data[0].get(5);
                Some((
                  String::from(format!("Название: {} /EditTitle\nДоп.инфо: {} /EditInfo\nГруппа: {} /EditGroup\nСтатус: {} /Toggle\nЦена: {} /EditPrice\nЗагрузить фото /EditImg\nУдалить блюдо /Remove",
                  title, info, group_num, active_to_str(active), price_with_unit(price))), image_id
               ))
            } else {
                None
            }
        }
        _ => None,
    }
}


// Добавляет новое блюдо
//
pub async fn rest_add_dish(rest_num: i32, group_num: i32, new_str: String) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("INSERT INTO dishes (rest_num, dish_num, title, info, active, group_num, price) 
   VALUES (
      $1::INTEGER, 
      (SELECT COUNT(*) FROM dishes WHERE rest_num = $2::INTEGER AND group_num = $3::INTEGER) + 1,
      $4::VARCHAR(100),
      'Порция 100гр.',
      TRUE,
      $5::INTEGER,
      0
   )", &[&rest_num, &rest_num, &group_num, &new_str, &group_num])
   .await;
   match query {
      Ok(_) => true,
      _ => false,
   }
}


// Редактирование названия блюда
//
pub async fn rest_dish_edit_title(rest_num: i32, group_num: i32, dish_num: i32, new_str: String) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE dishes SET title = $1::VARCHAR(100) WHERE rest_num=$2::INTEGER AND group_num=$3::INTEGER AND dish_num=$4::INTEGER", &[&new_str, &rest_num, &group_num, &dish_num])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}


// Редактирование описания блюда
//
pub async fn rest_dish_edit_info(rest_num: i32, group_num: i32, dish_num: i32, new_str: String) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE dishes SET info = $1::VARCHAR(255) WHERE rest_num=$2::INTEGER AND group_num=$3::INTEGER AND dish_num=$4::INTEGER", &[&new_str, &rest_num, &group_num, &dish_num])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}


// Переключение доступности блюда
//
pub async fn rest_dish_toggle(rest_num: i32, group_num: i32, dish_num: i32) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE dishes SET active = NOT active WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num=$3::INTEGER", &[&rest_num, &group_num, &dish_num])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}


// Изменение группы блюда
//
pub async fn rest_dish_edit_group(rest_num: i32, old_group_num: i32, dish_num: i32, new_group_num: i32) -> bool {
   // Проверим, что есть такая целевая группа
   let rows = DB.get().unwrap()
   .query("SELECT * FROM groups WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER", &[&rest_num, &new_group_num])
   .await;

   // Если целевой группы нет, выходим
   match rows {
      Ok(data) => {
         if data.is_empty() {
            return false;
         }
      }
      _ => return false
   }

   // Сохраним информацию о блюде
   let rows = DB.get().unwrap()
   .query("SELECT title, info, active, price, image_id FROM dishes WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num=$3::INTEGER", &[&rest_num, &old_group_num, &dish_num])
   .await;
   match rows {
      Ok(data) => {
         if data.is_empty() {
            return false;
         } else {
            let title: String = data[0].get(0);
            let info: String = data[0].get(1);
            let active: bool = data[0].get(2);
            let price: i32 = data[0].get(3);
            let image_id: Option<String> = data[0].get(4);

            // Добавляем блюдо в целевую группу
            let query = DB.get().unwrap()
            .execute("INSERT INTO dishes (rest_num, dish_num, title, info, active, group_num, price, image_id) 
            VALUES (
               $1::INTEGER, 
               (SELECT COUNT(*) FROM dishes WHERE rest_num = $2::INTEGER AND group_num = $3::INTEGER) + 1,
               $4::VARCHAR(100),
               $5::VARCHAR(255),
               $6::BOOLEAN,
               $7::INTEGER,
               $8::INTEGER,
               $9::VARCHAR(255)
            )", &[&rest_num, &rest_num, &new_group_num, &title, &info, &active, &new_group_num, &price, &image_id])
            .await;
            match query {
               Ok(inserted_num) => {
                  if inserted_num < 1 {
                     return false;
                  }
               }
               _ => return false
            }

            // Удалим блюдо из прежней группы
            rest_dish_remove(rest_num, old_group_num, dish_num).await
         }
      }
      _ => return false
   }
}


// Удаление блюда
//
pub async fn rest_dish_remove(rest_num: i32, group_num: i32, dish_num: i32) -> bool {
   // Выполняем запрос. Должно быть начало транзакции, потом коммит, но transaction требует mut
   let query = DB.get().unwrap()
   .execute("DELETE FROM dishes WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num=$3::INTEGER", &[&rest_num, &group_num, &dish_num])
   .await;
   match query {
      Ok(_) => {
         // Номера оставшихся блюд перенумеровываем для исключения дырки
         let query = DB.get().unwrap()
         .execute("UPDATE dishes SET dish_num = dish_num - 1 WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num > $3::INTEGER", &[&rest_num, &group_num, &dish_num])
         .await;
         match query {
            _ => {
               // Удаляем блюдо из заказов пользователей
               dish_remove_from_orders(rest_num, group_num, dish_num).await;
               true
            }
         }
      }
      _ => false,
   }
 }


// Удаление блюда из заказов пользователей
//
async fn dish_remove_from_orders(rest_num: i32, group_num: i32, dish_num: i32) {
   // Удалим блюдо из корзины всех пользователей
   let query = DB.get().unwrap()
   .execute("DELETE FROM orders WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num=$3::INTEGER", &[&rest_num, &group_num, &dish_num])
   .await;
   match query {
      Ok(_) => {
         // Обновим номера блюд в корзине согласно перенумерации самих блюд
         let query = DB.get().unwrap()
         .execute("UPDATE orders SET dish_num = dish_num - 1 WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num > $3::INTEGER", &[&rest_num, &group_num, &dish_num])
         .await;
         if let Err(_) = query {
               // Сообщим об ошибке
               log::info!("Error dish_remove_from_orders while recounting {}", make_key_3_int(rest_num, group_num, dish_num));
         }
      }
      Err(_) => {
         // Сообщим об ошибке
         log::info!("Error dish_remove_from_orders {}", make_key_3_int(rest_num, group_num, dish_num));
      }
   }
}


// Изменение цены блюда
//
pub async fn rest_dish_edit_price(rest_num: i32, group_num: i32, dish_num: i32, price: i32) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE dishes SET price = $1::INTEGER WHERE rest_num=$2::INTEGER AND group_num=$3::INTEGER AND dish_num=$4::INTEGER", &[&price, &rest_num, &group_num, &dish_num])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}


// Изменение фото блюда
//
pub async fn rest_dish_edit_image(rest_num: i32, group_num: i32, dish_num: i32, image_id: &String) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE dishes SET image_id = $1::VARCHAR(100) WHERE rest_num=$2::INTEGER AND group_num=$3::INTEGER AND dish_num=$4::INTEGER", &[&image_id, &rest_num, &group_num, &dish_num])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}

// Возвращает количество порций блюда в корзине
//
pub async fn amount_in_basket(rest_num: i32, group_num: i32, dish_num: i32, user_id: i32) -> i32 {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .query("SELECT amount FROM orders WHERE user_id=$1::INTEGER AND  rest_num=$2::INTEGER AND group_num=$3::INTEGER AND dish_num=$4::INTEGER", &[&user_id, &rest_num, &group_num, &dish_num])
   .await;

   match query {
      Ok(data) => if !data.is_empty() {
         data[0].get(0)
      } else {
         0
      }
      _ => 0,
   }
}

// Добавляет блюдо в корзину, возвращая новое количество
//
pub async fn add_dish_to_basket(rest_num: i32, group_num: i32, dish_num: i32, user_id: i32) -> Result<i32, ()> {
   // Текущее количество экземпляров в корзине
   let old_amount = amount_in_basket(rest_num, group_num, dish_num, user_id).await;

   // Если такая запись уже есть, надо увеличить на единицу количество, иначе создать новую запись
   if old_amount > 0 {
      // Выполняем запрос
      let query = DB.get().unwrap()
      .execute("UPDATE orders SET amount = amount + 1 WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num=$3::INTEGER AND user_id=$4::INTEGER", &[&rest_num, &group_num, &dish_num, &user_id])
      .await;
      match query {
         Ok(_) => Ok(old_amount + 1),
         _ => Err(()),
      }
   } else {
      // Выполняем запрос
      let query = DB.get().unwrap()
      .execute("INSERT INTO orders (rest_num, group_num, dish_num, user_id, amount) 
         VALUES ($1::INTEGER, $2::INTEGER, $3::INTEGER, $4::INTEGER, 1)", &[&rest_num, &group_num, &dish_num, &user_id])
      .await;
      match query {
         Ok(_) => Ok(1),
         _ => Err(()),
      }
   }
}


// Удаляет блюдо из корзины
//
pub async fn remove_dish_from_basket(rest_num: i32, group_num: i32, dish_num: i32, user_id: i32) -> Result<i32, ()> {
   // Текущее количество экземпляров в корзине
   let old_amount = amount_in_basket(rest_num, group_num, dish_num, user_id).await;

   // Если остался только один экземпляр или меньше, удаляем запись, иначе редактируем.
   if old_amount > 1 {
      // Выполняем запрос
      let query = DB.get().unwrap()
      .execute("UPDATE orders SET amount = amount - 1 WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num=$3::INTEGER AND user_id=$4::INTEGER", &[&rest_num, &group_num, &dish_num, &user_id])
      .await;
      match query {
         Ok(_) => Ok(old_amount - 1),
         _ => Err(()),
      }
   } else {
      // Выполняем запрос
      let query = DB.get().unwrap()
      .execute("DELETE FROM orders WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num=$3::INTEGER AND user_id=$4::INTEGER", &[&rest_num, &group_num, &dish_num, &user_id])
      .await;
      match query {
         Ok(_) => Ok(0),
         _ => Err(()),
      }
   }
}


// Содержимое корзины
//
pub struct Basket {
   pub rest_id: i32,
   pub restaurant: String,
   pub dishes: Vec<String>,
   pub total: i32,
}

// Возвращает содержимое корзины и итоговую сумму заказа
//
pub async fn basket_contents(user_id: i32) -> (Vec<Basket>, i32) {
   // Для возврата результата
   let mut res = Vec::<Basket>::new();
   let mut grand_total: i32 = 0;

   // Выберем все упомянутые рестораны
   let rows = DB.get().unwrap()
      .query("SELECT DISTINCT r.title, r.info, r.rest_num, r.user_id FROM orders as o 
         INNER JOIN restaurants r ON o.rest_num = r.rest_num 
         WHERE o.user_id = $1::INTEGER
         ORDER BY r.rest_num", 
      &[&user_id])
      .await;

   // Двигаемся по каждому ресторану
   if let Ok(data) = rows {
      for record in data {
         // Данные из запроса о ресторане
         let rest_title: String = record.get(0);
         let rest_info: String = record.get(1);
         let rest_num: i32 = record.get(2);
         let rest_id: i32 = record.get(3);

         // Для общей суммы заказа по ресторану
         let mut total: i32 = 0;

         // Теперь заправшиваем информацию о блюдах ресторана
         let rows = DB.get().unwrap()
         .query("SELECT d.title, d.price, o.amount, o.group_num, o.dish_num FROM orders as o 
            INNER JOIN groups g ON o.rest_num = g.rest_num AND o.group_num = g.group_num
            INNER JOIN dishes d ON o.rest_num = d.rest_num AND o.group_num = d.group_num AND o.dish_num = d.dish_num
            WHERE o.user_id = $1::INTEGER AND o.rest_num = $2::INTEGER
            ORDER BY o.group_num, o.dish_num", 
         &[&user_id, &rest_num])
         .await;
   
         // Двигаемся по каждой записи и сохраняем информацию о блюде
         let mut dishes = Vec::<String>::new();
         if let Ok(data) = rows {
            for record in data {
               // Данные из запроса
               let title: String = record.get(0);
               let price: i32 = record.get(1);
               let amount: i32 = record.get(2);
               let group_num: i32 = record.get(3);
               let dish_num: i32 = record.get(4);

               // Добавляем стоимость в итог
               total += price * amount;

               // Помещаем блюдо в список
               dishes.push(format!("{}: {} x {} шт. = {} /del{}", title, price, amount, price_with_unit(price * amount), make_key_3_int(rest_num, group_num, dish_num)));
            }
         }

         // Создаём корзину текущего ресторана
         let basket = Basket {
            rest_id,
            restaurant: format!("{}. {}. {}\n", rest_num, rest_title, rest_info),
            dishes,
            total,
         };

         // Обновляем общий итог
         grand_total += total;

         // Помещаем ресторан в список
         res.push(basket);
      }
   }
   // Возвращаем результат
   (res, grand_total)
}

// Очищает корзину указанного пользователя
//
pub async fn clear_basket(user_id: i32) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("DELETE FROM orders WHERE user_id = $1::INTEGER", &[&user_id])
   .await;
   match query {
      Ok(_) => true,
      _ => false,
   }
}

