/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Режим едока, просмотр корзины. 15 June 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{
   prelude::*,
   types::{ChatId, InlineKeyboardMarkup, CallbackQuery, ChatOrInlineMessage,},
};

use crate::commands as cmd;
use crate::database as db;
use crate::eater;

// Показывает список закзов для user_id
//
pub async fn next_with_info(cx: cmd::Cx<i32>) -> cmd::Res {
   // Извлечём параметры
   let user_id = cx.dialogue;
   
   // Получаем информацию из БД
   let (baskets, grand_total) = db::basket_contents(user_id).await;

   // Информация о едоке
   let basket_info = db::user_basket_info(user_id).await;
   let eater_info = if let Some(info) = basket_info {
      let method = if info.pickup {String::from("самовывоз")} else {String::from("курьером по адресу")};
      format!("Ваши контактные данные (для редактирования жмите на ссылки рядом): {} /edit_name\nКонтакт: {} /edit_contact\nАдрес: {} /edit_address\nМетод доставки: {} /toggle", info.name, info.contact, info.address_label(), method)
   } else {
      String::from("Информации о пользователе нет")
   };

   if baskets.is_empty() {
      // Отображаем информацию и кнопки меню
      cx.answer(format!("{}\n\nКорзина пуста", eater_info))
      .reply_markup(cmd::Basket::bottom_markup())
      .disable_notification(true)
      .send()
      .await?;
   } else {
      // Отображаем приветствие
      let s = format!("{}\n\nОбщая сумма заказа {}. Вы можете самостоятельно скопировать сообщения с заказом и переслать напрямую в заведение или в независимую доставку, а потом очистить корзину. Либо воспользоваться кнопками под заказом (перепроверьте ваши контактные данные)", eater_info, db::price_with_unit(grand_total));
      cx.answer(s)
      .reply_markup(cmd::Basket::bottom_markup())
      .disable_notification(true)
      .send()
      .await?;

      // Отдельными сообщениями выводим рестораны
      for basket in baskets {

         // Текст сообщения о корзине
         let s = make_basket_message_text(&basket);

         // Отправляем сообщение
         cx.answer(s)
         .reply_markup(cmd::Basket::inline_markup_send(basket.rest_id))
         .disable_notification(true)
         .send()
         .await?;
      }
   }

   // Теперь выводим собственные заказы в обработке другой стороной
   send_messages_for_eater(&cx, user_id).await;

   // Теперь выводим заказы, отправленные едоками нам, если мы вдруг ресторан
   let ticket_info = db::caterer_ticket_info(user_id).await;

   for ticket_item in ticket_info {
      // Извлечём данные
      let (eater_id, ticket) = ticket_item;
      let message_id = ticket.caterer_msg_id;

      // Отправляем стадию выполнения с цитированием заказа
      let (text, markup) = make_message_for_caterer(eater_id, ticket).await;
      let res = cx.answer(text)
      .reply_to_message_id(message_id)
      .reply_markup(markup)
      .send()
      .await;
      
      if let Err(e) = res {
         db::log(&format!("Error next_with_info send ticket2(): {}", e)).await
      }
   }
   
   // Переходим (остаёмся) в режим выбора ресторана
   next(cmd::Dialogue::BasketMode(user_id))
}

// Отправляет сообщение с информацией о заказе, ожидающем обработки другой стороной
async fn send_message_for_eater(cx: &cmd::Cx<i32>, caterer_id: i32, ticket: db::Ticket) {
   // Извлечём данные
   let message_id = ticket.eater_msg_id;

   // Отправляем стадию выполнения с цитированием заказа
   let (text, markup) = make_message_for_eater(caterer_id, ticket).await;
   let res = cx.answer(text)
   .reply_to_message_id(message_id)
   .reply_markup(markup)
   .send()
   .await;

   if let Err(e) = res {
      db::log(&format!("Error send_message_for_eater: {}", e)).await
   }
}

async fn send_messages_for_eater(cx: &cmd::Cx<i32>, eater_id: i32) {
   let ticket_info = db::eater_ticket_info(eater_id).await;
   for ticket_item in ticket_info {
      let (caterer_id, ticket) = ticket_item;
      send_message_for_eater(&cx, caterer_id, ticket).await;
   }
}

async fn send_message_for_eater2(chat_id: ChatId, caterer_id: i32, ticket: db::Ticket) {
   // Используем специально выделенный экземпляр бота
   if let Some(bot) = db::BOT.get() {

      // Извлечём данные
      let message_id = ticket.eater_msg_id;

      // Отправляем стадию выполнения с цитированием заказа
      let (text, markup) = make_message_for_eater(caterer_id, ticket).await;
      let res = bot.send_message(chat_id, text)
      .reply_to_message_id(message_id)
      .reply_markup(markup)
      .send()
      .await;

      if let Err(e) = res {
         db::log(&format!("Error send_message_for_eater2: {}", e)).await
      }
   } else {
      db::log(&String::from("Error send_message_for_eater2: no bot")).await
   }
}

async fn send_messages_for_eater2(chat_id: ChatId, eater_id: i32) {
   let ticket_info = db::eater_ticket_info(eater_id).await;
   for ticket_item in ticket_info {
      let (caterer_id, ticket) = ticket_item;
      send_message_for_eater2(chat_id.clone(), caterer_id, ticket).await;
   }
}


// Формирует сообщение с собственной корзиной
pub fn make_basket_message_text(basket: &db::Basket) -> String {
   // Заголовок с информацией о ресторане
   let mut s = basket.restaurant.clone();

   // Дополняем данными о блюдах
   for dish in basket.dishes.clone() {
      s.push_str(&format!("\n{}", dish))
   }

   // Итоговая стоимость
   s.push_str(&format!("\nВсего: {}", db::price_with_unit(basket.total)));
   s
}

// Формирует сообщение с заказом для показа едоку
pub async fn make_message_for_eater(caterer_id: i32, ticket: db::Ticket) -> (String, InlineKeyboardMarkup) {
   // Текст сообщения со стадией выполнения 
   let rest_name = db::restaurant_title_by_id(caterer_id).await;
   let stage = db::stage_to_str(ticket.stage);
   let s = format!("{}. Для отправки сообщения к '{}', например, с уточнением времени, нажмите на ссылку /snd{}", stage, rest_name, caterer_id);

   // Если заказ на последней стадии, то добавляем кнопку завершить кроме кнопки отмены
   if ticket.stage == 4 {
      (s, cmd::Basket::inline_markup_message_confirm(ticket.ticket_id))
   } else {
      (s, cmd::Basket::inline_markup_message_cancel(ticket.ticket_id))
   }
}

// Формирует сообщение с заказом для показа ресторатору
pub async fn make_message_for_caterer(eater_id: i32, ticket: db::Ticket) -> (String, InlineKeyboardMarkup) {
   // Текст сообщения
   let eater_name = db::user_name_by_id(eater_id).await;
   let stage1 = db::stage_to_str(ticket.stage);
   let stage2 = db::stage_to_str(ticket.stage + 1);
   let s = format!("Заказ вам от {} в '{}'. Для отправки заказчику сообщения, например, с уточнением времени, нажмите на ссылку /snd{}\nДля изменения статуса на '{}' нажмите кнопку 'Далее'", eater_name, stage1, eater_id, stage2);

   // Возвращаем сообщение со стадией выполнения и цитированием заказа
   (s, cmd::Basket::inline_markup_message_next(ticket.ticket_id))
}

// Показывает сообщение об ошибке/отмене без повторного вывода информации
async fn next_with_cancel(cx: cmd::Cx<i32>, text: &str) -> cmd::Res {
   cx.answer(text)
   .reply_markup(cmd::Basket::bottom_markup())
   .disable_notification(true)
   .send()
   .await?;

   // Извлечём параметры
   let user_id = cx.dialogue;

   // Остаёмся в прежнем режиме.
   next(cmd::Dialogue::BasketMode(user_id))
}



// Обработчик команд
//
pub async fn handle_commands(cx: cmd::Cx<i32>) -> cmd::Res {
   // Извлечём параметры
   let user_id = cx.dialogue;

   // Разбираем команду.
   match cx.update.text() {
      None => {
         next_with_cancel(cx, "Текстовое сообщение, пожалуйста!").await
      }
      Some(command) => {
         match cmd::Basket::from(command) {

            // В главное меню
            cmd::Basket::Main => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               eater::start(DialogueDispatcherHandlerCx::new(bot, update, ()), false).await
            }

            // Обновить
            cmd::Basket::Refresh => {
               next_with_info(cx).await
            }

            // Неизвестная команда
            cmd::Basket::UnknownCommand => {
               // Возможно это общая команда
               match cmd::Common::from(command) {
                  cmd::Common::Start => {
                     let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                     eater::start(DialogueDispatcherHandlerCx::new(bot, update, ()), false).await
                  }
                  cmd::Common::SendMessage(caterer_id) => {
                     // Отправляем приглашение ввести строку со слешем в меню для отмены
                     cx.answer(format!("Введите сообщение (/ для отмены)"))
                     .reply_markup(cmd::Caterer::slash_markup())
                     .disable_notification(true)
                     .send()
                     .await?;
      
                     // Переходим в режим ввода
                     next(cmd::Dialogue::MessageToCaterer(user_id, caterer_id, Box::new(cmd::Dialogue::BasketMode(user_id)), Box::new(cmd::Basket::bottom_markup())))
                  }
                  cmd::Common::UnknownCommand => {
                     let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                     next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, user_id), "Вы в меню корзина: неизвестная команда").await
                  }
               }
            }

            // Очистить корзину
            cmd::Basket::Clear => {
               if db::clear_basket(user_id).await {
                  // Сообщение в лог
                  let text = format!("{} корзина очищена", db::user_info(cx.update.from(), false));
                  db::log(&text).await;

                  // Отображаем пустую корзину
                  let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                  next_with_info(DialogueDispatcherHandlerCx::new(bot, update, user_id)).await
               } else {
                  let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                  next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, user_id), "Вы в меню корзина: ошибка очистки корзины").await
               }
            }

            // Удалить одну позицию
            cmd::Basket::Delete(rest_num, group_num, dish_num) => {
               // Запрос к базе данных
               match db::remove_dish_from_basket(rest_num, group_num, dish_num, user_id).await {
                  Ok(_) => {
                     // Сообщение в лог
                     let text = format!("{} корзина {} удалено", db::user_info(cx.update.from(), false), db::make_key_3_int(rest_num, group_num, dish_num));
                     db::log(&text).await;

                     // Отображаем изменённую корзину
                     let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                     next_with_info(DialogueDispatcherHandlerCx::new(bot, update, user_id)).await
                  }
                  _ => {
                     let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                     next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, user_id), "Вы в меню корзина: ошибка удаления блюда").await
                  }
               }
            }

            // Редактировать имя
            cmd::Basket::EditName => {
               // Отправляем приглашение ввести строку со слешем в меню для отмены
               cx.answer(format!("Отправьте ваше имя (/ для отмены)"))
               .reply_markup(cmd::Caterer::slash_markup())
               .disable_notification(true)
               .send()
               .await?;

               // Переходим в режим ввода
               next(cmd::Dialogue::BasketEditName(user_id))
            }

            // Редактировать контакт
            cmd::Basket::EditContact => {
               // Отправляем приглашение ввести строку со слешем в меню для отмены
               cx.answer(format!("Если хотите дать возможность ресторатору связаться с вами напрямую, укажите ник или телефон (/ для отмены)"))
               .reply_markup(cmd::Caterer::slash_markup())
               .disable_notification(true)
               .send()
               .await?;

               // Переходим в режим ввода
               next(cmd::Dialogue::BasketEditContact(user_id))
            }

            // Редактировать адрес
            cmd::Basket::EditAddress => {
               // Отправляем приглашение ввести строку со слешем в меню для отмены
               cx.answer(format!("Введите адрес для доставки или укажите точку на карте (/ для отмены). Также вы можете отправить произвольную точку или даже транслировать её изменение, для этого нажмите скрепку 📎 и выберите геопозицию."))
               .reply_markup(cmd::Basket::address_markup())
               .disable_notification(true)
               .send()
               .await?;

               // Переходим в режим ввода
               next(cmd::Dialogue::BasketEditAddress(user_id))
            }

            // Переключить способ доставки
            cmd::Basket::TogglePickup => {
               db::basket_toggle_pickup(user_id).await;
               next_with_info(cx).await
            }
         }
      }
   }
}

// Изменить имя едока
pub async fn edit_name_mode(cx: cmd::Cx<i32>) -> cmd::Res {
   // Извлечём параметры
   let user_id = cx.dialogue;
        
   if let Some(text) = cx.update.text() {
      // Удалим из строки слеши
      let s = cmd::remove_slash(text).await;

      // Если строка не пустая, продолжим
      if !s.is_empty() {
         // Сохраним новое значение в БД
         if db::basket_edit_name(user_id, s).await {
            // Покажем изменённую информацию
            next_with_info(cx).await
         } else {
            // Сообщим об ошибке
            next_with_cancel(cx, &format!("Ошибка edit_name_mode({})", user_id)).await
         }
      } else {
         // Сообщим об отмене
         next_with_cancel(cx, "Отмена ввода имени").await
      }
   } else {
      next(cmd::Dialogue::BasketMode(user_id))
   }
}

// Изменить контакт едока
pub async fn edit_contact_mode(cx: cmd::Cx<i32>) -> cmd::Res {
   // Извлечём параметры
   let user_id = cx.dialogue;
        
   if let Some(text) = cx.update.text() {
      // Удалим из строки слеши
      let s = cmd::remove_slash(text).await;

      // Если строка не пустая, продолжим
      if !s.is_empty() {
         // Сохраним новое значение в БД
         if db::basket_edit_contact(user_id, s).await {
            // Покажем изменённую информацию
            next_with_info(cx).await
         } else {
            // Сообщим об ошибке
            next_with_cancel(cx, &format!("Ошибка edit_contact_mode({})", user_id)).await
         }
      } else {
         // Сообщим об отмене
         next_with_cancel(cx, "Отмена ввода контакта").await
      }
   } else {
      next(cmd::Dialogue::BasketMode(user_id))
   }
}

// Изменить адрес едока
pub async fn edit_address_mode(cx: cmd::Cx<i32>) -> cmd::Res {
   // Извлечём параметры
   let user_id = cx.dialogue;
        
   // Ожидаем либо текстовое сообщение, либо локацию
   let option_text = cx.update.text();
   let option_location = cx.update.location();
   let message_id = cx.update.id;

   // Проверяем на текстовое сообщение
   if let Some(text) = option_text {
      // Удалим из строки слеши
      let s = cmd::remove_slash(text).await;

      // Если строка не пустая, продолжим
      if !s.is_empty() {
         // Сохраним новое значение в БД
         if db::basket_edit_address(user_id, s).await {
            // Покажем изменённую информацию
            next_with_info(cx).await
         } else {
            // Сообщим об ошибке
            next_with_cancel(cx, &format!("Ошибка edit_address_mode({})", user_id)).await
         }
      } else {
         // Сообщим об отмене
         next_with_cancel(cx, "Отмена ввода адреса").await
      }
   } else {
      // Проверяем на геометку
      if let Some(_location) = option_location {
         // Сохраним код сообщения
         if db::basket_edit_address(user_id, format!("Location{}", message_id)).await {
            // Покажем изменённую информацию
            next_with_info(cx).await
         } else {
            // Сообщим об ошибке
            next_with_cancel(cx, &format!("Ошибка basket_edit_address2({})", user_id)).await
         }
      } else {
         // Сообщим об отмене
         next_with_cancel(cx, "Отмена, ожидался либо текст либо геометка").await
      } 
   }
}

// Отправляет сообщение ресторатору с корзиной пользователя
pub async fn send_basket(cx: &DispatcherHandlerCx<CallbackQuery>, rest_id: i32, user_id: i32, message_id: i32) -> bool {
   // Откуда и куда
   let from = ChatId::Id(i64::from(user_id));
   let to = ChatId::Id(i64::from(rest_id));

   // Заново сгенерируем текст исходного сообщения уже без команд /del в тексте, чтобы пересылать его
   let (rest_num, rest_title, rest_info) = db::restaurant_num_and_title_by_id(rest_id).await;
   let basket_with_no_commands = db::basket_content(user_id, rest_num, rest_id, &rest_title, &rest_info, true).await;

   // Ссылка на исправляемое сообщение
   let original_message = ChatOrInlineMessage::Chat {
      chat_id: from.clone(),
      message_id,
   };

   // Исправим исходное сообщение на новый текст
   if let Err(e) = cx.bot.edit_message_text(original_message, make_basket_message_text(&basket_with_no_commands)).send().await {
      let s = format!("Error send_basket edit_message_text(): {}", e);
      db::log(&s).await;
   }
   
   // Информация о едоке
   let basket_info = db::user_basket_info(user_id).await;
   let (eater_info, location_message_id) = if let Some(info) = basket_info {
      let method = if info.pickup {String::from("Cамовывоз")} else {format!("Курьером по адресу {}", info.address_label())};
      (format!("Заказ от {}\nКонтакт: {}\n{}", info.name, info.contact, method), info.address_message_id())
   } else {
      (String::from("Информации о пользователе нет"), None)
   };

   // Отправим сообщение с контактными данными
   db::log_and_notify(&eater_info).await;
   match cx.bot.send_message(to.clone(), eater_info).send().await {
      Ok(_) => {
         // Перешлём сообщение с геолокацией, если она задана
         if let Some(location_message) = location_message_id {

            db::log_forward(from.clone(), location_message).await;
            if let Err(e) = cx.bot.forward_message(to.clone(), from.clone(), location_message).send().await {
               db::log(&format!("Error send_basket forward location({}, {}, {}): {}", user_id, rest_id, message_id, e)).await;
            }
         }

         // Перешлём сообщение с заказом, при этом надо сохранить его идентификатор в чате назначения
         db::log_forward(from.clone(), message_id).await;
         match cx.bot.forward_message(to, from.clone(), message_id).send().await {
            Ok(new_message) => {

               // Переместим заказ из корзины в обработку
               if db::order_to_ticket(user_id, rest_id, message_id, new_message.id).await {
                  // Отправим новое сообщение, уже со статусом заказа
                  send_messages_for_eater2(from, user_id).await;

                  // Все операции прошли успешно
                  return true;
               }
            }
            Err(err) =>  { db::log(&format!("Error send_basket({}, {}, {}): {}", user_id, rest_id, message_id, err)).await;}
         }
      }
      Err(err) =>  { db::log(&format!("Error send_basket announcement({}, {}, {}): {}", user_id, rest_id, message_id, err)).await;}
   }
   
   // Раз попали сюда, значит что-то пошло не так
   false
}

// Отправляет сообщение ресторатору с корзиной пользователя
// pub async fn prepare_to_send_message(user_id: i32, rest_id: i32) -> bool {
//    // Используем специально выделенный экземпляр бота
//    if let Some(bot) = db::BOT.get() {
//       // Приглашающее сообщение
//       let s = format!("Для ввода сообщения к '{}' нажмите на ссылку /snd{}", db::restaurant_title_by_id(rest_id).await, rest_id);
//       let to = ChatId::Id(i64::from(user_id));
//       match bot.send_message(to, s).send().await {
//          Ok(_) => {true}
//          Err(err) =>  {
//             db::log(&format!("Error prepare_to_send_message({}, {}): {}", user_id, rest_id, err)).await;
//             false
//          }
//       }
//    } else {false}
// }
