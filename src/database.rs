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
use std::collections::{HashMap, BTreeMap};
use teloxide::types::InputFile;

use once_cell::sync::Lazy;
use std::sync::Mutex;

pub static DB: OnceCell<tokio_postgres::Client> = OnceCell::new();


fn restaurants() -> &'static HashMap<u32, &'static str> {
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
}

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




pub async fn restaurant_by_category_from_db(_category: String) -> String {
    let mut res = String::default();
    let hash = restaurants();
    for (key, value) in hash {
        let res1 = format!("\n   {} /rest0{}", value, key);
        res.push_str(&res1);
    }
    res
}
    
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
// [Caterer]
// ============================================================================
struct Restaurant {
    //id: i32,
    //title: String,
    //info: String,
    //active: bool,
    groups: BTreeMap<i32, Group>,
    dishes: BTreeMap<i32, Dish>,
}


static REST_DB: Lazy<Mutex<Restaurant>> = Lazy::new(|| {

    let dish1 = Dish {
        title: String::from("Борщ"),
        info: String::from("Со сметаной и укропчиком, среднего размера тарелка"),
        active: true,
        group_id: 1,
        price: 0,
        image_id: Option::None,
    };

    let dish2 = Dish {
        title: String::from("Картофельное пюре"),
        info: String::from("С маслицем и зеленью, порция 100гр."),
        active: true,
        group_id: 1,
        price: 0,
        image_id: Option::None,
    };

    let dish3 = Dish {
        title: String::from("Мясо по-французски"),
        info: String::from("Заморское блюдо, порция 100гр."),
        active: true,
        group_id: 1,
        price: 0,
        image_id: Option::None,
    };

    let dish4 = Dish {
        title: String::from("Шарлотка"),
        info: String::from("Хороша, когда больше есть нечего, порция 1 кусочек"),
        active: true,
        group_id: 1,
        price: 0,
        image_id: Option::None,
    };

    let dish5 = Dish {
        title: String::from("Чай"),
        info: String::from("Без сахара в маленькой чашке"),
        active: true,
        group_id: 2,
        price: 0,
        image_id: Option::None,
    };

    let group1 = Group {
      //   title: String::from("Основная"),
      //   info: String::from("Блюда подаются на тарелке"),
      //   active: true,
      //   cat_id: 2,
      //   opening_time: NaiveTime::from_hms(0, 0, 0),
      //   closing_time: NaiveTime::from_hms(0, 0, 0),
    };

    let group2 = Group {
      //   title: String::from("Завтраки"),
      //   info: String::from("Имеются салфетки"),
      //   active: true,
      //   cat_id: 1,
      //   opening_time: NaiveTime::from_hms(7, 0, 0),
      //   closing_time: NaiveTime::from_hms(11, 0, 0),
    };

    let mut rest = Restaurant {
        //id: 0,
        //title: String::from("Хинкалий"),
        //info: String::from("Наш адрес 00NDC, доставка @nick, +84123"),
        //active: true,
        groups: BTreeMap::new(),
        dishes: BTreeMap::new(),
    };

    rest.dishes.insert(1, dish1);
    rest.dishes.insert(2, dish2);
    rest.dishes.insert(3, dish3);
    rest.dishes.insert(4, dish4);
    rest.dishes.insert(5, dish5);
    
    rest.groups.insert(1, group1);
    rest.groups.insert(2, group2);

    Mutex::new(rest)
});


struct Group {
   //  title: String,
   //  info: String,
   //  active: bool,
   //  cat_id: i32,
   //  opening_time: NaiveTime,
   //  closing_time: NaiveTime,    
}

//
// Dish
//
struct Dish {
    title: String,
    info: String,
    active: bool,
    group_id: i32,
    price: u32,
    image_id: Option::<String>,
}

impl Dish {

    fn to_str(&self) -> String {
        String::from(format!("Название: {} /EditTitle\nДоп.инфо: {} /EditInfo\nГруппа: {} /EditGroup\nСтатус: {} /Toggle\nЦена: {} /EditPrice\nЗагрузить фото /EditImg\nУдалить блюдо /Remove",
            self.title, self.info, self.group_id, active_to_str(self.active), self.price))
    }

    /*fn to_str_short(&self) -> String {
        String::from(format!("{} {}k₫", self.title, self.price))
    }*/

    fn toggle(&mut self) {
        self.active = !self.active; 
    }
}

pub async fn dish_info(_rest_id: i32, dish_id: i32) -> String {
    if let Some(dish) = REST_DB.lock().unwrap().dishes.get(&dish_id) {
        dish.to_str()
    } else {
        String::from("")
    }
}

pub async fn rest_add_dish(_rest_id: i32, group_id: i32, new_str: String) {
    let dish = Dish {
        title: new_str,
        info: String::from("Порция 100гр."),
        active: true,
        group_id,
        price: 0,
        image_id: Option::None,
    };

    let dishes = & mut(REST_DB.lock().unwrap().dishes);
    let dish_id = dishes.len() as i32 + 1;
    dishes.insert(dish_id, dish);
}

pub async fn rest_dish_edit_title(_rest_id: i32, dish_id: i32, new_str: String) {
    if let Some(dish) = REST_DB.lock().unwrap().dishes.get_mut(&dish_id) {
        dish.title = new_str;
    }
}

pub async fn rest_dish_edit_info(_rest_id: i32, dish_id: i32, new_str: String) {
    if let Some(dish) = REST_DB.lock().unwrap().dishes.get_mut(&dish_id) {
        dish.info = new_str;
    }
}

pub async fn rest_dish_toggle(_rest_id: i32, dish_id: i32) {
    if let Some(dish) = REST_DB.lock().unwrap().dishes.get_mut(&dish_id) {
        dish.toggle();
    }
}

pub async fn rest_dish_edit_group(_rest_id: i32, dish_id: i32, group_id : i32) -> bool {
    // Ресторан
    let mut rest = REST_DB.lock().unwrap();

    // Проверим, есть ли такая группа
    if rest.groups.get(&group_id).is_some() {
        // Обновим код группы у блюда
        if let Some(dish) = rest.dishes.get_mut(&dish_id) {
            dish.group_id = group_id;
            return true;
        }
    }
        false
}

pub async fn rest_dish_remove(_rest_id: i32, dish_id: i32) {
    let dishes = & mut(REST_DB.lock().unwrap().dishes);
    dishes.remove(&dish_id);
}

// Возвращает группу блюда
pub async fn dish_group(_rest_id: i32, dish_id: i32) -> i32 {
    if let Some(dish) = REST_DB.lock().unwrap().dishes.get(&dish_id) {
        dish.group_id
    } else {
        0
    }
}

pub async fn rest_dish_edit_price(_rest_id: i32, dish_id: i32, price: u32) {
    if let Some(dish) = REST_DB.lock().unwrap().dishes.get_mut(&dish_id) {
        dish.price = price;
    }
}

pub async fn rest_dish_edit_image(_rest_id: i32, dish_id: i32, image_id: &String) {
    if let Some(dish) = REST_DB.lock().unwrap().dishes.get_mut(&dish_id) {
        dish.image_id = Option::Some(image_id.to_string());
    }
}

pub async fn dish_image(_rest_id: i32, dish_id: i32) -> Option::<String> {
    match REST_DB.lock().unwrap().dishes.get_mut(&dish_id) {
        Some(dish) => dish.image_id.clone(),
        _ => None,
    }
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
    user_id     INTEGER         NOT NULL,
    title       VARCHAR(100)    NOT NULL,
    info        VARCHAR(255)    NOT NULL,
    active      BOOLEAN         NOT NULL,
    image_id    VARCHAR(100)
);

INSERT INTO restaurants (user_id, title, info, active)
VALUES (409664508, 'Плакучая ива', 'Наш адрес 00NDC, доставка @nick, +84123', FALSE),
       (501159140, 'Плакучая ива', 'Наш адрес 00NDC, доставка @nick, +84123', FALSE);*/

// Возвращает истину, если пользователю разрешён доступ в режим ресторатора
//
pub async fn is_rest_owner(user_id : i32) -> bool {
    // Выполняем запрос
    let rows = DB.get().unwrap()
        .query("SELECT * FROM restaurants WHERE user_id=$1::INTEGER", &[&user_id])
        .await;

    // Проверяем результат
    match rows {
        Ok(data) => !data.is_empty(),
        _ => false,
    }
//    user_id == 409664508 || user_id == 501159140
}


// Возвращает строку с информацией о ресторане
//
pub async fn rest_info(rest_id: i32) -> Option<(String, Option<String>)> {
    // Выполняем запрос
    let rows = DB.get().unwrap()
        .query("SELECT title, info, active, image_id FROM restaurants WHERE user_id=$1::INTEGER", &[&rest_id])
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
                let groups: String = group_titles(rest_id).await;
                Some((
                    String::from(format!("Название: {} /EditTitle\nОписание: {} /EditInfo\nСтатус: {} /Toggle\nГруппы и время работы (добавить новую /AddGroup):\n{}",
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

pub async fn rest_edit_title(rest_id: i32, new_str: String) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE restaurants SET title = $1::VARCHAR(100) WHERE user_id=$2::INTEGER", &[&new_str, &rest_id])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}

pub async fn rest_edit_info(rest_id: i32, new_str: String) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE restaurants SET info = $1::VARCHAR(255) WHERE user_id=$2::INTEGER", &[&new_str, &rest_id])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}

pub async fn rest_toggle(rest_id: i32) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE restaurants SET active = NOT active WHERE user_id=$1::INTEGER", &[&rest_id])
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
    PRIMARY KEY (user_id, group_num),
    user_id         INTEGER         NOT NULL,
    group_num       INTEGER         NOT NULL,
    title           VARCHAR(100)    NOT NULL,
    info            VARCHAR(255)    NOT NULL,
    active          BOOLEAN         NOT NULL,
    cat_id          INTEGER         NOT NULL,
    opening_time    TIME            NOT NULL,    
    closing_time    TIME            NOT NULL  
);

INSERT INTO groups (user_id, group_num, title, info, active, cat_id, opening_time, closing_time)
VALUES (409664508, 1, 'Основная', 'Блюда подаются на тарелке', TRUE, 2, '00:00', '00:00'),
       (501159140, 1, 'Основная', 'Блюда подаются на тарелке', TRUE, 2, '00:00', '00:00');*/

// Возвращает строки с краткой информацией о группах
//
async fn group_titles(rest_id: i32) -> String {
    // Выполняем запрос
    let rows = DB.get().unwrap()
        .query("SELECT group_num, title, opening_time, closing_time FROM groups WHERE user_id=$1::INTEGER", &[&rest_id])
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
pub async fn group_info(rest_id: i32, group_id: i32) -> Option<String> {
    
     // Выполняем запрос
     let rows = DB.get().unwrap()
     .query("SELECT title, info, active, cat_id, opening_time, closing_time FROM groups WHERE user_id=$1::INTEGER AND group_num=$2::INTEGER", &[&rest_id, &group_id])
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
                let dishes: String = dish_titles(rest_id, group_id).await;
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

pub async fn rest_add_group(rest_id: i32, new_str: String) -> bool {
   
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("INSERT INTO groups (user_id, group_num, title, info, active, cat_id, opening_time, closing_time) 
   VALUES (
      $1::INTEGER, 
      (SELECT COUNT(*) FROM groups WHERE user_id=$2::INTEGER) + 1,
      $3::VARCHAR(100),
      'Блюда подаются на тарелке',
      TRUE,
      2,
      '00:00',
      '00:00'
   )", &[&rest_id, &rest_id, &new_str])
   .await;
   match query {
      Ok(_) => true,
      _ => false,
   }
}

pub async fn rest_group_edit_title(rest_id: i32, group_id: i32, new_str: String) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE groups SET title = $1::VARCHAR(100) WHERE user_id=$2::INTEGER AND group_num=$3::INTEGER", &[&new_str, &rest_id, &group_id])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}

pub async fn rest_group_edit_info(rest_id: i32, group_id: i32, new_str: String) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE groups SET info = $1::VARCHAR(255) WHERE user_id=$2::INTEGER AND group_num=$3::INTEGER", &[&new_str, &rest_id, &group_id])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}

pub async fn rest_group_toggle(rest_id: i32, group_id: i32) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE groups SET active = NOT active WHERE user_id=$1::INTEGER AND group_num=$2::INTEGER", &[&rest_id, &group_id])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}

pub async fn rest_group_edit_category(rest_id: i32, group_id: i32, new_cat : i32) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE groups SET cat_id = $1::INTEGER WHERE user_id=$2::INTEGER AND group_num=$3::INTEGER", &[&new_cat, &rest_id, &group_id])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}

pub async fn rest_group_edit_time(rest_id: i32, group_id: i32, opening_time: NaiveTime, closing_time: NaiveTime) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("UPDATE groups SET opening_time = $1::TIME, closing_time = $2::TIME WHERE user_id=$3::INTEGER AND group_num=$4::INTEGER", &[&opening_time, &closing_time, &rest_id, &group_id])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}

pub async fn rest_group_remove(rest_id: i32, group_id: i32) -> bool {
   // Выполняем запрос
   let query = DB.get().unwrap()
   .execute("DELETE FROM groups WHERE user_id=$1::INTEGER AND group_num=$2::INTEGER;
   UPDATE groups SET group_num = group_num - 1 WHERE user_id=$3::INTEGER AND group_num>$4::INTEGER
   ", &[&rest_id, &group_id, &rest_id, &group_id])
   .await;
   match query {
       Ok(_) => true,
       _ => false,
   }
}


// ============================================================================
// [Dish]
// ============================================================================
/*Таблица с данными о блюдах
CREATE TABLE dishes (
    PRIMARY KEY (user_id, dish_num),
    user_id         INTEGER         NOT NULL,
    dish_num        INTEGER         NOT NULL,
    title           VARCHAR(100)    NOT NULL,
    info            VARCHAR(255)    NOT NULL,
    active          BOOLEAN         NOT NULL,
    group_id        INTEGER         NOT NULL,
    price           INTEGER         NOT NULL,
    image_id        VARCHAR(100)
);

*/

// Возвращает строки с краткой информацией о группах
//
async fn dish_titles(rest_id: i32, group_id: i32) -> String {
    // Выполняем запрос
    let rows = DB.get().unwrap()
        .query("SELECT dish_num, title, price FROM dishes WHERE user_id=$1::INTEGER AND group_id=$2::INTEGER", &[&rest_id, &group_id])
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
