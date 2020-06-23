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

use crate::database as db;
use crate::commands as cmd;
use crate::eat_group;

#[derive(Copy, Clone)]
enum CallbackCommand {
    Add(i32, i32, i32), // rest_num, group_num, dish_num
    Remove(i32, i32, i32), // rest_num, group_num, dish_num
    GroupsByRestaurantAndCategory(i32, i32), // rest_num, cat_id
    UnknownCommand,
}

impl CallbackCommand {
   pub fn from(input: &str) -> CallbackCommand {
      // Попытаемся извлечь аргументы
      let r_part = input.get(3..).unwrap_or_default();
      match db::parse_key_3_int(r_part) {
         Ok((first, second, third)) => {
            match input.get(..3).unwrap_or_default() {
               "add" => CallbackCommand::Add(first, second, third),
               "del" => CallbackCommand::Remove(first, second, third),
               "grc" => CallbackCommand::GroupsByRestaurantAndCategory(first, second),
               _ => CallbackCommand::UnknownCommand,
            }
         }
         _ => CallbackCommand::UnknownCommand,
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
         match CallbackCommand::from(&data) {
            CallbackCommand::UnknownCommand => format!("Error handle_message {}", &data),
            CallbackCommand::Add(rest_num, group_num, dish_num) => format!("Добавить {}: {}", db::make_key_3_int(rest_num, group_num, dish_num), db::is_success(add_dish(&cx, rest_num, group_num, dish_num, user_id).await)),
            CallbackCommand::Remove(rest_num, group_num, dish_num) => format!("Удалить {}: {}", db::make_key_3_int(rest_num, group_num, dish_num), db::is_success(remove_dish(&cx, rest_num, group_num, dish_num, user_id).await)),
            CallbackCommand::GroupsByRestaurantAndCategory(rest_num, cat_id) => 
               format!("Группы '{}' {}", db::id_to_category(cat_id), db::is_success(eat_group::show_inline_interface(&cx, rest_num, cat_id).await)),
         }
      }
   };

   // Отправляем ответ, который показывается во всплывающем окошке
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
      Ok(new_amount) => {
         // Сообщение в лог
         let text = format!("{} блюдо {} +1", db::user_info(Some(&cx.update.from), false), db::make_key_3_int(rest_num, group_num, dish_num));
         db::log(&text).await;

         // Изменяем инлайн кнопки
         update_keyboard(cx, rest_num, group_num, dish_num, new_amount).await
      }
      Err(_) => false,
   }
}


// Удаляет блюдо из корзины
//
async fn remove_dish(cx: &DispatcherHandlerCx<CallbackQuery>, rest_num: i32, group_num: i32, dish_num: i32, user_id: i32) -> bool {
   // Если операция с БД успешна, надо отредактировать пост
   match db::remove_dish_from_basket(rest_num, group_num, dish_num, user_id).await {
      Ok(new_amount) => {
         // Сообщение в лог
         let text = format!("{} блюдо {} -1", db::user_info(Some(&cx.update.from), false), db::make_key_3_int(rest_num, group_num, dish_num));
         db::log(&text).await;

         // Изменяем инлайн кнопки
         update_keyboard(cx, rest_num, group_num, dish_num, new_amount).await
      }
      Err(_) => false,
   }
}


// Обновляет инлайн-клавиатуру для правки количества
//
async fn update_keyboard(cx: &DispatcherHandlerCx<CallbackQuery>, rest_num: i32, group_num: i32, dish_num: i32, new_amount: i32) -> bool {
   let message = cx.update.message.as_ref().unwrap();
   let inline_keyboard = cmd::EaterDish::inline_markup(&db::make_key_3_int(rest_num, group_num, dish_num), new_amount);
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

