/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Модуль для связи с СУБД. 28 May 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use chrono::{NaiveTime, Timelike};
use once_cell::sync::{OnceCell};
use text_io::try_scan;
use teloxide::{
   types::{User, InputFile, },
};
use tokio_postgres::{Row, types::ToSql, };
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

// Информация о ресторане
pub struct Restaurant {
   pub user_id: i32,
   pub title: String,
   pub info: String,
   pub active: bool,
   pub enabled: bool,
   pub num: i32,
   pub image_id: Option<String>,
   pub opening_time: NaiveTime,
   pub closing_time: NaiveTime,
}

impl Restaurant {
   pub fn from_db(row: &Row) -> Self {
      Self {
         user_id: row.get(0),
         title: row.get(1),
         info: row.get(2),
         active: row.get(3),
         enabled: row.get(4),
         num: row.get(5),
         image_id: row.get(6),
         opening_time: row.get(7),
         closing_time: row.get(8),
      }
   }

   // Возвращает собственную картинку или картинку по-умолчанию
   pub fn image_or_default(&self) -> String {
      if let Some(id) = self.image_id.clone() {id}
      else {settings::default_photo_id()}
   }
}

// Тип запроса информации о ресторане
pub enum RestBy {
   Id(i32),    // по user_id
   Num(i32),   // по номеру
}

// Тип запроса списка ресторанов
pub enum RestListBy {
   All,              // все рестораны
   Category(i32),    // активные, с группами в указанной категории
   Time(NaiveTime),  // активные, с группами, работающими в указанное время
}

// Список ресторанов
pub type RestList = Vec<Restaurant>;

// Возвращает список ресторанов
pub async fn rest_list(by: RestListBy) -> Option<RestList> {
   // Получим клиента БД из пула
   let client = db_client().await?;

   // Выберем нужный текст запроса
   let statement_text =  match by {
      RestListBy::All =>
         "SELECT r.user_id, r.title, r.info, r.active, r.enabled, r.rest_num, r.image_id, r.opening_time, r.closing_time FROM restaurants AS r
         ORDER BY rest_num",
      RestListBy::Category(_cat_id) =>
         "SELECT r.user_id, r.title, r.info, r.active, r.enabled, r.rest_num, r.image_id, r.opening_time, r.closing_time FROM restaurants AS r 
            INNER JOIN (SELECT DISTINCT rest_num FROM groups WHERE cat_id=$1::INTEGER AND active = TRUE) g ON r.rest_num = g.rest_num 
            WHERE r.active = TRUE",
      RestListBy::Time(_time) =>
         "SELECT r.user_id, r.title, r.info, r.active, r.enabled, r.rest_num, r.image_id, r.opening_time, r.closing_time FROM restaurants AS r 
            INNER JOIN (SELECT DISTINCT rest_num FROM groups WHERE active = TRUE AND 
            ($1::TIME BETWEEN opening_time AND closing_time) OR (opening_time > closing_time AND $1::TIME > opening_time)) g ON r.rest_num = g.rest_num WHERE r.active = TRUE",
   };

   // Подготовим нужный запрос с кешем благодаря пулу
   let statement = client.prepare(statement_text).await;

   // Если запрос подготовлен успешно, выполняем его
   match statement {
      Ok(stmt) => {
         let rows = match by {
            RestListBy::All => client.query(&stmt, &[]).await,
            RestListBy::Category(cat_id) => client.query(&stmt, &[&cat_id]).await,
            RestListBy::Time(time) => client.query(&stmt, &[&time]).await,
         };

         // Возвращаем результат
         match rows {
            Ok(data) => if data.is_empty() {None} else {Some(data.into_iter().map(|row| (Restaurant::from_db(&row))).collect())},
            Err(e) => {
               // Сообщаем об ошибке и возвращаем пустой результат
               settings::log(&format!("db::rest_list: {}", e)).await;
               None
            }
         }
      }
      Err(e) => {
         // Сообщаем об ошибке и возвращаем пустой результат
         settings::log(&format!("db::rest_list prepare: {}", e)).await;
         None
      }
   }
}

// Возвращает информацию о ресторане
pub async fn restaurant(by: RestBy) -> Option<Restaurant> {
   // Получим клиента БД из пула
   let client = db_client().await?;

   // Подготовим нужный запрос с кешем благодаря пулу
   let statement = match by {
      RestBy::Id(_user_id) => client.prepare("SELECT user_id, title, info, active, enabled, rest_num, image_id, opening_time, closing_time FROM restaurants
         WHERE user_id=$1::INTEGER"),
      RestBy::Num(_rest_num) => client.prepare("SELECT user_id, title, info, active, enabled, rest_num, image_id, opening_time, closing_time FROM restaurants
         WHERE rest_num=$1::INTEGER"),
   }.await;

   // Если запрос подготовлен успешно, выполняем его
   match statement {
      Ok(stmt) => {
         let rows = match by {
            RestBy::Id(user_id) => client.query_one(&stmt, &[&user_id]).await,
            RestBy::Num(rest_num) => client.query_one(&stmt, &[&rest_num]).await,
         };

         // Возвращаем результат
         match rows {
            Ok(row) => Some(Restaurant::from_db(&row)),
            Err(e) => {
               // Сообщаем об ошибке и возвращаем пустой результат
               settings::log(&format!("db::restaurant: {}", e)).await;
               None
            }
         }
      }
      Err(e) => {
         // Сообщаем об ошибке и возвращаем пустой результат
         settings::log(&format!("db::restaurant prepare: {}", e)).await;
         None
      }
   }
}

// Возвращает номер ресторана, если пользователю разрешён доступ в режим ресторатора
pub async fn rest_num(user : Option<&teloxide::types::User>) -> Result<i32, ()> {
   // Проверяем, передан ли пользователь.
   let u = user.ok_or(())?;

   // Выполняем запрос
   let rows = db_client().await.ok_or(())?
   .query_one("SELECT rest_num FROM restaurants WHERE user_id=$1::INTEGER AND enabled = TRUE", &[&u.id])
   .await;

   // Возвращаем номер ресторана, если такой есть.
   match rows {
      Ok(data) => Ok(data.get(0)),
      _ => Err(()),
   }
}

pub async fn rest_edit_title(rest_num: i32, new_str: String) -> bool {
   execute_one("UPDATE restaurants SET title = $1::VARCHAR(100) WHERE rest_num=$2::INTEGER", &[&new_str, &rest_num]).await
}

pub async fn rest_edit_info(rest_num: i32, new_str: String) -> bool {
   execute_one("UPDATE restaurants SET info = $1::VARCHAR(512) WHERE rest_num=$2::INTEGER", &[&new_str, &rest_num]).await
}

pub async fn rest_toggle(rest_num: i32) -> bool {
   execute_one("UPDATE restaurants SET active = NOT active WHERE rest_num=$1::INTEGER", &[&rest_num]).await
}

// Изменение фото ресторана
pub async fn rest_edit_image(rest_num: i32, image_id: &String) -> bool {
   execute_one("UPDATE restaurants SET image_id = $1::VARCHAR(512) WHERE rest_num=$2::INTEGER", &[&image_id, &rest_num]).await
}

// Изменяет владельца ресторана
pub async fn transfer_ownership(rest_num: i32, new_user_id: i32) -> bool {
   execute_one("UPDATE restaurants SET user_id = $1::INTEGER WHERE rest_num=$2::INTEGER", &[&new_user_id, &rest_num]).await
}

// Регистрация или разблокировка ресторатора
pub async fn register_caterer(user_id: i32) -> bool {
   // Попробуем разблокировать пользователя
   if execute_one_no_error("UPDATE restaurants SET enabled = TRUE WHERE user_id=$1::INTEGER", &[&user_id]).await {
      return true;
   }

   // Cоздадим новую запись
   execute_one("INSERT INTO restaurants (user_id, title, info, active, enabled, opening_time, closing_time) VALUES ($1::INTEGER, 'Мяу', 'Наш адрес 00NDC, доставка @nick, +84123', FALSE, TRUE, '07:00', '23:00')", &[&user_id])
   .await
}

// Приостановка доступа ресторатора
pub async fn hold_caterer(user_id: i32) -> bool {
   execute_one("UPDATE restaurants SET enabled = FALSE, active = FALSE WHERE user_id=$1::INTEGER", &[&user_id]).await
}

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

// Обновляет время работы ресторана на основании времени, заданного в группах
pub async fn rest_edit_time(rest_num: i32) -> bool {
   // Определяем самое частое время открытия и закрытия групп и записываем его как время ресторана
   execute_one("UPDATE restaurants SET opening_time = (SELECT opening_time FROM groups WHERE rest_num = $1::INTEGER GROUP BY opening_time ORDER BY Count(*) DESC LIMIT 1),
      closing_time = (SELECT closing_time FROM groups WHERE rest_num = $1::INTEGER GROUP BY closing_time ORDER BY Count(*) DESC LIMIT 1)
      WHERE rest_num = $1::INTEGER", &[&rest_num])
   .await
}

// ============================================================================
// [Groups table]
// ============================================================================
// Информация о ресторане
pub struct Group {
   pub rest_num: i32,
   pub num: i32,
   pub title: String,
   pub info: String,
   pub active: bool,
   pub cat_id: i32,
   pub opening_time: NaiveTime,
   pub closing_time: NaiveTime,
}

impl Group {
   // Инициализация из БД
   pub fn from_db(row: &Row) -> Self {
      Group {
         rest_num: row.get(0),
         num: row.get(1),
         title: row.get(2),
         info: row.get(3),
         active: row.get(4),
         cat_id: row.get(5),
         opening_time: row.get(6),
         closing_time: row.get(7),
      }
   }

   // Строка со временем работы группы, исключая время по-умолчанию для краткости
   fn work_time(&self, def_opening_time: NaiveTime, def_closing_time: NaiveTime) -> String {
      // Четыре варианта отображения времени
      if self.opening_time != def_opening_time && self.closing_time != def_closing_time {
         // Показываем и время начала и время конца
         format!(" ({}-{})", str_time(self.opening_time), str_time(self.closing_time))
      } else if self.opening_time != def_opening_time && self.closing_time == def_closing_time {
         // Показываем время начала
         format!(" (c {})", str_time(self.opening_time))
      } else if self.opening_time == def_opening_time && self.closing_time != def_closing_time {
         // Показываем время конца
         format!(" (до {})", str_time(self.closing_time))
      } else {
         // Не показываем время
         String::default()
      }
   }

   // Возвращает название вместе со временем работы
   pub fn title_with_time(&self, def_opening_time: NaiveTime, def_closing_time: NaiveTime) -> String {
      format!("{}{}", self.title, self.work_time(def_opening_time, def_closing_time))
   }
}

// Тип запроса информации о группе ресторана
pub enum GroupListBy {
   All(i32),               // все группы ресторана с указанным номером
   Category(i32, i32),     // активные, по номеру ресторана и категории
   Time(i32, NaiveTime),   // активные, по номеру ресторана и группами, работающими в указанное время
}

// Список групп
pub type GroupList = Vec<Group>;

// Возвращает список групп ресторана
pub async fn group_list(by: GroupListBy) -> Option<GroupList> {
   // Получим клиента БД из пула
   let client = db_client().await?;

   // Выберем нужный текст запроса
   let statement_text =  match by {
      GroupListBy::All(_rest_num) =>
         "SELECT g.rest_num, g.group_num, g.title, g.info, g.active, g.cat_id, g.opening_time, g.closing_time FROM groups as g
         WHERE rest_num=$1::INTEGER",
      GroupListBy::Category(_rest_num, _cat_id) =>
         "SELECT g.rest_num, g.group_num, g.title, g.info, g.active, g.cat_id, g.opening_time, g.closing_time FROM groups as g
         WHERE active = TRUE AND rest_num=$1::INTEGER AND cat_id=$2::INTEGER",
      GroupListBy::Time(_rest_num, _time) =>
         "SELECT g.rest_num, g.group_num, g.title, g.info, g.active, g.cat_id, g.opening_time, g.closing_time FROM groups as g
         WHERE active = TRUE AND rest_num=$1::INTEGER AND (($2::TIME BETWEEN opening_time AND closing_time) OR (opening_time > closing_time AND $2::TIME > opening_time))",
   };

   // Подготовим нужный запрос с кешем благодаря пулу
   let statement = client.prepare(statement_text).await;

   // Если запрос подготовлен успешно, выполняем его
   match statement {
      Ok(stmt) => {
         let rows = match by {
            GroupListBy::All(rest_num) => client.query(&stmt, &[&rest_num]).await,
            GroupListBy::Category(rest_num, cat_id) => client.query(&stmt, &[&rest_num, &cat_id]).await,
            GroupListBy::Time(rest_num, time) => client.query(&stmt, &[&rest_num, &time]).await,
         };

         // Возвращаем результат
         match rows {
            Ok(data) => if data.is_empty() {None} else {Some(data.into_iter().map(|row| (Group::from_db(&row))).collect())},
            Err(e) => {
               // Сообщаем об ошибке и возвращаем пустой результат
               settings::log(&format!("db::group_list: {}", e)).await;
               None
            }
         }
      }
      Err(e) => {
         // Сообщаем об ошибке и возвращаем пустой результат
         settings::log(&format!("db::group_list prepare: {}", e)).await;
         None
      }
   }
}

// Возвращает информацию о группе
pub async fn group(rest_num: i32, group_num: i32) -> Option<Group> {
   // Получим клиента БД из пула
   let client = db_client().await?;

   // Подготовим запрос
   let statement = client.prepare("SELECT g.rest_num, g.group_num, g.title, g.info, g.active, g.cat_id, g.opening_time, g.closing_time FROM groups as g
      WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER")
   .await;

   // Если запрос подготовлен успешно, выполняем его
   match statement {
      Ok(stmt) => {
         let rows = client.query_one(&stmt, &[&rest_num, &group_num]).await;

         // Возвращаем результат
         match rows {
            Ok(data) => Some(Group::from_db(&data)),
            Err(e) => {
               // Сообщаем об ошибке и возвращаем пустой результат
               settings::log(&format!("db::group(rest_num={}, group_num={}): {}", rest_num, group_num, e)).await;
               None
            }
         }
      }
      Err(e) => {
         // Сообщаем об ошибке и возвращаем пустой результат
         settings::log(&format!("db::group prepare: {}", e)).await;
         None
      }
   }
}

// Добавляет новую группу
pub async fn rest_add_group(rest_num: i32, new_str: String) -> bool {
   execute_one("INSERT INTO groups (rest_num, group_num, title, info, active, cat_id, opening_time, closing_time) 
      VALUES (
         $1::INTEGER, 
         (SELECT COUNT(*) FROM groups WHERE rest_num=$2::INTEGER) + 1,
         $3::VARCHAR(100),
         'Блюда подаются на тарелке',
         TRUE,
         2,
         '07:00',
         '23:00'
      )", &[&rest_num, &rest_num, &new_str]
   )
   .await
}

// Изменяет название группы
pub async fn rest_group_edit_title(rest_num: i32, group_num: i32, new_str: String) -> bool {
   execute_one("UPDATE groups SET title = $1::VARCHAR(100) WHERE rest_num=$2::INTEGER AND group_num=$3::INTEGER", &[&new_str, &rest_num, &group_num]).await
}

// Изменяет описание группы
pub async fn rest_group_edit_info(rest_num: i32, group_num: i32, new_str: String) -> bool {
   execute_one("UPDATE groups SET info = $1::VARCHAR(512) WHERE rest_num=$2::INTEGER AND group_num=$3::INTEGER", &[&new_str, &rest_num, &group_num]).await
}

// Переключает доступность группы
pub async fn rest_group_toggle(rest_num: i32, group_num: i32) -> bool {
   execute_one("UPDATE groups SET active = NOT active WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER", &[&rest_num, &group_num]).await
}

// Изменяет категорию группы
pub async fn rest_group_edit_category(rest_num: i32, group_num: i32, new_cat : i32) -> bool {
   execute_one("UPDATE groups SET cat_id = $1::INTEGER WHERE rest_num=$2::INTEGER AND group_num=$3::INTEGER", &[&new_cat, &rest_num, &group_num]).await
}

// Изменяет время доступности группы
pub async fn rest_group_edit_time(rest_num: i32, group_num: i32, opening_time: NaiveTime, closing_time: NaiveTime) -> bool {
   if execute_one("UPDATE groups SET opening_time = $1::TIME, closing_time = $2::TIME WHERE rest_num=$3::INTEGER AND group_num=$4::INTEGER", &[&opening_time, &closing_time, &rest_num, &group_num]).await {
      rest_edit_time(rest_num).await
   } else {
      false
   }
}

// Удаляет группу и изменяет порядковый номер оставшихся групп, в т.ч. и у блюд
pub async fn rest_group_remove(rest_num: i32, group_num: i32) -> bool {

   // Получаем клиента БД
   let client = db_client().await;
   if client.is_none() {return false;}

   // В клиенте действительное значение, можно смело развернуть
   let mut client = client.unwrap();

   // Если у группы есть блюда, выходим с неудачей
   let rows = client.query("SELECT dish_num FROM dishes WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER", &[&rest_num, &group_num]).await;
   match rows {
      Ok(data) => if !data.is_empty() {return false;}
      Err(e) => {
         settings::log(&format!("db::rest_group_remove no_dishes(rest_num={}, group_num={}): {}", rest_num, group_num, e)).await;
         return false;
      }
   }

   // Начинаем транзакцию
   let trans = client.transaction().await;
   if let Err(e) = trans {
      settings::log(&format!("db::rest_group_remove: {}", e)).await;
      return false;
   }
   let trans = trans.unwrap();

   // Удаляем группу
   let res = trans.execute("DELETE FROM groups WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER", &[&rest_num, &group_num])
   .await;
   match res {
      Ok(_) => {

         // Номера групп перенумеровываем для исключения дырки
         let res = trans.execute("UPDATE groups SET group_num = group_num - 1 WHERE rest_num=$1::INTEGER AND group_num>$2::INTEGER", &[&rest_num, &group_num])
         .await;
         match res {
            Ok(_) => {

               // Перенумеровываем группы у блюд
               let res = trans.execute("UPDATE dishes SET group_num = group_num - 1 WHERE rest_num=$1::INTEGER AND group_num>$2::INTEGER", &[&rest_num, &group_num])
               .await;
               match res {
                  Ok(_) => {
                     // Завершаем транзацию и возвращаем успех
                     match trans.commit().await {
                        Ok(_) => return true,
                        Err(e) => settings::log(&format!("db::rest_group_remove commit: {}", e)).await,
                     }
                  }
                  Err(e) => settings::log(&format!("db::rest_group_remove update dishes: {}", e)).await,
               }
            }
            Err(e) => settings::log(&format!("db::rest_group_remove update groups: {}", e)).await,
         }      
      }
      Err(e) => settings::log(&format!("db::rest_group_remove delete from groups: {}", e)).await,
   }
   false
}
 
// ============================================================================
// [Dishes table]
// ============================================================================
// Информация о блюде
#[derive(Clone)]
pub struct Dish {
   pub rest_num: i32,
   pub num: i32,
   pub title: String,
   pub info: String,
   pub active: bool,
   pub group_num: i32,
   pub price: i32,
   pub image_id: Option<String>,
}

impl Dish {
   pub fn from_db(row: &Row) -> Self {
      Dish {
         rest_num: row.get(0),
         num: row.get(1),
         title: row.get(2),
         info: row.get(3),
         active: row.get(4),
         group_num: row.get(5),
         price: row.get(6),
         image_id: row.get(7),
      }
   }

   // Возвращает название вместе с ценой
   pub fn title_with_price(&self) -> String {
      format!("{} {}", self.title, settings::price_with_unit(self.price))
   }

   // Возвращает описание для едока
   pub fn info_for_eater(&self) -> String {
      // Если описание слишком короткое, не выводим его
      let info_str = if self.info.len() < 3 {
         String::default()
      } else {
         format!("{}\n", self.info)
      };

      // Если цена нулевая, не выводим её
      let price_str = if self.price > 0 {format!("Цена: {}", settings::price_with_unit(self.price))}
      else {String::default()};

      format!("<b>{}</b>\n<i>{}</i>{}", self.title, info_str, price_str)
   }

   // Возвращает описание для ресторатора
   pub fn info_for_caterer(&self) -> String {
      format!("Название: {} /EditTitle\nДоп.инфо: {} /EditInfo\nГруппа: {} /EditGroup\nСтатус: {} /Toggle\nЦена: {} /EditPrice\nЗагрузить фото /EditImg\nУдалить блюдо /Remove\nСообщение для рекламы /Promote",
      self.title, self.info, self.group_num, active_to_str(self.active), settings::price_with_unit(self.price))
   }
}

// Тип запроса информации о блюдах
pub enum DishesBy {
   All(i32, i32),    // все по номеру ресторана и группы
   Active(i32, i32), // только активные по номеру ресторана и группы
   Find(String), // поиск по названию
}

// Список блюд
pub type DishList = Vec<Dish>;

// Возвращает список блюд
pub async fn dish_list(by: DishesBy) -> Option<DishList> {
   // Получим клиента БД из пула
   let client = db_client().await?;

   // Выберем нужный текст запроса
   let statement_text =  match by {
      DishesBy::All(_rest_num, _group_num) =>
         "SELECT d.rest_num, d.dish_num, d.title, d.info, d.active, d.group_num, d.price, d.image_id FROM dishes as d
         WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER ORDER BY dish_num",
      DishesBy::Active(_rest_num, _group_num) =>
         "SELECT d.rest_num, d.dish_num, d.title, d.info, d.active, d.group_num, d.price, d.image_id FROM dishes as d
         WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND active = TRUE ORDER BY dish_num",
      DishesBy::Find(_) =>
         "SELECT DISTINCT d.rest_num, d.dish_num, d.title, d.info, d.active, d.group_num, d.price, d.image_id FROM dishes as d
         INNER JOIN restaurants r ON r.rest_num = d.rest_num
         INNER JOIN groups g ON g.group_num = d.group_num
         WHERE r.active = TRUE AND r.enabled = TRUE AND g.active = TRUE AND UPPER(d.title) like UPPER($1::VARCHAR(100)) ORDER BY d.rest_num, d.group_num",
   };

   // Подготовим нужный запрос с кешем благодаря пулу
   let statement = client.prepare(statement_text).await;

   // Если запрос подготовлен успешно, выполняем его
   match statement {
      Ok(stmt) => {
         let rows = match by {
            DishesBy::All(rest_num, group_num) => client.query(&stmt, &[&rest_num, &group_num]).await,
            DishesBy::Active(rest_num, group_num) => client.query(&stmt, &[&rest_num, &group_num]).await,
            DishesBy::Find(text) => client.query(&stmt, &[&text]).await,
         };

         // Возвращаем результат
         match rows {
            Ok(data) => if data.is_empty() {None} else {Some(data.into_iter().map(|row| (Dish::from_db(&row))).collect())},
            Err(e) => {
               // Сообщаем об ошибке и возвращаем пустой результат
               settings::log(&format!("db::dish_list: {}", e)).await;
               None
            }
         }
      }
      Err(e) => {
         // Сообщаем об ошибке и возвращаем пустой результат
         settings::log(&format!("db::dish_list prepare: {}", e)).await;
         None
      }
   }
}

// Тип запроса информации о блюде
pub enum DishBy {
   All(i32, i32, i32),  // по номеру ресторана, группы и блюда
   Active(i32, i32, i32),    // только активное по номеру ресторана, группы и блюда
}

// Возвращает информацию о блюде
pub async fn dish(by: DishBy) -> Option<Dish> {
   // Получим клиента БД из пула
   let client = db_client().await?;

   // Выберем нужный текст запроса
   let statement_text =  match by {
      DishBy::All(_rest_num, _group_num, _dish_num) =>
         "SELECT d.rest_num, d.dish_num, d.title, d.info, d.active, d.group_num, d.price, d.image_id FROM dishes as d
         WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num=$3::INTEGER",
      DishBy::Active(_rest_num, _group_num, _dish_num) =>
         "SELECT d.rest_num, d.dish_num, d.title, d.info, d.active, d.group_num, d.price, d.image_id FROM dishes as d
         WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num=$3::INTEGER AND active = TRUE",
   };

   // Подготовим нужный запрос с кешем благодаря пулу
   let statement = client.prepare(statement_text).await;

   // Если запрос подготовлен успешно, выполняем его
   match statement {
      Ok(stmt) => {
         let rows = match by {
            DishBy::All(rest_num, group_num, dish_num) => client.query_one(&stmt, &[&rest_num, &group_num, &dish_num]).await,
            DishBy::Active(rest_num, group_num, dish_num) => client.query_one(&stmt, &[&rest_num, &group_num, &dish_num]).await,
         };

         // Возвращаем результат
         match rows {
            Ok(row) => Some(Dish::from_db(&row)),
            Err(e) => {
               // Сообщаем об ошибке и возвращаем пустой результат
               settings::log(&format!("Error dish: {}", e)).await;
               None
            }
         }
      }
      Err(e) => {
         // Сообщаем об ошибке и возвращаем пустой результат
         settings::log(&format!("db::dish_list prepare: {}", e)).await;
         None
      }
   }
}

// Возвращает картинку блюда, если задана, иначе пытается получить картинку ресторана и т.д.
pub async fn load_dish_image(dish: &Dish) -> InputFile {
   
   // Возвращает картинку для указанного ресторана, если есть
   async fn load_rest_image(rest_num: i32) -> Option<String> {
      // Получаем клиента БД
      let client = db_client().await;
      if client.is_none() {return None;}

      // В клиенте действительное значение, можно смело развернуть
      let client = client.unwrap();

      // Подготовим запрос
      let statement = client.prepare("SELECT image_id FROM restaurants WHERE rest_num=$1::INTEGER").await;

      // Если запрос подготовлен успешно, выполняем его
      match statement {
         Ok(stmt) => {
            let rows = client.query_one(&stmt, &[&rest_num]).await;

            // Возвращаем результат
            match rows {
               Ok(data) => return data.get(0),
               Err(e) => {
                  // Сообщаем об ошибке и возвращаем пустой результат
                  settings::log(&format!("db::load_rest_image(rest_num={}): {}", rest_num, e)).await;
               }
            }
         }
         Err(e) => {
            // Сообщаем об ошибке и возвращаем пустой результат
            settings::log(&format!("db::load_rest_image prepare: {}", e)).await;
         }
      }
      None
   }

   // Получаем идентификатор от первого доступного источника
   let id = dish.image_id.to_owned()
   .unwrap_or(load_rest_image(dish.rest_num).await
   .unwrap_or(settings::default_photo_id()));

   // Возврашаем объект
   InputFile::file_id(id)
}

// Добавляет новое блюдо
pub async fn rest_add_dish(rest_num: i32, group_num: i32, new_str: String) -> bool {
   execute_one("INSERT INTO dishes (rest_num, dish_num, title, info, active, group_num, price) 
   VALUES (
      $1::INTEGER, 
      (SELECT COUNT(*) FROM dishes WHERE rest_num = $2::INTEGER AND group_num = $3::INTEGER) + 1,
      $4::VARCHAR(100),
      'Порция 100гр.',
      TRUE,
      $5::INTEGER,
      0
   )", &[&rest_num, &rest_num, &group_num, &new_str, &group_num])
   .await
}

// Редактирование названия блюда
pub async fn rest_dish_edit_title(rest_num: i32, group_num: i32, dish_num: i32, new_str: String) -> bool {
   execute_one("UPDATE dishes SET title = $1::VARCHAR(100) WHERE rest_num=$2::INTEGER AND group_num=$3::INTEGER AND dish_num=$4::INTEGER", &[&new_str, &rest_num, &group_num, &dish_num])
   .await
}

// Редактирование описания блюда
pub async fn rest_dish_edit_info(rest_num: i32, group_num: i32, dish_num: i32, new_str: String) -> bool {
   execute_one("UPDATE dishes SET info = $1::VARCHAR(512) WHERE rest_num=$2::INTEGER AND group_num=$3::INTEGER AND dish_num=$4::INTEGER", &[&new_str, &rest_num, &group_num, &dish_num])
   .await
}

// Переключение доступности блюда
pub async fn rest_dish_toggle(rest_num: i32, group_num: i32, dish_num: i32) -> bool {
   execute_one("UPDATE dishes SET active = NOT active WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num=$3::INTEGER", &[&rest_num, &group_num, &dish_num])
   .await
}

// Изменение группы блюда
pub async fn rest_dish_edit_group(rest_num: i32, old_group_num: i32, dish_num: i32, new_group_num: i32) -> bool {
   // Получаем клиента БД
   let client = db_client().await;
   if client.is_none() {return false;}

   // В клиенте действительное значение, можно смело развернуть
   let client = client.unwrap();

   // Проверим, что есть такая целевая группа
   let row = client.query_one("SELECT group_num FROM groups WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER", &[&rest_num, &new_group_num])
   .await;

   // Если целевой группы нет, выходим
   if let Err(_) = row {
      return false
   }

   // Сохраним информацию о блюде
   match dish(DishBy::All(rest_num, old_group_num, dish_num)).await {
      Some(dish) => {
         // Добавляем блюдо в целевую группу
         let res = execute_one("INSERT INTO dishes (rest_num, dish_num, title, info, active, group_num, price, image_id) 
            VALUES (
               $1::INTEGER, 
               (SELECT COUNT(*) FROM dishes WHERE rest_num = $1::INTEGER AND group_num = $2::INTEGER) + 1,
               $3::VARCHAR(100),
               $4::VARCHAR(512),
               $5::BOOLEAN,
               $2::INTEGER,
               $6::INTEGER,
               $7::VARCHAR(512)
            )", &[&dish.rest_num, &new_group_num, &dish.title, &dish.info, &dish.active, &dish.price, &dish.image_id]
         )
         .await;

         // Если успешно, удалим блюдо из прежней группы
         if res {
            // Игнорируем результат, задвоение это нестрашно, иначе надо создавать транзакцию
            rest_dish_remove(rest_num, old_group_num, dish_num).await;

            true
         } else {
            false
         }
      }
      None => false
   }
}

// Удаление блюда
pub async fn rest_dish_remove(rest_num: i32, group_num: i32, dish_num: i32) -> bool {
   // Получаем клиента БД
   let client = db_client().await;
   if client.is_none() {return false;}

   // В клиенте действительное значение, можно смело развернуть
   let mut client = client.unwrap();

   // Начинаем транзакцию
   let trans = client.transaction().await;
   if let Err(e) = trans {
      settings::log(&format!("db::rest_dish_remove: {}", e)).await;
      return false;
   }
   let trans = trans.unwrap();

   // Удаляем блюдо
   let res = trans.execute("DELETE FROM dishes WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num=$3::INTEGER", &[&rest_num, &group_num, &dish_num])
   .await;
   match res {
      Ok(_) => {

         // Номера оставшихся блюд перенумеровываем для исключения дырки
         let res = trans.execute("UPDATE dishes SET dish_num = dish_num - 1 WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num > $3::INTEGER", &[&rest_num, &group_num, &dish_num])
         .await;
         match res {
            Ok(_) => {
               // Завершаем транзацию, удаляем блюдо из заказов пользователей и возвращаем успех
               match trans.commit().await {
                  Ok(_) => {
                     dish_remove_from_orders(rest_num, group_num, dish_num).await;
                     return true;
                  }
                  Err(e) => settings::log(&format!("db::rest_group_remove commit: {}", e)).await,
               }
            }
            Err(e) => settings::log(&format!("db::rest_dish_remove update (rest_num={}, group_num={}, dish_num={}): {}", rest_num, group_num, dish_num, e)).await,
         }
      },
      Err(e) => settings::log(&format!("db::rest_dish_remove delete (rest_num={}, group_num={}, dish_num={}): {}", rest_num, group_num, dish_num, e)).await,
   }
   false
}

// Изменение цены блюда
pub async fn rest_dish_edit_price(rest_num: i32, group_num: i32, dish_num: i32, price: i32) -> bool {
   execute_one("UPDATE dishes SET price = $1::INTEGER WHERE rest_num=$2::INTEGER AND group_num=$3::INTEGER AND dish_num=$4::INTEGER", &[&price, &rest_num, &group_num, &dish_num])
   .await
}

// Изменение фото блюда
pub async fn rest_dish_edit_image(rest_num: i32, group_num: i32, dish_num: i32, image_id: &String) -> bool {
   execute_one("UPDATE dishes SET image_id = $1::VARCHAR(100) WHERE rest_num=$2::INTEGER AND group_num=$3::INTEGER AND dish_num=$4::INTEGER", &[&image_id, &rest_num, &group_num, &dish_num])
   .await
}

// ============================================================================
// [Users table]
// ============================================================================

// Обновляет временную отметку последнего входа, возвращая истину, если данные существовали ранее
async fn user_update_last_seen(user: Option<&User>) -> bool {
   async fn insert(u : &User) {
      // Информация о пользователе
      let name = if let Some(last_name) = &u.last_name {
         format!("{} {}", u.first_name, last_name)
      } else {u.first_name.clone()};

      let contact = if let Some(username) = &u.username {
         format!(" @{}", username)
      } else {String::from("-")};

      // Создаём новую запись о пользователе
      execute_one("INSERT INTO users (user_id, user_name, contact, address, last_seen, compact, pickup) VALUES ($1::INTEGER, $2::VARCHAR(100), $3::VARCHAR(100), '-', NOW(), FALSE, FALSE)",
         &[&u.id, &name, &contact]
      )
      .await;
   }

   if let Some(u) = user {
      // Получаем клиента БД
      let client = db_client().await;
      if client.is_none() {return false;}

      // В клиенте действительное значение, можно смело развернуть
      let client = client.unwrap();

      // Подготовим нужный запрос с кешем благодаря пулу на обновление времени последней активности
      let statement = client.prepare("UPDATE users SET last_seen = NOW() WHERE user_id=$1::INTEGER").await;

      match statement {
         Ok(stmt) => {
            // Если запрос подготовлен успешно, выполняем его
            let query = client.execute(&stmt, &[&u.id]).await;

            // Возвращаем результат
            match query {
               Ok(0) => insert(u).await,  // если обновили 0 записей, вставим новую
               Ok(1) => return true,      // обновление было успешным, значит данные уже существовали
               Ok(n) => settings::log(&format!("Error user_update_last_seen, updated {} instead one", n)).await,
               Err(e) => settings::log(&format!("Error user_update_last_seen: {}", e)).await,
            }
         },
         Err(e) => settings::log(&format!("db::dish_list prepare: {}", e)).await,
      }
   } else {settings::log(&format!("Error user_update_last_seen, no user")).await;}

   // Считывать настройку пользователя нет смысла
   false
}

// Возвращает настройку пользователя и обновляет временную метку последнего входа
pub async fn user_compact_interface(user: Option<&User>) -> bool {
   // Обновим отметку и узнаем, есть ли смысл читать настройку из базы
   if user_update_last_seen(user).await {

      // Получаем клиента БД
      let client = db_client().await;
      if client.is_none() {return false;}

      // В клиенте действительное значение, можно смело развернуть
      let client = client.unwrap();

      // Пытаемся прочитать настройки из БД
      let query = client.query_one("SELECT compact FROM users WHERE user_id=$1::INTEGER", &[&user.unwrap().id]).await;
      match query {
         Ok(row) => return row.get(0), // и выходим
         Err(e) => settings::log(&format!("Error user_compact_interface: {}", e)).await,
      }
   }

   // Возвращаем значение по-умолчанию
   false
}

// Переключает режим интерфейса
pub async fn user_toggle_interface(user: Option<&User>) {
   if let Some(u) = user {
      execute_one("UPDATE users SET compact = NOT compact WHERE user_id=$1::INTEGER", &[&u.id])
      .await;
   } else {
      // Если не передали пользователя, сообщим об этом
      settings::log(&format!("Error toggle interface settings, no user")).await;
   }
}

// Информация о пользователе для корзины
pub struct UserBasketInfo {
   pub name: String, 
   pub contact: String, 
   pub address: String,    // лежит либо текст с адресом либо LocationNNN, где NNN это id сообщения с локацией
   pub pickup: bool,
}

impl UserBasketInfo {
   pub fn from_db(row: &Row) -> Self {
      Self {
         name: row.get(0),
         contact: row.get(1),
         address: row.get(2),
         pickup: row.get(3),
      }
   }

   // Возвращает истину, если адрес задан геопозицией
   pub fn is_geolocation(&self) -> bool {
      return self.address.get(..8).unwrap_or_default() == "Location";
   }

   // Возвращает либо сам адрес либо надпись, что задана точка
   pub fn address_label(&self) -> String {
      // Если адрес начинается с ключевого слова, значит там id сообщения с локацией
      if self.is_geolocation() {String::from("на карте")} else {self.address.clone()}
   }

   // Возвращает id сообщения с локацией, если имеется и не самовывоз
   pub fn address_message_id(&self) -> Option<i32> {
      if self.pickup {return None;}

      if self.is_geolocation() {
         // Пытаемся получить продолжение строки
         if let Some(s) = self.address.get(8..) {
            // Пытаемся преобразовать в число.
            if let Ok(res) = s.parse::<i32>() {Some(res)} else {None}
         } else {None}
      } else {None}
   }
}


pub async fn user_basket_info(user_id: i32) -> Option<UserBasketInfo> {
   // Получаем клиента БД
   let client = db_client().await?;

   let query = client.query("SELECT user_name, contact, address, pickup from users WHERE user_id=$1::INTEGER", &[&user_id])
   .await;

   match query {
      Ok(data) => {
         if !data.is_empty() {
            return Some(UserBasketInfo::from_db(&data[0]));
         }
      }
      // Если произошла ошибка, сообщим о ней
      Err(e) => settings::log(&format!("Error db::user_basket_info: {}", e)).await,
   }
   
   // Если попали сюда, значит была ошибка
   None
}

// Изменение имени пользователя
pub async fn basket_edit_name(user_id: i32, s: String) -> bool {
   execute_one("UPDATE users SET user_name = $1::VARCHAR(100) WHERE user_id=$2::INTEGER", &[&s, &user_id])
   .await
}

// Возврат имени пользователя
pub async fn user_name_by_id(user_id: i32) -> String {
   // Получаем клиента БД
   let client = db_client().await;
   if client.is_none() {return String::from("Неизвестное имя");}

   // Выполняем запрос
   let query = client.unwrap()
   .query_one("SELECT user_name FROM users WHERE user_id=$1::INTEGER", &[&user_id])
   .await;
   match query {
       Ok(data) => data.get(0),
       Err(e) => {
         settings::log(&format!("Error db::basket_edit_name: {}", e)).await;
         String::from("Неизвестное имя")
       }
   }
}

// Изменение контакта пользователя
pub async fn basket_edit_contact(user_id: i32, s: String) -> bool {
   execute_one("UPDATE users SET contact = $1::VARCHAR(100) WHERE user_id=$2::INTEGER", &[&s, &user_id])
   .await
}

// Изменение адреса пользователя
pub async fn basket_edit_address(user_id: i32, s: String) -> bool {
   execute_one("UPDATE users SET address = $1::VARCHAR(100) WHERE user_id=$2::INTEGER", &[&s, &user_id])
   .await
}

// Изменение способа доставки
pub async fn basket_toggle_pickup(user_id: i32) -> bool {
   execute_one("UPDATE users SET pickup = NOT pickup WHERE user_id=$1::INTEGER", &[&user_id])
   .await
}

// ============================================================================
// [Orders table]
// ============================================================================

// Перемещает заказ из таблицы orders в tickets
pub async fn order_to_ticket(eater_id: i32, caterer_id: i32, eater_order_msg_id: i32, caterer_order_msg_id: i32) -> bool {
   // Получаем клиента БД
   let client = db_client().await;
   if client.is_none() {return false;}

   // В клиенте действительное значение, можно смело развернуть
   let mut client = client.unwrap();

   // Начинаем транзакцию
   let trans = client.transaction().await;
   if let Err(e) = trans {
      settings::log(&format!("db::order_to_ticket: {}", e)).await;
      return false;
   }
   let trans = trans.unwrap();

   // Удаляем все блюда ресторана из orders
   let res = trans.execute("DELETE FROM orders o USING restaurants r WHERE o.rest_num = r.rest_num AND o.user_id = $1::INTEGER AND r.user_id = $2::INTEGER", &[&eater_id, &caterer_id])
   .await;
   match res {
      Ok(_) => {
         // Создаём запись в tickets
         let res = trans.execute("INSERT INTO tickets (eater_id, caterer_id, eater_msg_id, caterer_msg_id, eater_status_msg_id, caterer_status_msg_id, stage) VALUES ($1::INTEGER, $2::INTEGER, $3::INTEGER, $4::INTEGER, NULL, NULL, 1)", &[&eater_id, &caterer_id, &eater_order_msg_id, &caterer_order_msg_id])
         .await;
         match res {
            Ok(_) => {
               // Завершаем транзацию и возвращаем успех
               match trans.commit().await {
                  Ok(_) => return true,
                  Err(e) => settings::log(&format!("db::order_to_ticket commit: {}", e)).await,
               }
            }
            Err(e) => settings::log(&format!("db::order_to_ticket insert: {}", e)).await,
         }
      }
      Err(e) => settings::log(&format!("db::order_to_ticket delete from orders: {}", e)).await,
   }
   false
}

// Удаление блюда из заказов пользователей
async fn dish_remove_from_orders(rest_num: i32, group_num: i32, dish_num: i32) {
   // Получаем клиента БД
   let client = db_client().await;
   if client.is_none() {return;}

   // В клиенте действительное значение, можно смело развернуть
   let mut client = client.unwrap();

   // Начинаем транзакцию
   let trans = client.transaction().await;
   if let Err(e) = trans {
      settings::log(&format!("db::dish_remove_from_orders: {}", e)).await;
      return;
   }
   let trans = trans.unwrap();

   // Удалим блюдо из корзины всех пользователей
   let res = trans.execute("DELETE FROM orders WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num=$3::INTEGER", &[&rest_num, &group_num, &dish_num])
   .await;
   match res {
      Ok(_) => {
         // Обновим номера блюд в корзине согласно перенумерации самих блюд
         let res = trans.execute("UPDATE orders SET dish_num = dish_num - 1 WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num > $3::INTEGER", &[&rest_num, &group_num, &dish_num])
         .await;
         match res {
            Ok(_) => {
               // Завершаем транзацию и возвращаем успех
               match trans.commit().await {
                  Ok(_) => return,
                  Err(e) => settings::log(&format!("db::dish_remove_from_orders({}) commit: {}", make_key_3_int(rest_num, group_num, dish_num), e)).await,
               }
            }
            Err(e) => settings::log(&format!("db::dish_remove_from_orders({}) recounting: {}", make_key_3_int(rest_num, group_num, dish_num), e)).await,
         }
      }
      Err(e) => {
         // Сообщим об ошибке
         log::info!("db::dish_remove_from_orders({}): {}", make_key_3_int(rest_num, group_num, dish_num), e);
      }
   }
}

// Возвращает количество порций блюда в корзине
pub async fn amount_in_basket(rest_num: i32, group_num: i32, dish_num: i32, user_id: i32) -> i32 {
   // Получаем клиента БД
   let client = db_client().await;
   if client.is_none() {return 0;}

   // В клиенте действительное значение, можно смело развернуть
   let client = client.unwrap();

   // Подготовим запрос
   let statement = client.prepare("SELECT amount FROM orders WHERE user_id=$1::INTEGER AND rest_num=$2::INTEGER AND group_num=$3::INTEGER AND dish_num=$4::INTEGER")
   .await;

   // Если запрос подготовлен успешно, выполняем его
   match statement {
      Ok(stmt) => {
         let rows = client.query(&stmt, &[&user_id, &rest_num, &group_num, &dish_num]).await;

         // Возвращаем результат
         match rows {
            Ok(data) => {
               if !data.is_empty() {return data[0].get(0);}
            }
            Err(e) => {
               // Сообщаем об ошибке и возвращаем пустой результат
               settings::log(&format!("db::amount_in_basket(rest_num={}, group_num={}, dish_num={}, user_id={}): {}", rest_num, group_num, dish_num, user_id, e)).await;
            }
         }
      }
      Err(e) => {
         // Сообщаем об ошибке и возвращаем пустой результат
         settings::log(&format!("db::amount_in_basket prepare: {}", e)).await;
      }
   }

   // Раз попали сюда, значит ничего нет
   0
}

// Добавляет блюдо в корзину, возвращая новое количество
pub async fn add_dish_to_basket(rest_num: i32, group_num: i32, dish_num: i32, user_id: i32) -> Result<i32, ()> {

   // Текущее количество экземпляров в корзине
   let old_amount = amount_in_basket(rest_num, group_num, dish_num, user_id).await;

   // Если такая запись уже есть, надо увеличить на единицу количество, иначе создать новую запись
   let query_str = if old_amount > 0 {
      "UPDATE orders SET amount = amount + 1 WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num=$3::INTEGER AND user_id=$4::INTEGER"
   } else {
      "INSERT INTO orders (rest_num, group_num, dish_num, user_id, amount) VALUES ($1::INTEGER, $2::INTEGER, $3::INTEGER, $4::INTEGER, 1)"
   };

   // Получим клиента БД из пула
   let client = db_client().await;
   if client.is_none() {return Err(());}

   // В клиенте действительное значение, можно смело развернуть
   let client = client.unwrap();

   // Подготовим запрос
   let statement = client.prepare(query_str).await;

   // Если запрос подготовлен успешно, выполняем его
   match statement {
      Ok(stmt) => {
         let rows = client.execute(&stmt, &[&rest_num, &group_num, &dish_num, &user_id]).await;

         // Возвращаем результат
         match rows {
            Ok(1) => return Ok(old_amount + 1),
            Err(e) => settings::log(&format!("db::add_dish_to_basket (rest_num={}, group_num={}, dish_num={}, user_id={}): {}", rest_num, group_num, dish_num, user_id, e)).await,
            _ => settings::log(&format!("db::add_dish_to_basket more than 1 (rest_num={}, group_num={}, dish_num={}, user_id={})", rest_num, group_num, dish_num, user_id)).await,
         }
      }
      Err(e) => settings::log(&format!("db::add_dish_to_basket: {}", e)).await,
   }
   Err(())
}

// Удаляет блюдо из корзины
pub async fn remove_dish_from_basket(rest_num: i32, group_num: i32, dish_num: i32, user_id: i32) -> Result<i32, ()> {
   // Текущее количество экземпляров в корзине
   let old_amount = amount_in_basket(rest_num, group_num, dish_num, user_id).await;

   // Если остался только один экземпляр или меньше, удаляем запись, иначе редактируем.
   let query_str = if old_amount > 1 {
      "UPDATE orders SET amount = amount - 1 WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num=$3::INTEGER AND user_id=$4::INTEGER"
   } else {
      "DELETE FROM orders WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER AND dish_num=$3::INTEGER AND user_id=$4::INTEGER"
   };

   // Получим клиента БД из пула
   let client = db_client().await;
   if client.is_none() {return Err(());}

   // В клиенте действительное значение, можно смело развернуть
   let client = client.unwrap();

   // Подготовим запрос
   let statement = client.prepare(query_str).await;

   // Если запрос подготовлен успешно, выполняем его
   match statement {
      Ok(stmt) => {
         let rows = client.execute(&stmt, &[&rest_num, &group_num, &dish_num, &user_id]).await;

         // Возвращаем результат
         match rows {
            Ok(1) => return Ok(old_amount - 1),
            Err(e) => settings::log(&format!("db::remove_dish_from_basket (rest_num={}, group_num={}, dish_num={}, user_id={}): {}", rest_num, group_num, dish_num, user_id, e)).await,
            _ => settings::log(&format!("db::remove_dish_from_basket more than 1 (rest_num={}, group_num={}, dish_num={}, user_id={})", rest_num, group_num, dish_num, user_id)).await,
         }
      }
      Err(e) => settings::log(&format!("db::remove_dish_from_basket: {}", e)).await,
   }
   Err(())
}

// Содержимое корзины одного ресторана
pub struct Basket {
   pub rest_id: i32,
   pub restaurant: String,
   pub dishes: Vec<String>,
   pub total: i32,
}

// Содержимое корзин всех ресторанов
pub struct Baskets {
   pub baskets: Vec<Basket>,
   pub grand_total: i32,
}

// Возвращает содержимое корзины всех ресторанов и итоговую сумму заказа
pub async fn basket_contents(user_id: i32) -> Option<Baskets> {
   // Получим клиента БД из пула
   let client = db_client().await?;

   // Подготовим нужный запрос с кешем благодаря пулу - выберем все упомянутые рестораны
   let statement = client.prepare("SELECT DISTINCT r.title, r.info, r.rest_num, r.user_id FROM orders as o 
      INNER JOIN restaurants r ON o.rest_num = r.rest_num 
      WHERE o.user_id = $1::INTEGER
      ORDER BY r.rest_num"
   ).await;

   // Если запрос подготовлен успешно, выполняем его
   match statement {
      Ok(stmt) => {
         let rows = client.query(&stmt, &[&user_id]).await;
      
         // Возвращаем результат
         match rows {
            Ok(data) => if !data.is_empty() {
               // Для возврата результата
               let mut baskets = Vec::<Basket>::new();
               let mut grand_total: i32 = 0;

               // Проходим по всем записям
               for record in data {
                  // Данные из запроса о ресторане
                  let rest_title: String = record.get(0);
                  let rest_info: String = record.get(1);
                  let rest_num: i32 = record.get(2);
                  let rest_id: i32 = record.get(3);
         
                  // Создаём корзину ресторана
                  let basket_opt = basket_content(user_id, rest_num, rest_id, &rest_title, &rest_info, false).await;
         
                  if let Some(basket) = basket_opt {
                     // Обновляем общий итог
                     grand_total += basket.total;

                     // Помещаем ресторан в список
                     baskets.push(basket);
                  }
               }

               // Возвращаем результат
               return Some(Baskets{baskets, grand_total})
            }
            Err(e) => settings::log(&format!("db::basket_contents: {}", e)).await,
         }
      }
      Err(e) => settings::log(&format!("db::basket_contents prepare: {}", e)).await,
   }
   None
}

// Возвращает содержимое корзины и итоговую сумму заказа
pub async fn basket_content(user_id: i32, rest_num: i32, rest_id: i32, rest_title: &String, rest_info: &String, no_commands: bool) -> Option<Basket> {
   // Получим клиента БД из пула
   let client = db_client().await?;

   // Подготовим нужный запрос с кешем благодаря пулу - информация о блюдах ресторана
   let statement = client.prepare("SELECT d.title, d.price, o.amount, o.group_num, o.dish_num FROM orders as o 
      INNER JOIN groups g ON o.rest_num = g.rest_num AND o.group_num = g.group_num
      INNER JOIN dishes d ON o.rest_num = d.rest_num AND o.group_num = d.group_num AND o.dish_num = d.dish_num
      WHERE o.user_id = $1::INTEGER AND o.rest_num = $2::INTEGER
      ORDER BY o.group_num, o.dish_num"
   ).await;

   // Если запрос подготовлен успешно, выполняем его
   match statement {
      Ok(stmt) => {
         let rows = client.query(&stmt, &[&user_id, &rest_num]).await;
         match rows {
            Ok(data) => if !data.is_empty() {

               // Для общей суммы заказа по ресторану
               let mut total: i32 = 0;
               let mut dishes = Vec::<String>::new();

               // Двигаемся по каждой записи и сохраняем информацию о блюде
               for record in data {
                  // Данные из запроса
                  let title: String = record.get(0);
                  let price: i32 = record.get(1);
                  let amount: i32 = record.get(2);
                  let group_num: i32 = record.get(3);
                  let dish_num: i32 = record.get(4);

                  // Добавляем стоимость в итог
                  total += price * amount;

                  // Строка с информацией о блюде - с командами или без
                  let s = if no_commands {
                     format!("{}: {} x {} шт. = {}", title, price, amount, settings::price_with_unit(price * amount))
                  } else {
                     format!("{}: {} x {} шт. = {} /del{}", title, price, amount, settings::price_with_unit(price * amount), make_key_3_int(rest_num, group_num, dish_num))
                  };

                  // Помещаем блюдо в список
                  dishes.push(s);
               }
            
               // Возвращаем результат
               return Some(Basket{
                  rest_id,
                  restaurant: format!("{}. {}. {}\n", rest_num, rest_title, rest_info),
                  dishes,
                  total,
               })
            }
            Err(e) => settings::log(&format!("db::basket_content: {}", e)).await,
         }
      }
      Err(e) => settings::log(&format!("db::basket_content prepare: {}", e)).await,
   }
   None
}

// Очищает корзину указанного пользователя
pub async fn clear_basket(user_id: i32) -> bool {
   execute("DELETE FROM orders WHERE user_id = $1::INTEGER", &[&user_id]).await
}

// ============================================================================
// [Tickets table]
// ============================================================================

pub struct Ticket {
   pub ticket_id: i32,                    // Уникальный ключ БД
   pub eater_id: i32,                     // Уникальный ключ БД
   pub caterer_id: i32,                   // Уникальный ключ БД
   pub eater_order_msg_id: i32,           // Сообщение с самим заказом в чате с едоком
   pub caterer_order_msg_id: i32,         // Сообщение с самим заказом в чате с ресторатором
   pub eater_status_msg_id: Option<i32>,  // Сообщение со статусом заказа в чате с едоком
   pub caterer_status_msg_id: Option<i32>,// Сообщение со статусом заказа в чате с ресторатором
   pub stage: i32,
}

impl Ticket {
   pub fn from_db(row: &Row) -> Self {
      Self {
         ticket_id: row.get(0), 
         eater_id: row.get(1), 
         caterer_id: row.get(2), 
         eater_order_msg_id: row.get(3),
         caterer_order_msg_id: row.get(4),
         eater_status_msg_id: row.get(5),
         caterer_status_msg_id: row.get(6),
         stage: row.get(7),
      }
   }
}

// Тип запроса информации о тикете
pub enum TicketBy {
   TicketId(i32),                // по коду
   EaterAndCatererId(i32, i32),  // по номеру едока и ресторатора
}

// Тип запроса информации о списке тикетов
pub enum TicketListBy {
   EaterId(i32),     // по номеру едока
   CatererId(i32),   // по номеру ресторатора
}

// Для списка тикетов
pub type TicketList = Vec<Ticket>;

// Возвращает список тикетов
pub async fn ticket_list_by(by: TicketListBy) -> Option<TicketList> {
   // Получим клиента БД из пула
   let client = db_client().await?;

   // Выберем нужный текст запроса
   let statement_text =  match by {
      TicketListBy::EaterId(_id) =>
         "SELECT ticket_id, eater_id, caterer_id, eater_msg_id, caterer_msg_id, eater_status_msg_id, caterer_status_msg_id, stage FROM tickets WHERE eater_id=$1::INTEGER AND stage < 5",
      TicketListBy::CatererId(_id) =>
         "SELECT ticket_id, eater_id, caterer_id, eater_msg_id, caterer_msg_id, eater_status_msg_id, caterer_status_msg_id, stage FROM tickets WHERE caterer_id=$1::INTEGER AND stage < 5",
   };

   // Подготовим нужный запрос с кешем благодаря пулу
   let statement = client.prepare(statement_text).await;

   // Если запрос подготовлен успешно, выполняем его
   match statement {
      Ok(stmt) => {
         let rows = match by {
            TicketListBy::EaterId(id) => client.query(&stmt, &[&id]).await,
            TicketListBy::CatererId(id) => client.query(&stmt, &[&id]).await,
         };

         // Возвращаем результат
         match rows {
            Ok(data) => if data.is_empty() {None} else {Some(data.into_iter().map(|row| (Ticket::from_db(&row))).collect())},
            Err(e) => {
               // Сообщаем об ошибке и возвращаем пустой результат
               settings::log(&format!("db::ticket_list_by: {}", e)).await;
               None
            }
         }
      }
      Err(e) => {
         // Сообщаем об ошибке и возвращаем пустой результат
         settings::log(&format!("db::ticket_list_by prepare: {}", e)).await;
         None
      }
   }
}

// Возвращает тикеты с владельцами
pub async fn ticket(by: TicketBy) -> Option<Ticket> {
   // Получим клиента БД из пула
   let client = db_client().await?;

   // Выберем нужный текст запроса
   let statement_text =  match by {
      TicketBy::TicketId(_id) =>
         "SELECT ticket_id, eater_id, caterer_id, eater_msg_id, caterer_msg_id, eater_status_msg_id, caterer_status_msg_id, stage FROM tickets WHERE ticket_id=$1::INTEGER",
      TicketBy::EaterAndCatererId(_eater_id, _caterer_id) =>
         "SELECT ticket_id, eater_id, caterer_id, eater_msg_id, caterer_msg_id, eater_status_msg_id, caterer_status_msg_id, stage FROM tickets WHERE eater_id=$1::INTEGER AND caterer_id=$2::INTEGER AND stage < 5",
   };

   // Подготовим нужный запрос с кешем благодаря пулу
   let statement = client.prepare(statement_text).await;

   // Если запрос подготовлен успешно, выполняем его
   match statement {
      Ok(stmt) => {
         let rows = match by {
            TicketBy::TicketId(id) => client.query_one(&stmt, &[&id]).await,
            TicketBy::EaterAndCatererId(eater_id, caterer_id) => client.query_one(&stmt, &[&eater_id, &caterer_id]).await,
         };

         // Возвращаем результат
         match rows {
            Ok(row) => Some(Ticket::from_db(&row)),
            Err(e) => {
               // Сообщаем об ошибке и возвращаем пустой результат
               settings::log(&format!("db::ticket: {}", e)).await;
               None
            }
         }
      }
      Err(e) => {
         // Сообщаем об ошибке и возвращаем пустой результат
         settings::log(&format!("db::ticket prepare: {}", e)).await;
         None
      }
   }
}

// Сохраняет ссылки на сообщения со статусом для последующего редактирования при изменении тикета
pub async fn ticket_save_status_msg(ticket_id: i32, eater_status_msg_id: i32, caterer_status_msg_id: i32) -> bool {
   execute_one("UPDATE tickets SET eater_status_msg_id = $1::INTEGER, caterer_status_msg_id = $2::INTEGER WHERE ticket_id=$3::INTEGER", &[&eater_status_msg_id, &caterer_status_msg_id, &ticket_id])
   .await
}

// Изменяет стадию заказа
pub async fn basket_edit_stage(ticket_id: i32, stage: i32) -> bool {
   execute_one("UPDATE tickets SET stage = $1::INTEGER WHERE ticket_id=$2::INTEGER AND stage < 5", &[&stage, &ticket_id])
   .await
}

// Увеличивает стадию заказа
pub async fn basket_next_stage(user_id: i32, ticket_id: i32) -> bool {
   // Выполняем запрос, статус ещё должен быть незавешённым
   execute_one("UPDATE tickets SET stage = stage + 1 WHERE ticket_id=$1::INTEGER AND stage < 5 AND (stage != 4 OR caterer_id != $2::INTEGER)", &[&ticket_id, &user_id])
   .await
}

// Возвращает стадию заказа
pub async fn basket_stage(ticket_id: i32) -> i32 {
   // Выполняем запрос, статус ещё должен быть незавешённым
   let query = DB.get().unwrap().get().await.unwrap()
   .query_one("SELECT stage FROM tickets WHERE ticket_id=$1::INTEGER", &[&ticket_id])
   .await;
   match query {
      Ok(data) => data.get(0),
      Err(e) => {
         settings::log(&format!("Error db::basket_stage: {}", e)).await;
         0
      }
   }
}


// ============================================================================
// [Misc]
// ============================================================================
// Для отображения статуса
pub fn active_to_str(active : bool) -> &'static str {
   if active {
       "показывается"
   } else {
       "скрыт"
   }
}

pub fn enabled_to_str(enabled : bool) -> &'static str {
   if enabled {
       "доступен"
   } else {
       "в бане"
   }
}

pub fn enabled_to_cmd(enabled : bool) -> &'static str {
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
pub fn id_to_category(cat_id : i32) -> &'static str {
   match cat_id {
      1 => "Соки воды",
      2 => "Еда",
      3 => "Напитки",
      4 => "Развлечения",
      _ => "Неизвестная категория",
   }
} 

pub fn category_to_id(category: &str) -> i32 {
   match category {
      "Соки воды" => 1,
      "Еда" => 2,
      "Напитки" => 3,
      "Развлечения" => 4,
      _ => 0,
   }
}

// Режим интерфейса
pub fn interface_mode(is_compact: bool) -> String {
   if is_compact {
      String::from("со ссылками")
   } else {
      String::from("с кнопками")
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


// Возвращает название стадии
pub fn stage_to_str(stage: i32) -> String {
   let res = match stage {
      1 => "Ожидание подтверждения",
      2 => "В процессе приготовления",
      3 => "Готово, идёт доставка",
      4 => "Подтвердить получение и закрыть заказ",
      5 => "Завершено",
      6 => "Отменено",
      _ => "Ошибка",
   };
   String::from(res)
}

// Удаляет минуты из времени, если они нулевые
pub fn str_time(time: NaiveTime) -> String {
   if time.minute() == 0 {
      time.format("%H").to_string()
   } else {
      time.format("%H:%M").to_string()
   }
}

// Обёртка, выполняет запрос, обновляющий 1 запись и возвращает истину, если успешно
async fn execute_one(sql_text: &str, params: &[&(dyn ToSql + Sync)]) -> bool {
   // Получим клиента БД из пула
   let client = db_client().await;
   if client.is_none() {return false;}

   // Выполняем запрос
   let query = client.unwrap().execute(sql_text, params).await;

   // При успешной операции должна быть обновлена 1 запись
   match query {
      Ok(1) => true,
      Ok(n) => {
         settings::log(&format!("db::execute({}): updated {} records instead one", sql_text, n)).await;
         false
      }
      Err(e) => {
         settings::log(&format!("db::execute({}): {}", sql_text, e)).await;
         false
      }
   }
}

// Подобна execute_one(), но не выводит сообщений об ошибках
async fn execute_one_no_error(sql_text: &str, params: &[&(dyn ToSql + Sync)]) -> bool {
   // Получим клиента БД из пула
   let client = db_client().await;
   if client.is_none() {return false;}

   // Выполняем запрос
   let query = client.unwrap().execute(sql_text, params).await;

   // При успешной операции должна быть обновлена 1 запись
   match query {
      Ok(1) => true,
      _ => false,
   }
}


// Обёртка, выполняет запрос, без проверки на обновление только одной записи и возвращает истину, если успешно
async fn execute(sql_text: &str, params: &[&(dyn ToSql + Sync)]) -> bool {
   // Получим клиента БД из пула
   let client = db_client().await;
   if client.is_none() {return false;}

   // Выполняем запрос
   let query = client.unwrap().execute(sql_text, params).await;

   // При успешной операции должна быть обновлена 1 запись
   match query {
      Ok(_) => true,
      Err(e) => {
         settings::log(&format!("db::execute({}): {}", sql_text, e)).await;
         false
      }
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
   if let Err(_) = CI.set(hash) {
      settings::log(&format!("Error db::cat_image_init2")).await;
   }
}

// Возвращает картинку для категории
pub fn cat_image(cat_id: i32) -> String {
   if let Some(lock) = CI.get() {
      lock.read().unwrap().get(&cat_id).unwrap().to_owned()
   } else {
      settings::default_photo_id()
   }
}

// Сохраняет новую картинку для категории
pub async fn save_cat_image(cat_id: i32, image_id: String) {
   if let Some(lock) = CI.get() {
      let mut hash = lock.write().unwrap();
      hash.insert(id, image);    
   }

   // Поробуем обновить запись
   if !execute_one_no_error("UPDATE category SET image_id = $1::VARCHAR(512) WHERE cat_id=$2::INTEGER", &[&image_id, &cat_id]).await {
      // Если не получилось, вставляем новую
      execute_one("INSERT INTO category(cat_id, image_id) VALUES ($1::INTEGER, $2::VARCHAR(512))", &[&cat_id, &image_id]).await;
   }
}