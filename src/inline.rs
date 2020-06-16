/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Инлайн-запросы к боту, то есть с упоминанием его имени. 16 June 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{
   prelude::*, 
   types::{InlineQuery},
};

// use crate::database as db;

#[derive(Copy, Clone)]
enum InlineCommand {
    UnknownCommand,
}

impl InlineCommand {
   pub fn from(_input: &str) -> InlineCommand {
      InlineCommand::UnknownCommand
   }
}

pub async fn handle_message(cx: DispatcherHandlerCx<InlineQuery>) {
   let query = &cx.update.query;

   // Распознаем полученную команду
   match InlineCommand::from(&query) {
      InlineCommand::UnknownCommand => log::error!("Unknown inline message: {}", &query)
   }


/*   let query_id = &query.id;

   // Сообщение для отправки обратно
   let msg = match &query.data {
      None => {
         String::from("Error handle_message None")
      }
      Some(data) => {
         // Код едока
         let user_id = query.from.id;

         // Идентифицируем и исполним команду
         match InlineCommand::from(&data) {
            InlineCommand::UnknownCommand => format!("Error handle_message {}", &data),
            InlineCommand::Add(rest_num, group_num, dish_num) => format!("Добавить {} {}", db::make_dish_key(rest_num, group_num, dish_num), db::is_success(add_dish(&cx, rest_num, group_num, dish_num, user_id).await)),
            InlineCommand::Remove(rest_num, group_num, dish_num) => format!("Удалить {} {}", db::make_dish_key(rest_num, group_num, dish_num), db::is_success(remove_dish(&cx, rest_num, group_num, dish_num, user_id).await)),
         }
      }
   }
 */
}
