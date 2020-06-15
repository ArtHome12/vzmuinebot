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
   types::{CallbackQuery, ChatOrInlineMessage, ChatId},
};

use text_io::scan;
use crate::database as db;
use crate::commands as cmd;

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
   let query = &cx.update;
   let query_id = &query.id;

   // Сообщение для отправки обратно
   let msg = match &query.data {
      None => {
         String::from("Error handle_message None")
      }
      Some(data) => {
         // Код едока
         let user_id = query.from.id;

         // Идентифицируем и исполним команду
         match OrdersCommand::from(&data) {
            OrdersCommand::UnknownCommand => format!("Error handle_message {}", &data),
            OrdersCommand::Add(rest_num, group_num, dish_num) => format!("Добавить {}:{}:{} {}", rest_num, group_num, dish_num, db::is_success(add_dish(&cx, rest_num, group_num, dish_num, user_id).await)),
            OrdersCommand::Remove(rest_num, group_num, dish_num) => format!("Удалить {}:{}:{} {}", rest_num, group_num, dish_num, db::is_success(remove_dish(&cx, rest_num, group_num, dish_num, user_id).await)),
         }
      }
   };

   // Обновляем исходное сообщение

   // Отправляем ответ
   match cx.bot.answer_callback_query(query_id)
      .text(&msg)
      .send()
      .await {
         Err(_) => log::info!("Error handle_message {}", &msg),
         _ => (),
   }
}

// Добавляет блюдо в корзину
//
async fn add_dish(cx: &DispatcherHandlerCx<CallbackQuery>, rest_num: i32, group_num: i32, dish_num: i32, user_id: i32) -> bool {
   // Если операция с БД успешна, надо отредактировать пост
   match db::add_dish_to_basket(rest_num, group_num, dish_num, user_id).await {
      Ok(new_amount) => update_keyboard(cx, rest_num, group_num, dish_num, new_amount).await,
      Err(_) => false,
   }
}


// Удаляет блюдо из корзины
//
async fn remove_dish(cx: &DispatcherHandlerCx<CallbackQuery>, rest_num: i32, group_num: i32, dish_num: i32, user_id: i32) -> bool {
   // Если операция с БД успешна, надо отредактировать пост
   match db::remove_dish_from_basket(rest_num, group_num, dish_num, user_id).await {
      Ok(new_amount) => update_keyboard(cx, rest_num, group_num, dish_num, new_amount).await,
      Err(_) => false,
   }
}


// Обновляет инлайн-клавиатуру для правки количества
//
async fn update_keyboard(cx: &DispatcherHandlerCx<CallbackQuery>, rest_num: i32, group_num: i32, dish_num: i32, new_amount: i32) -> bool {
   let message = cx.update.message.as_ref().unwrap();
   let inline_keyboard = cmd::EaterDish::inline_markup(&db::make_dish_key(rest_num, group_num, dish_num), new_amount);
   let chat_message = ChatOrInlineMessage::Chat {
      chat_id: ChatId::Id(message.chat_id()),
      message_id: message.id,
   };
   match cx.bot.edit_message_reply_markup(chat_message)
      .reply_markup(inline_keyboard)
      .send()
      .await {
         Err(_) => {
            log::info!("Error edit_message_reply_markup {}:{}:{}", rest_num, group_num, dish_num);
            false
         }
         _ => true,
   }
}