/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Модуль для связи с СУБД. 28 May 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

extern crate once_cell;

use once_cell::sync::{OnceCell};
use std::collections::HashMap;
use teloxide::types::InputFile;



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

pub async fn is_rest_owner(user_id : i32) -> bool {
    user_id == 409664508 || user_id == 501159140
}

pub async fn restaurant_info(user_id: i32) -> String {
    String::from("
Название: Название /EditTitle
Описание: Наш адрес 00NDC, доставка @nick, +84123456789 /EditInfo
Статус: работаем /Toggle
Группы и время работы (добавить новую /AddGroup):
   Основная группа 07:00-23:00 /EditGroup
   Завтраки 07:00-11:00 /EdGr1
")
}