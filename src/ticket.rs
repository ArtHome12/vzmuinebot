/* ===============================================================================
Restaurant menu bot.
Ticket ro placed order. 06 June 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{
   prelude::*,
   types::{CallbackQuery, ParseMode, },
};
use regex::Regex;
use lazy_static::lazy_static;

use crate::database as db;
use crate::customer::*;
use crate::node;

type Update = UpdateWithCx<AutoSend<Bot>, CallbackQuery>;

async fn send_msg(receiver: i64, cx: &Update, text: &str) -> Result<(), String> {
   let mut ans = cx.requester
   .send_message(receiver, text)
   .parse_mode(ParseMode::Html);

   if let Some(reply_to_id) = &cx.update.message {
      ans = ans.reply_to_message_id(reply_to_id.id);
   }

   ans.await
   .map_err(|err| format!("ticket::send_msg for receiver={} {}", receiver, err))?;
   Ok(())
}

async fn forward_msg(receiver: i64, cx: &Update, message_id: i32) -> Result<(), String> {
   let from = &cx.update.message;
   if from.is_none() {
      Err(format!("forward_msg none cx.update.message for receiver={}", receiver))
   } else {
      let from = from.clone().unwrap().chat_id();
      cx.requester.forward_message(receiver, from, message_id).await
      .map_err(|err| format!("forward_msg {}", err))?;
      Ok(())
   }
}

async fn send_msg_to_owners(owners: &node::Owners, cx: &Update, text: &str) -> Result<(), String> {
   // Try to send to all owners
   let owner1 = send_msg(owners[0], cx, text).await;
   let owner2 = send_msg(owners[1], cx, text).await;
   let owner3 = send_msg(owners[2], cx, text).await;

   // Report an error from owner1 if there are no successful attempts
   owner3.or(owner2).or(owner1)
}

async fn forward_msg_to_owners(owners: &node::Owners, cx: &Update, message_id: i32) -> Result<(), String> {
   // Try to send to all owners
   let owner1 = forward_msg(owners[0], cx, message_id).await;
   let owner2 = forward_msg(owners[1], cx, message_id).await;
   let owner3 = forward_msg(owners[2], cx, message_id).await;

   // Report an error from owner1 if there are no successful attempts
   owner3.or(owner2).or(owner1)
}


pub async fn make_ticket(cx: &Update, node_id: i32) -> Result<&'static str, String> {

   // Load customer info
   let user_id = cx.update.from.id;
   let customer = db::user(user_id).await?;

   // Load owners node
   let node = db::node(db::LoadNode::EnabledIdNoChildren(node_id)).await?;
   let owners = if let Some(node) = node { node.owners } else { node::Owners::default() };

   // Check valid owner
   if owners[0] < 9999 && owners[1] < 9999 && owners[2] < 9999 {
      let text = "Заведение пока не подключено к боту, пожалуйста скопируйте ваш заказ отправьте по указанным контактным данным напрямую, после чего можно очистить корзину";
      send_msg(user_id, cx, text).await?;
      return Ok("Неудачно");
   }

   // Get source message text and id
   let old_text = cx.update.message.clone()
   .and_then(|f| f.text()
      .and_then(|f| Some(f.to_string()))
   );
   if old_text.is_none() {
      let text = "Не удаётся получить текст заказа, возможно слишком старое сообщение";
      send_msg(user_id, cx, text).await?;
      return Ok("Неудачно");
   }
   let old_text = old_text.unwrap();
   let orig_msg_id = cx.update.message.clone().unwrap().id; // unwrap checked above

   // Check delivery address if not pickup
   if matches!(customer.delivery, Delivery::Courier) {

      match customer.is_location() {
         true => {
            // Send to owner a message with the geographic location to make sure it's still available
            let message_id = customer.location_id().unwrap_or_default();
            let res = forward_msg_to_owners(&owners, cx, message_id).await;
            if let Err(err) = res {
               let text = format!("Недоступно сообщение с геопозицией, пожалуйста обновите адрес\n<i>{}</i>", err);
               send_msg(user_id, cx, &text).await?;
               return Ok("Неудачно");
            }
         }

         false => {
            if customer.address.len() < 1 {
               let text = "Пожалуйста, введите адрес или переключитесь на самовывоз при помощи кнопок внизу.\nЭта информация будет сохранена для последующих заказов, при необходимости вы всегда сможете её изменить";
               send_msg(user_id, cx, text).await?;
               return Ok("Неудачно");
            }
         }
      }
   }

   // Edit the original message - remove commands from text
   lazy_static! {
      static ref HASHTAG_REGEX : Regex = Regex::new(r" /del\d+").unwrap();
   }
   let text = HASHTAG_REGEX.replace_all(&old_text, "");

   cx.requester.edit_message_text(user_id, orig_msg_id, text)
   .await
   .map_err(|err| format!("make_ticket edit_message user_id={} {}", user_id, err))?;

   // Send to owner info about customer
   let text = format!("Заказ от {}:\nКонтакт для связи: {}\nСпособ доставки: {}",
      customer.name,
      customer.contact,
      customer.delivery_desc()
   );
   send_msg_to_owners(&owners, cx, &text).await?;

   // Forward edited message with order
   forward_msg_to_owners(&owners, cx, orig_msg_id).await?;

   // В БД переносим заказ из корзины в обработку
   // Отправляем сообщение едоку со статусом заказа
   // То же самое ресторатору
   // сохраним ссылки на сообщения со статусом для возможности их редактирования

   Ok("В разработке!")

/*
                     // Переместим заказ из корзины в обработку
                     if db::order_to_ticket(user_id, rest_id, message_id, new_message.id).await {

                        // Прочитаем только что записанный тикет из базы
                        let ticket = db::ticket(db::TicketBy::EaterAndCatererId(user_id, rest_id)).await;
                        if ticket.is_none() {
                           return false;
                        }
                        let ticket = ticket.unwrap();

                        // Отправим сообщение едоку, уже со статусом заказа
                        let eater_msg = send_message_for(cx.bot.clone(), from, InfoFor::Eater, &ticket).await;
                        if let Err(e) = eater_msg {
                           settings::log(&format!("Error send_basket({}, {}, {}), send_messages_for_eater: {}", user_id, rest_id, message_id, e)).await;
                           return false;
                        }

                        // И то же самое для ресторатора
                        let caterer_msg = send_message_for(cx.bot.clone(), to, InfoFor::Caterer, &ticket).await;
                        if let Err(e) = caterer_msg {
                           settings::log(&format!("Error send_basket({}, {}, {}), send_messages_for_caterer: {}", user_id, rest_id, message_id, e)).await;
                           return false;
                        }

                        // Все операции прошли успешно, сохраним ссылки на сообщения со статусом для возможности их редактирования
                        return db::ticket_save_status_msg(ticket.ticket_id, eater_msg.unwrap().id, caterer_msg.unwrap().id).await;
                     }
                  }
                  Err(err) =>  { settings::log(&format!("Error send_basket({}, {}, {}): {}", user_id, rest_id, message_id, err)).await;}
               }
            }
            Err(err) =>  { settings::log(&format!("Error send_basket announcement({}, {}, {}): {}", user_id, rest_id, message_id, err)).await;}
         }
      }
      None => {
         let s = format!("Error send_basket none info");
         settings::log(&s).await;
      }
   };

   
   // Раз попали сюда, значит что-то пошло не так
   false
 */
}