/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Модуль для связи с СУБД. 28 May 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use chrono::{NaiveTime};

use once_cell::sync::{OnceCell};
use std::collections::{HashMap};
use teloxide::types::InputFile;


pub static DB: OnceCell<tokio_postgres::Client> = OnceCell::new();


/*fn restaurants() -> &'static HashMap<u32, &'static str> {
    static INSTANCE: OnceCell<HashMap<u32, &'static str>> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert(1, "Ёлки-палки");
        m.insert(2, "Крошка-картошка");
        m.insert(3, "Плакучая ива");
        m.insert(4, "Националь");
        m.insert(5, "Му-му");
        m
    })
}*/

fn dishes() -> &'static HashMap<u32, &'static str> {
    static INSTANCE: OnceCell<HashMap<u32, &'static str>> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert(1, "Борщ");
        m.insert(2, "Картофельное пюре");
        m.insert(3, "Мясо по-французски");
        m.insert(4, "Шарлотка");
        m.insert(5, "Чай");
        m
    })
}




// ============================================================================
// [User]
// ============================================================================

// Возвращает список ресторанов с активными группами в данной категории
//
pub async fn restaurant_by_category_from_db(cat_id: i32) -> String {
   // Выполняем запрос
   let rows = DB.get().unwrap()
      .query("SELECT r.title, r.rest_num from restaurants as r INNER JOIN (SELECT DISTINCT rest_num FROM groups WHERE cat_id=$1::INTEGER) g ON r.rest_num = g.rest_num", &[&cat_id])
      .await;

   // Строка для возврата результата
   let mut res = String::default();

   // Проверяем результат
   if let Ok(data) = rows {
      for record in data {
         let title: String = record.get(0);
         let rest_num: i32 = record.get(1);
         res.push_str(&format!("   {} /rest{}\n", title, rest_num));
      }
   }

   // На случай пустого списка сообщим об этом
   if res.is_empty() {
      String::from("   пусто :(")
   } else {
      res
   }


   /*let mut res = String::default();
   let hash = restaurants();
   for (key, value) in hash {
       let res1 = format!("\n   {} /rest0{}", value, key);
       res.push_str(&res1);
   }
   res*/
}

// Возвращает список блюд выбранного ресторана и категории
//
pub async fn dishes_by_restaurant_and_category_from_db(_category: String, _restaurant: String) -> String {
   let mut res = String::default();
   let hash = dishes();
   for (key, value) in hash {
       let res1 = format!("\n   {} /dish010{}", value, key);
       res.push_str(&res1);
   }
   res
}

// Возвращает информацию о блюде - картинку, цену и описание.
pub struct DishInfo {
   pub img : InputFile,
   pub price : u32,
   pub desc : String,
}

pub async fn dish(_dish_id : String) -> Option<DishInfo> {
   let dish_info = DishInfo {
       img : InputFile::file("media/dish.jpg"),
       price : 100,
       desc : String::from("Просто пальчики оближешь"),
   };

   Some(dish_info)
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


// Используется при редактировании категории группы
//
fn id_to_category(cat_id : i32) -> &'static str {
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


// ============================================================================
// [Caterer]
// ============================================================================
/* 
Таблица с данными о ресторанах
CREATE TABLE restaurants (
    PRIMARY KEY (user_id),
    user_id     INTEGER       NOT NULL,
    title       VARCHAR(100)  NOT NULL,
    info        VARCHAR(255)  NOT NULL,
    active      BOOLEAN       NOT NULL,
    enabled     BOOLEAN       NOT NULL
    rest_num    SERIAL,
    image_id    VARCHAR(255)
);

INSERT INTO restaurants (user_id, title, info, active)
VALUES (409664508, 'Плакучая ива', 'Наш адрес 00NDC, доставка @nick, +84123', FALSE, TRUE),
       (501159140, 'Плакучая ива', 'Наш адрес 00NDC, доставка @nick, +84123', FALSE, TRUE);*/

// Возвращает номер ресторана, если пользователю разрешён доступ в режим ресторатора
//
pub async fn rest_num(user_id : i32) -> i32 {
    // Выполняем запрос
    let rows = DB.get().unwrap()
        .query("SELECT rest_num FROM restaurants WHERE user_id=$1::INTEGER AND enabled = TRUE", &[&user_id])
        .await;

    // Проверяем результат
    match rows {
        Ok(data) => if !data.is_empty() {
           data[0].get(0)
        } else {
           0
        }
        _ => 0,
    }
//    user_id == 409664508 || user_id == 501159140
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


// ============================================================================
// [Group]
// ============================================================================
/*Таблица с данными о группах
CREATE TABLE groups (
    PRIMARY KEY (rest_num, group_num),
    rest_num        INTEGER         NOT NULL,
    group_num       INTEGER         NOT NULL,
    title           VARCHAR(100)    NOT NULL,
    info            VARCHAR(255)    NOT NULL,
    active          BOOLEAN         NOT NULL,
    cat_id          INTEGER         NOT NULL,
    opening_time    TIME            NOT NULL,    
    closing_time    TIME            NOT NULL  
);

INSERT INTO groups (rest_num, group_num, title, info, active, cat_id, opening_time, closing_time)
VALUES (409664508, 1, 'Основная', 'Блюда подаются на тарелке', TRUE, 2, '00:00', '00:00'),
       (501159140, 1, 'Основная', 'Блюда подаются на тарелке', TRUE, 2, '00:00', '00:00');*/

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
                title, opening_time, closing_time, group_num
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
                    String::from(format!("Название: {} /EditTitle\nДоп.инфо: {} /EditInfo\nКатегория: {} /EditCat\nСтатус: {} /Toggle\nВремя: {}-{} /EditTime
Удалить группу /Remove\nНовое блюдо /AddDish\n{}",
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
/*Таблица с данными о блюдах
CREATE TABLE dishes (
    PRIMARY KEY (rest_num, group_num, dish_num),
    rest_num         INTEGER        NOT NULL,
    dish_num        INTEGER         NOT NULL,
    title           VARCHAR(100)    NOT NULL,
    info            VARCHAR(255)    NOT NULL,
    active          BOOLEAN         NOT NULL,
    group_num       INTEGER         NOT NULL,
    price           INTEGER         NOT NULL,
    image_id        VARCHAR(255)
);

*/

// Возвращает строки с краткой информацией о блюдах
//
async fn dish_titles(rest_num: i32, group_num: i32) -> String {
    // Выполняем запрос
    let rows = DB.get().unwrap()
        .query("SELECT dish_num, title, price FROM dishes WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER", &[&rest_num, &group_num])
        .await;

    // Строка для возврата результата
    let mut res = String::default();

    // Проверяем результат
    if let Ok(data) = rows {
        for record in data {
            let dish_num: i32 = record.get(0);
            let title: String = record.get(1);
            let price: i32 = record.get(2);
            res.push_str(&format!("   {} {}k₫ /EdDi{}\n", 
                title, price, dish_num
            ));
        }
    }
    res
}


// Возвращает информацию о блюде
//
pub async fn dish_info(rest_num: i32, dish_num: i32) -> Option<(String, Option<String>)> {
     // Выполняем запрос
     let rows = DB.get().unwrap()
     .query("SELECT title, info, active, group_num, price, image_id FROM dishes WHERE rest_num=$1::INTEGER AND dish_num=$2::INTEGER", &[&rest_num, &dish_num])
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
                  String::from(format!("Название: {} /EditTitle\nДоп.инфо: {} /EditInfo\nГруппа: {} /EditGroup\nСтатус: {} /Toggle\nЦена: {} k₫ /EditPrice\nЗагрузить фото /EditImg\nУдалить блюдо /Remove",
                  title, info, group_num, active_to_str(active), price)), image_id
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
pub async fn rest_dish_edit_title(rest_num: i32, dish_num: i32, new_str: String) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE dishes SET title = $1::VARCHAR(100) WHERE rest_num=$2::INTEGER AND dish_num=$3::INTEGER", &[&new_str, &rest_num, &dish_num])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}


// Редактирование описания блюда
//
pub async fn rest_dish_edit_info(rest_num: i32, dish_num: i32, new_str: String) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE dishes SET info = $1::VARCHAR(255) WHERE rest_num=$2::INTEGER AND dish_num=$3::INTEGER", &[&new_str, &rest_num, &dish_num])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}


// Переключение доступности блюда
//
pub async fn rest_dish_toggle(rest_num: i32, dish_num: i32) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE dishes SET active = NOT active WHERE rest_num=$1::INTEGER AND dish_num=$2::INTEGER", &[&rest_num, &dish_num])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}


// Изменение группы блюда
//
pub async fn rest_dish_edit_group(rest_num: i32, dish_num: i32, group_num : i32) -> bool {
   // Проверим, что есть такая группа
   let rows = DB.get().unwrap()
   .query("SELECT * FROM groups WHERE rest_num=$1::INTEGER AND group_num=$2::INTEGER", &[&rest_num, &group_num])
   .await;

   // Если код группы действителен, сохраняем его
   match rows {
      Ok(data) => {
            if !data.is_empty() {
            // Выполняем запрос
            let query = DB.get().unwrap()
            .execute("UPDATE dishes SET group_num = $1::INTEGER WHERE rest_num=$2::INTEGER AND dish_num=$3::INTEGER", &[&group_num, &rest_num, &dish_num])
            .await;
            match query {
               Ok(_) => true,
               _ => false,
            }
         } else {
            false
         }
      }
      _ => false,
   }
}


// Удаление блюда
//
pub async fn rest_dish_remove(rest_num: i32, dish_num: i32) -> bool {
   // Выполняем запрос. Должно быть начало транзакции, потом коммит, но transaction требует mut
   let query = DB.get().unwrap()
   .execute("DELETE FROM dishes WHERE rest_num=$1::INTEGER AND dish_num=$2::INTEGER", &[&rest_num, &dish_num])
   .await;
   match query {
      Ok(_) => {
         // Номера оставшихся блюд перенумеровываем для исключения дырки
         let query = DB.get().unwrap()
         .execute("UPDATE dishes SET dish_num = dish_num - 1 WHERE rest_num=$1::INTEGER AND dish_num>$2::INTEGER", &[&rest_num, &dish_num])
         .await;
         match query {
         Ok(_) => true,
         _ => false,
         }
      }
      _ => false,
   }
 }


// Возвращает группу блюда
//
pub async fn dish_group(rest_num: i32, dish_num: i32) -> i32 {
   let rows = DB.get().unwrap()
   .query("SELECT group_num FROM dishes WHERE rest_num=$1::INTEGER AND dish_num=$2::INTEGER", &[&rest_num, &dish_num])
   .await;

   // Возвращаем код группы
   match rows {
      Ok(data) => {
         if !data.is_empty() {
            data[0].get(0)
         } else {
            1
         }
      }
      _ => 1,
   }
}


// Изменение цены блюда
//
pub async fn rest_dish_edit_price(rest_num: i32, dish_num: i32, price: i32) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE dishes SET price = $1::INTEGER WHERE rest_num=$2::INTEGER AND dish_num=$3::INTEGER", &[&price, &rest_num, &dish_num])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}


// Изменение фото блюда
//
pub async fn rest_dish_edit_image(rest_num: i32, dish_num: i32, image_id: &String) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE dishes SET image_id = $1::VARCHAR(100) WHERE rest_num=$2::INTEGER AND dish_num=$3::INTEGER", &[&image_id, &rest_num, &dish_num])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}

