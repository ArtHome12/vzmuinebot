/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Локализация. 19 June 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

// use once_cell::sync::{OnceCell};
// use std::collections::HashMap;

// pub static LANG: OnceCell<HashMap<String, HashMap<String, String>>> = OnceCell::new();

pub enum Res {
   DatabaseEmpty, // "   пусто :("
   DatabaseRestInfo, // "Заведение: {}\nОписание: {}\nПодходящие разделы меню для {}:\n{}"
}

// Возвращает шаблон на нужном языке
//
pub fn t(_lang: &str, resource: Res) -> String {
   match resource {
      DatabaseEmpty => String::from("   пусто :("),
      DatabaseRestInfo => String::from("Заведение: {}\nОписание: {}\nПодходящие разделы меню для {}:\n{}"),
   }
}