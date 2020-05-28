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

/*static RESTAURANTS: Lazy<HashMap<u32, String>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(1, "Ёлки-палки".to_string());
    m.insert(2, "Крошка-картошка".to_string());
    m.insert(3, "Плакучая ива".to_string());
    m.insert(4, "Националь".to_string());
    m.insert(5, "Му-му".to_string());
    m
});*/

fn hashmap() -> &'static HashMap<u32, &'static str> {
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




pub async fn restaurant_by_category_from_db(_category: String) -> String {
    /*    String::from("
            Ёлки-палки /rest01
            Крошка-картошка /rest02
            Плакучая ива /rest03
            Националь /rest04
            Хинкал /rest05"
    )*/

    let mut res = String::default();
    let hash = hashmap();
    for (key, value) in hash {
        let res1 = format!("{}: {}\n", key, value);
        res.push_str(&res1);
    }
    res
}
    