/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Обработка нажатий на инлайн-кнопки. 14 June 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{
   prelude::*, 
   types::{CallbackQuery},
};

use text_io::scan;

#[derive(Copy, Clone)]
enum OrdersCommand {
    Add(i32, i32, i32), // rest_num, group_num, dish_num
    Remove(i32, i32, i32), // rest_num, group_num, dish_num
    UnknownCommand,
}

impl OrdersCommand {
   pub fn from(input: &str) -> OrdersCommand {
      // Попытаемся извлечь аргументы
      let rest_num: i32;
      let group_num: i32;
      let dish_num: i32;
      let r_part = input.get(3..).unwrap_or_default();
      scan!(r_part.bytes() => "{}:{}:{}", rest_num, group_num, dish_num);

      match input.get(..3).unwrap_or_default() {
         "add" => OrdersCommand::Add(rest_num, group_num, dish_num),
         "del" => OrdersCommand::Remove(rest_num, group_num, dish_num),
         _ => OrdersCommand::UnknownCommand,
     }
   }
}

pub async fn handle_message(cx: DispatcherHandlerCx<CallbackQuery>) {
   let query = cx.update;

   // Сообщение для отправки обратно
   let msg = match query.data {
      None => {
         String::from("Error handle_message None")
      }
      Some(data) => {
         // Идентифицируем и исполним команду
         match OrdersCommand::from(&data) {
            OrdersCommand::UnknownCommand => format!("Error handle_message {}", &data),
            OrdersCommand::Add(rest_num, group_num, dish_num) => format!("Добавить {}:{}:{}", rest_num, group_num, dish_num),
            OrdersCommand::Remove(rest_num, group_num, dish_num) => format!("Удалить {}:{}:{}", rest_num, group_num, dish_num),
         }
      }
   };

   // Отправляем ответ
   match cx.bot.answer_callback_query(query.id)
      .text(&msg)
      .send()
      .await {
      Err(_) => log::info!("Error handle_message {}", &msg),
      _ => (),
   }
}