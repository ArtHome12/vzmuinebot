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

fn active_to_str(active : bool) -> &'static str {
    if active {
        "показывается"
    } else {
        "скрыт"
    }
}

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


struct Restaurant {
    //id: i32,
    title: String,
    info: String,
    active: bool,
    groups: BTreeMap<i32, Group>
}

use once_cell::sync::Lazy;
use std::sync::Mutex;


static REST_DB: Lazy<Mutex<Restaurant>> = Lazy::new(|| {

    let group1 = Group {
        title: String::from("Основная"),
        info: String::from("Блюда подаются на тарелке"),
        active: true,
        cat_id: 1,
        opening_time: NaiveTime::from_hms(0, 0, 0),
        closing_time: NaiveTime::from_hms(0, 0, 0),
    };

    let group2 = Group {
        title: String::from("Завтраки"),
        info: String::from("Имеются салфетки"),
        active: true,
        cat_id: 1,
        opening_time: NaiveTime::from_hms(7, 0, 0),
        closing_time: NaiveTime::from_hms(11, 0, 0),
    };

    let mut map = BTreeMap::new();
    map.insert(1, group1);
    map.insert(2, group2);
    
    Mutex::new(Restaurant {
        //id: 0,
        title: String::from("Хинкалий"),
        info: String::from("Наш адрес 00NDC, доставка @nick, +84123"),
        active: true,
        groups: map,
    })
});


impl Restaurant {
    fn to_str(&self) -> String {
        // Информация о ресторане
        let mut s = String::from(format!("Название: {} /EditTitle\nОписание: {} /EditInfo\nСтатус: {} /Toggle\nГруппы и время работы (добавить новую /AddGroup):\n",
            self.title, self.info, active_to_str(self.active)));

        // Добавим информацию о группах
        for (key, value) in &self.groups {
            s.push_str(&format!("   {}{}\n", value.to_str_short(), key));
        };
        s
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


struct Group {
    //id: i32,
    title: String,
    info: String,
    active: bool,
    cat_id: i32,
    opening_time: NaiveTime,
    closing_time: NaiveTime,    
}

impl Group {

    fn to_str(&self) -> String {
        String::from(format!("Название: {} /EditTitle\nДоп.инфо: {} /EditInfo\nКатегория: {} /EditCat\nСтатус: {} /Toggle\nВремя: {}-{} /EditTime
Удалить группу /Remove\nНовое блюдо /AddDish\nАджапсандали /EdDi1\nКиндзмараули /EdDi2\nГварцители /EdDi3",
            self.title, self.info, id_to_category(self.cat_id), active_to_str(self.active), self.opening_time.format("%H:%M"), self.closing_time.format("%H:%M")))
    }

    fn to_str_short(&self) -> String {
        String::from(format!("{} {}-{} /EdGr", self.title, self.opening_time.format("%H:%M"), self.closing_time.format("%H:%M")))
    }

    fn toggle(&mut self) {
        self.active = !self.active; 
    }
}

pub async fn group_info(_rest_id: i32, group_id: i32) -> String {
    if let Some(group) = REST_DB.lock().unwrap().groups.get(&group_id) {
        group.to_str()
    } else {
        String::from("")
    }
}

pub async fn rest_add_group(_rest_id: i32, new_str: String) {
    let group = Group {
        title: new_str,
        info: String::from("Блюда подаются на тарелке"),
        active: true,
        cat_id: 1,
        opening_time: NaiveTime::from_hms(0, 0, 0),
        closing_time: NaiveTime::from_hms(0, 0, 0),
    };

    let groups = & mut(REST_DB.lock().unwrap().groups);
    let group_id = groups.len() as i32 + 1;
    groups.insert(group_id, group);
}

pub async fn rest_group_edit_title(_rest_id: i32, group_id: i32, new_str: String) {
    if let Some(group) = REST_DB.lock().unwrap().groups.get_mut(&group_id) {
        group.title = new_str;
    }
}

pub async fn rest_group_edit_info(_rest_id: i32, group_id: i32, new_str: String) {
    if let Some(group) = REST_DB.lock().unwrap().groups.get_mut(&group_id) {
        group.info = new_str;
    }
}

pub async fn rest_group_toggle(_rest_id: i32, group_id: i32) {
    if let Some(group) = REST_DB.lock().unwrap().groups.get_mut(&group_id) {
        group.toggle();
    }
}

pub async fn rest_group_edit_category(_rest_id: i32, group_id: i32, new_cat : i32) {
    if let Some(group) = REST_DB.lock().unwrap().groups.get_mut(&group_id) {
        group.cat_id = new_cat;
    }
}

pub async fn rest_group_edit_time(_rest_id: i32, group_id: i32, opening_time: NaiveTime, closing_time: NaiveTime) {
    if let Some(group) = REST_DB.lock().unwrap().groups.get_mut(&group_id) {
        group.opening_time = opening_time;
        group.closing_time = closing_time;
    }
}

pub async fn rest_group_remove(_rest_id: i32, group_id: i32) {
    let groups = & mut(REST_DB.lock().unwrap().groups);
    
    // Первую группу не удаляем
    if group_id > 1 {
        groups.remove(&group_id);
    }
}


//
// Dish
//
pub async fn dish_info(_rest_id: i32, dish_id: i32) -> String {
    String::from("")
}

pub async fn dish_group(_rest_id: i32, dish_id: i32) -> i32 {
    0
}

pub async fn rest_add_dish(_rest_id: i32, new_str: String) {
}

pub async fn rest_dish_edit_title(_rest_id: i32, _dish_id: i32, _new_str: String) {
}

pub async fn rest_dish_edit_info(_rest_id: i32, _dish_id: i32, _new_str: String) {
}

pub async fn rest_dish_toggle(_rest_id: i32, _dish_id: i32) {
}

pub async fn rest_dish_edit_group(_rest_id: i32, _dish_id: i32, _new_cat : i32) {
}

pub async fn rest_dish_remove(_rest_id: i32, _dish_id: i32) {
}

