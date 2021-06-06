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
   types::{CallbackQuery, },
};

use crate::database as db;
use crate::customer::*;

async fn msg(cx: &UpdateWithCx<AutoSend<Bot>, CallbackQuery>, text: &str) -> Result<(), String> {
   let user_id = cx.update.from.id;
   let mut ans = cx.requester.send_message(user_id, text);

   if let Some(reply_to_id) = &cx.update.message {
      ans = ans.reply_to_message_id(reply_to_id.id);
   }

   ans.await
   .map_err(|err| format!("ticket::msg {}", err))?;
   Ok(())
}

pub async fn make_ticket(cx: &UpdateWithCx<AutoSend<Bot>, CallbackQuery>, node_id: i32) -> Result<&'static str, String> {

   // Load owner node
   let node = db::node(db::LoadNode::EnabledIdNoChildren(node_id)).await?;
   let owner = if let Some(node) = node { node.owners[0] } else { 0 };

   // Check valid owner
   if owner < 9999 {
      let text = "Заведение пока не подключено к боту, пожалуйста скопируйте ваш заказ отправьте по указанным контактным данным напрямую, после чего можно очистить корзину";
      msg(cx, text).await?;
      return Ok("Неудачно");
   }

   // Load customer info
   let user_id = cx.update.from.id;
   let customer = db::user(user_id).await?;

   // Check delivery address if not pickup
   if matches!(customer.delivery, Delivery::Courier) {

      match customer.is_location() {
         true => {
            // Send the customer a message with the geographic location to make sure it's still available
            /* let message_id = customer.location_id().unwrap_or_default();
            let from = cx.update.message.unwrap().chat_id(); // from bot
            let res = cx.requester.forward_message(user_id, from, message_id).await;
            match res {
               Ok(_) => String::from("прежняя геопозиция в сообщении выше"),
               Err(_) => String::from("сохранённая геопозиция больше недоступна"),
            } */
         }

         false => {
            if customer.address.len() < 1 {
               let text = "Пожалуйста, введите адрес или переключитесь на самовывоз при помощи кнопок внизу.\nЭта информация будет сохранена для последующих заказов, при необходимости вы всегда сможете её изменить";
               msg(cx, text).await?;
               return Ok("Неудачно");
            }
         }
      }
   }

   Ok("В разработке!")

/*    // Откуда и куда
   let from = ChatId::Id(i64::from(user_id));
   let to = ChatId::Id(i64::from(rest_id));

   // Сообщение с геолокацией, если есть
   let location_message = basket_info.address_message_id();

      // Если не самовывоз, то проверим контактную информацию
   if !basket_info.pickup {
      // Если адрес слишком короткий, выходим с сообщением
      if basket_info.address.len() < 3 {
         let msg = String::from("Пожалуйста, введите адрес, нажав /edit_address или переключитесь на самовывоз, нажав /toggle\nЭта информация будет сохранена для последующих заказов, при необходимости вы всегда сможете её изменить");
         let res = cx.bot.send_message(from.clone(), msg).send().await;
         if let Err(e) = res {
            let msg = format!("basket::send_basket 3(): {}", e);
            settings::log(&msg).await;
         }
         return false;
      } 

      // Если задано местоположение на карте, надо проверить, что сообщение с геолокацией ещё доступно
      if basket_info.is_geolocation() {
         // Заготовим текст сообщения с ошибкой заранее
         let err_message = String::from("Недоступно сообщение с геопозицией, пожалуйста укажите адрес ещё раз, нажав /edit_address");

         // Код сообщения
         if let Some(msg_id) = location_message {
            // Отправим сообщение самому едоку для контроля и проверки, что нет ошибки
            let res = cx.bot.forward_message(to.clone(), from.clone(), msg_id).send().await;
            if let Err(e) = res {
               let err_message = format!("{}\n<i>{}</i>", err_message, e);
               let res = cx.bot.send_message(from.clone(), err_message)
               .parse_mode(ParseMode::HTML)
               .send().await;
               if let Err(e) = res {
                  let msg = format!("basket::send_basket 4(): {}", e);
                  settings::log(&msg).await;
               }
               return false;
            }
         } else {
            // Этот код никогда не должен выполниться
            let res = cx.bot.send_message(from.clone(), err_message).send().await;
            if let Err(e) = res {
               let msg = format!("basket::send_basket 5(): {}", e);
               settings::log(&msg).await;
            }
            return false;
         }
      }
   }

   // Начнём с запроса информации о ресторане-получателе
   match db::restaurant(db::RestBy::Id(rest_id)).await {
      Some(rest) => {

         // Заново сгенерируем текст исходного сообщения уже без команд /del в тексте, чтобы пересылать его
         let basket_with_no_commands = db::basket_content(user_id, rest.num, rest_id, &rest.title, &rest.info, true).await;

         // Ссылка на исправляемое сообщение
         let original_message = ChatOrInlineMessage::Chat {
            chat_id: from.clone(),
            message_id,
         };

         // Исправим исходное сообщение на новый текст, чтобы исчезли команды и кнопка "оформить"
         if let Err(e) = cx.bot.edit_message_text(original_message, make_basket_message_text(&basket_with_no_commands)).send().await {
            let s = format!("Error send_basket edit_message_text(): {}", e);
            settings::log(&s).await;
         }
         
         // Информация о едоке
         let method = if basket_info.pickup {String::from("Cамовывоз")} else {format!("Курьером по адресу {}", basket_info.address_label())};
         let eater_info = format!("Заказ от {}\nКонтакт: {}\n{}", basket_info.name, basket_info.contact, method);

         // Отправим сообщение с контактными данными (геолокация уже отправлена выше)
         settings::log_and_notify(&eater_info).await;
         match cx.bot.send_message(to.clone(), eater_info).send().await {
            Ok(_) => {
               // Пересылаем сообщение с заказом
               settings::log_forward(from.clone(), message_id).await;
               match cx.bot.forward_message(to.clone(), from.clone(), message_id).send().await {
                  Ok(new_message) => {

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