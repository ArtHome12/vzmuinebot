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

#[derive(Copy, Clone)]
enum OrdersCommand {
    Basket,
    Add(i32, i32, i32), // rest_num, group_num, dish_num
    Remove(i32, i32, i32), // rest_num, group_num, dish_num
    UnknownCommand,
}

impl OrdersCommand {
   pub fn from(input: &str) -> OrdersCommand {
      match input {
         // Сначала проверим на цельные команды.
         "bas" => OrdersCommand::Basket,
         _ => {
             // Ищем среди команд с цифровыми суффиксами - аргументами
             match input.get(..4).unwrap_or_default() {
               "add" => OrdersCommand::Add(1, 1, 1),
               "del" => OrdersCommand::Remove(1, 1, 1),
               _ => OrdersCommand::UnknownCommand,
             }
         }
     }
   }
}

pub async fn handle_message(cx: DispatcherHandlerCx<CallbackQuery>) {
   let query = cx.update;

   // Сообщение для отправки обратно
   let msg = match query.data {
      None => {
         "Error handle_message None"
      }
      Some(data) => {
         // Идентифицируем и исполним команду
         match OrdersCommand::from(&data) {
            OrdersCommand::Basket => "В корзину",
            OrdersCommand::UnknownCommand => "Error handle_message Some",
            OrdersCommand::Add(_rest_num, _group_num, _dish_num) => "Добавить",
            OrdersCommand::Remove(_rest_num, _group_num, _dish_num) => "Удалить",
         }
      }
   };

   // Отправляем ответ
   match cx.bot.answer_callback_query(query.id)
      .text(msg)
      .send()
      .await {
      Err(_) => log::info!("Error handle_message {}", msg),
      _ => (),
   }
}