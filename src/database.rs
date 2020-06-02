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

// ============================================================================
// [Caterer]
// ============================================================================
pub async fn is_rest_owner(user_id : i32) -> bool {
    user_id == 409664508 || user_id == 501159140
}

pub struct Restaurant {
    //id: i32,
    title: String,
    info: String,
    active: bool,
}

use once_cell::sync::Lazy;
use std::sync::Mutex;


static REST_DB: Lazy<Mutex<Restaurant>> = Lazy::new(|| {
    Mutex::new(Restaurant {
        //id: 0,
        title: String::from("Хинкал"),
        info: String::from("Наш адрес 00NDC, доставка @nick, +84123"),
        active: true,
    })
});



//pub static REST_DB: OnceCell<Mutex<Restaurant>> = OnceCell::new();

impl Restaurant {
    fn to_str(&self) -> String {
        String::from(format!("Название: {} /EditTitle\nОписание: {} /EditInfo\nСтатус: {} /Toggle\nГруппы и время работы (добавить новую /AddGroup):\n   Основная группа 07:00-23:00 /EdGr1\n   Завтраки 07:00-11:00 /EdGr2",
            self.title, self.info, self.active))
    }

    fn set_title(&mut self, new_title : String) {
        self.title = new_title;
    }

    fn toggle(&mut self) {
        self.active = !self.active; 
    }
}

pub async fn rest_info(_rest_id: i32) -> String {
    REST_DB.lock().unwrap().to_str()
}

pub async fn rest_edit_title(_rest_id: i32, new_str: String) {
    REST_DB.lock().unwrap().set_title(new_str);
}

pub async fn rest_edit_info(_rest_id: i32, new_str: String) {
    REST_DB.lock().unwrap().info = new_str;
}

pub async fn rest_toggle(_rest_id: i32) {
    REST_DB.lock().unwrap().toggle();
}

/*    String::from("
Название: Хинкал /EditTitle
Описание: Наш адрес 00NDC, доставка @nick, +84123 /EditInfo
Статус: работаем /Toggle
Группы и время работы (добавить новую /AddGroup):
   Основная группа 07:00-23:00 /EdGr1
   Завтраки 07:00-11:00 /EdGr2
")*/




pub async fn group_info(_rest_id: i32, _gproup_id: i32) -> String {
    String::from("
Название: Основная /EditTitle
Доп.инфо: Блюда подаются на тарелке /EditInfo
Категория: Еда /EditCategory
Статус: показывать /Toggle
Время: 00:00-00:00 /EditTime
Удалить группу /Delete
Новое блюдо /AddDish
Хинкали /EdDi1
Киндзмараули /EdDi2
Гварцители /EdDi3
")
}

/*pub async fn rest_edit_group(_rest_id: i32, _category_id: i32, _group_id: i32, _new_str: String) {

}*/

pub async fn rest_add_group(_rest_id: i32, _new_str: String) {

}

pub async fn rest_group_edit_title(_rest_id: i32, _group_id: i32, _new_str: String) {

}

pub async fn rest_group_edit_info(_rest_id: i32, _group_id: i32, _new_str: String) {

}

pub async fn rest_group_toggle(_rest_id: i32, _group_id: i32) {

}

