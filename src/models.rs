/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Модуль связи с СУБД. 22 May 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use crate::schema::menu_items;


#[derive(Queryable)]
pub struct MenuItems {
    pub id: i32,
    pub title: String,
    pub price: i32,
    pub category: String,
}

#[derive(Insertable)]
#[table_name = "menu_items"]
pub struct NewMenuItem<'a> {
    pub title: &'a str,
    //pub price: &'a diesel::sql_types::Integer,
    pub category: &'a str,
}