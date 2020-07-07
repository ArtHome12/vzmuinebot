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
   types::{CallbackQuery, ChatOrInlineMessage, ChatId, InlineKeyboardButton,
      },
};

use crate::database as db;
use crate::commands as cmd;
use crate::eat_rest;
use crate::eat_rest_now;
use crate::eat_group;
use crate::eat_group_now;
use crate::eat_dish;
use crate::basket;

#[derive(Copy, Clone)]
enum CallbackCommand {
    Add(i32, i32, i32), // rest_num, group_num, dish_num
    Remove(i32, i32, i32), // rest_num, group_num, dish_num
    GroupsByRestaurantAndCategory(i32, i32), // rest_num, cat_id
    ReturnToCategory(i32), // cat_id
    Dishes(i32, i32, i32),  // rest_num, group_num, cat_id или 0 для автоопределения
    ReturnToGroups(i32, i32), // rest_num, cat_id
    Dish(i32, i32, i32),  // rest_num, group_num, dish_num
    ReturnToDishes(i32, i32, i32),  // rest_num, group_num, cat_id или 0 для автоопределения
    GroupsByRestaurantNow(i32), // rest_num
    ReturnToRestaurantsNow,
    SendBasket(i32), // rest_id
   //  BasketMessageToCaterer(i32), // rest_id
    BasketCancel(i32), // ticket_id
    BasketNext(i32), // ticket_id
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
               "rca" => CallbackCommand::ReturnToCategory(first),
               "drg" => CallbackCommand::Dishes(first, second, third),
               "rrg" => CallbackCommand::ReturnToGroups(first, second),
               "dis" => CallbackCommand::Dish(first, second, third),
               "rrd" => CallbackCommand::ReturnToDishes(first, second, third),
               "rng" => CallbackCommand::GroupsByRestaurantNow(first),
               "rno" => CallbackCommand::ReturnToRestaurantsNow,
               "bas" => CallbackCommand::SendBasket(first),
               // "bse" => CallbackCommand::BasketMessageToCaterer(first),
               "bca" => CallbackCommand::BasketCancel(first),
               "bne" => CallbackCommand::BasketNext(first),
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
            CallbackCommand::UnknownCommand => { db::log(&format!("UnknownCommand {}", &data)).await; format!("UnknownCommand {}", &data)}
            CallbackCommand::Add(rest_num, group_num, dish_num) => format!("Добавить {}: {}", db::make_key_3_int(rest_num, group_num, dish_num), db::is_success(add_dish(&cx, rest_num, group_num, dish_num, user_id).await)),
            CallbackCommand::Remove(rest_num, group_num, dish_num) => format!("Удалить {}: {}", db::make_key_3_int(rest_num, group_num, dish_num), db::is_success(remove_dish(&cx, rest_num, group_num, dish_num, user_id).await)),
            CallbackCommand::GroupsByRestaurantAndCategory(rest_num, cat_id) => 
               format!("Группы '{}' {}", db::id_to_category(cat_id), db::is_success(eat_group::show_inline_interface(&cx, rest_num, cat_id).await)),
            CallbackCommand::ReturnToCategory(cat_id) => 
               format!("Возврат к '{}' {}", db::id_to_category(cat_id), db::is_success(eat_rest::show_inline_interface(&cx, cat_id).await)),
            CallbackCommand::Dishes(rest_num, group_num, cat_id) => 
               format!("Блюда {}:{} {}", rest_num, group_num, db::is_success(eat_dish::show_inline_interface(&cx, rest_num, group_num, cat_id).await)),
            CallbackCommand::ReturnToGroups(rest_num, cat_id) => 
               format!("Группы '{}' {}", db::id_to_category(cat_id), db::is_success(eat_group::show_inline_interface(&cx, rest_num, cat_id).await)),
            CallbackCommand::Dish(rest_num, group_num, dish_num) =>
               format!("Блюдо '{}': {}", db::make_key_3_int(rest_num, group_num, dish_num), db::is_success(eat_dish::show_dish(&cx, rest_num, group_num, dish_num).await)),
            CallbackCommand::ReturnToDishes(rest_num, group_num, cat_id) =>
               format!("Блюда {}:{} {}", rest_num, group_num, db::is_success(eat_dish::show_inline_interface(&cx, rest_num, group_num, cat_id).await)),
            CallbackCommand::GroupsByRestaurantNow(rest_num) => 
               format!("Работающие: {}", db::is_success(eat_group_now::show_inline_interface(&cx, rest_num).await)),
            CallbackCommand::ReturnToRestaurantsNow => 
               format!("Работающие: {}", db::is_success(eat_rest_now::show_inline_interface(&cx).await)),
            CallbackCommand::SendBasket(rest_id) => {
               let res = match query.message.clone() {
                  Some(message) => basket::send_basket(&cx, rest_id, user_id, message.id).await,
                  None => false,
               };
               format!("Отправка: {}", db::is_success(res))
            }
            // CallbackCommand::BasketMessageToCaterer(rest_id) => format!("{}", db::is_success(basket::prepare_to_send_message(user_id, rest_id).await)),
            CallbackCommand::BasketCancel(ticket_id) => format!("{}", db::is_success(cancel_ticket(&cx, user_id, ticket_id).await)),
            CallbackCommand::BasketNext(ticket_id) => format!("{}", db::is_success(process_ticket(&cx, ticket_id).await)),
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
async fn update_keyboard(cx: &DispatcherHandlerCx<CallbackQuery>, rest_num: i32, group_num: i32, dish_num: i32, new_amount: i32) -> bool {
   let message = cx.update.message.as_ref().unwrap();

   let button_back = InlineKeyboardButton::callback(String::from("Назад"), format!("rrd{}", db::make_key_3_int(rest_num, group_num, 0)));
   let inline_keyboard = cmd::EaterDish::inline_markup(&db::make_key_3_int(rest_num, group_num, dish_num), new_amount)
   .append_to_row(button_back, 0);

   let chat_message = ChatOrInlineMessage::Chat {
      chat_id: ChatId::Id(message.chat_id()),
      message_id: message.id,
   };
   match cx.bot.edit_message_reply_markup(chat_message)
      .reply_markup(inline_keyboard)
      .send()
      .await {
         Err(_) => {
            let text = format!("Error edit_message_reply_markup {}:{}:{}", rest_num, group_num, dish_num);
            db::log(&text).await;
            false
         }
         _ => true,
   }
}

// Отправить сообщение с цитированием
pub async fn message_with_quote(cx: &DispatcherHandlerCx<CallbackQuery>, chat_id: ChatId, s: &str, reply_id: i32) {
   if let Err(e) = cx.bot.send_message(chat_id, s).reply_to_message_id(reply_id).send().await {
      db::log(&format!("Error callback::message_with_quote: {}", e)).await;
   }
}

// Отредактировать сообщение
// pub async fn edit_message(chat_id: ChatId, s: &str) -> bool {
//    // Используем специально выделенный экземпляр бота
//    if let Some(bot) = db::BOT.get() {
//       if let Err(e) = bot.edit_message_text(chat_id, s).send().await {
//          db::log(&format!("Ошибка edit_message {}", e)).await;
//          false
//       } else {true}
//    } else {false}
// }

// Удаляет инлайн-кнопки под сообщением
/* async fn remove_inline_markup(cx: &DispatcherHandlerCx<CallbackQuery>, chat_id: i32, message_id: i32) {

   // Код чата и сообщения
   let chat_message = ChatOrInlineMessage::Chat {
      chat_id: ChatId::Id(i64::from(chat_id)),
      message_id,
   };

   // Выполняем операцию, при ошибке - текст в служебный чат
   if let Err(e) = cx.bot.edit_message(chat_message).send().await {
      let text = format!("Error callback::remove_inline_markup({}, {}): {}", chat_id, message_id, e);
      db::log(&text).await;
   }
}
 */

// Отменяет заказ, как со стороны ресторатора, так и едока
async fn cancel_ticket(cx: &DispatcherHandlerCx<CallbackQuery>, user_id: i32, ticket_id: i32) -> bool {
   // Если операция с БД успешна, надо отредактировать сообщения и дополнительно уведомить другую сторону
   if db::basket_edit_stage(ticket_id, 6).await {
      
      // Информация о тикете
      if let Some(t) = db::ticket_with_owners(ticket_id).await {

         // Удаляем инлайн кнопки под заказом в чате едока и ресторатора - не работает
         // remove_inline_markup(cx, t.eater_id, t.ticket.eater_msg_id).await;
         // remove_inline_markup(cx, t.caterer_id, t.ticket.caterer_msg_id).await;

         // Адрес другой стороны это адрес, не совпадающий с нашим собственным
         let (other_chat_id, other_msg_id, this_chat_id, this_msg_id) = if user_id == t.caterer_id {
            (t.eater_id, t.ticket.eater_msg_id, t.caterer_id, t.ticket.caterer_msg_id)
         } else {
            (t.caterer_id, t.ticket.caterer_msg_id, t.eater_id, t.ticket.eater_msg_id)
         };

         // Отправим сообщение другой стороне
         let s = String::from("Заказ был отменён другой стороной, для получения актуального списка заказов обновите корзину");
         let s = format!("{} (other_chat_id={}, other_msg_id={})", s, other_chat_id, other_msg_id);
         message_with_quote(cx, ChatId::Id(i64::from(other_chat_id)), &s, other_msg_id).await;

         // Сообщение в своём чате
         let s = String::from("Вы отменили заказ, обновите корзину");
         let s = format!("{} (this_chat_id={}, this_msg_id={})", s, this_chat_id, this_msg_id);
         let this_chat = ChatId::Id(i64::from(this_chat_id));
         message_with_quote(cx, this_chat.clone(), &s, this_msg_id).await;

         // Два сообщения в служебный чат - об отмене и сам отменённый заказ
         db::log(&format!("Заказ отменён по инициативе {}", user_id)).await;
         db::log_forward(this_chat,this_msg_id).await;

         true
      } else {false}
   } else {false}
}

// Переводит заказ на следующую стадицю
async fn process_ticket(cx: &DispatcherHandlerCx<CallbackQuery>, ticket_id: i32) -> bool {
   // Продолжаем только если операция с БД успешна
   if db::basket_next_stage(ticket_id).await {
      
      // Информация о тикете
      if let Some(t) = db::ticket_with_owners(ticket_id).await {

         // Адрес другой стороны это адрес, не совпадающий с нашим собственным
         // let to = if user_id == t.caterer_id {t.eater_id} else {t.caterer_id};

         // // Сообщение для правки в собственном чате
         // let message = cx.update.message.as_ref().unwrap();
         // let chat_id = ChatId::Id(message.chat_id());

         // // Отправим сообщение другой стороне
         // let s = String::from("Заказ был отменён другой стороной, для получения актуального списка заказов повторно зайдите в корзину");
         // if reply_message(ChatId::Id(i64::from(to)), &s, t.ticket.message_id).await {
            
         //    let chat_message = ChatOrInlineMessage::Chat {
         //       chat_id: chat_id.clone(),
         //       message_id: message.id,
         //    };

         //    // Исправим сообщение в своём чате
         //    let s = String::from("Вы отменили заказ");
         //    if let Err(e) = cx.bot.edit_message_text(chat_message, s).send().await {
         //       let text = format!("Error cancel_ticket: {}", e);
         //       db::log(&text).await;
         //       false
         //    } else {
         //       // Сообщение об отмене в служебный чат
         //       db::log(&format!("Заказ отменён по инициативе {}", user_id)).await;
         //       db::log_forward(chat_id, t.ticket.message_id).await;

         //       true
         //    }
         // } else {
         //    let s = String::from("Не удалось уведомить другую сторону об отмене заказа");
         //    if let Err(e) = cx.bot.send_message(chat_id, s).send().await {
         //       let text = format!("Error cancel_ticket2: {}", e);
         //       db::log(&text).await;
         //    }
         //    false
         // }
         false
      } else {false}
   } else {false}
}

