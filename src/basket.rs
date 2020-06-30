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
   types::{ChatId},
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
         // Заголовок с информацией о ресторане
         let mut s = basket.restaurant;

         // Дополняем данными о блюдах
         for dish in basket.dishes {
            s.push_str(&format!("\n{}", dish))
         }

         // Итоговая стоимость
         s.push_str(&format!("\nВсего: {}", db::price_with_unit(basket.total)));

         cx.answer(s)
         .reply_markup(cmd::Basket::inline_markup_send(basket.rest_id))
         .disable_notification(true)
         .send()
         .await?;
      }

      // Теперь выводим заказы в обработке
      if let Some(bot) = db::BOT.get() {
         // Данные в базе
         let ticket_info = db::ticket_info(user_id).await;

         // Чат назначения - собственный
         let to = ChatId::Id(i64::from(user_id));

         for ticket_item in ticket_info {
            // Извлечём данные
            let (caterer_id, ticket) = ticket_item;

            // Текст сообщения
            let rest_name = db::restaurant_title_by_id(caterer_id).await;
            let stage = db::stage_to_str(ticket.stage);
            let s = format!("{}. Для отправки сообщения к '{}', например, с уточнением времени, нажмите на ссылку /snd{}", stage, rest_name, caterer_id);
            // Отправляем стадию выполнения с цитированием заказа
            let res = bot.send_message(to.clone(), s)
            .reply_to_message_id(ticket.message_id)
            .reply_markup(cmd::Basket::inline_markup_message_cancel(caterer_id))
            .send()
            .await;

            if let Err(e) = res {
               db::log(&format!("Error next_with_info send ticket(): {}", e)).await
            }
         }
      }
   }

   // Переходим (остаёмся) в режим выбора ресторана
   next(cmd::Dialogue::BasketMode(user_id))
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
pub async fn handle_selection_mode(cx: cmd::Cx<i32>) -> cmd::Res {
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

            // Неизвестная команда
            cmd::Basket::UnknownCommand => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, user_id), "Вы в меню корзина: неизвестная команда").await
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

            // Редактировать сообщение для отправки ресторатору
            cmd::Basket::Send(rest_id) => {
               // Отправляем приглашение ввести строку со слешем в меню для отмены
               cx.answer(format!("Введите сообщение (/ для отмены)"))
               .reply_markup(cmd::Caterer::slash_markup())
               .disable_notification(true)
               .send()
               .await?;

               // Переходим в режим ввода
               next(cmd::Dialogue::BasketMessageToCaterer(user_id, rest_id))
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
         next_with_cancel(cx, "Отмена").await
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
         next_with_cancel(cx, "Отмена").await
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
         next_with_cancel(cx, "Отмена").await
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
pub async fn send_basket(rest_id: i32, user_id: i32, message_id: i32) -> bool {
   // Используем специально выделенный экземпляр бота
   if let Some(bot) = db::BOT.get() {
      // Откуда и куда
      let from = ChatId::Id(i64::from(user_id));
      let to = ChatId::Id(i64::from(rest_id));

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
      match bot.send_message(to.clone(), eater_info).send().await {
         Ok(_) => {
            // Перешлём сообщение с геолокацией, если она задана
            if let Some(location_message) = location_message_id {

               db::log_forward(from.clone(), location_message).await;
               if let Err(e) = bot.forward_message(to.clone(), from.clone(), location_message).send().await {
                  db::log(&format!("Error send_basket forward location({}, {}, {}): {}", user_id, rest_id, message_id, e)).await;
               }
            }

            // Перешлём сообщение с заказом
            db::log_forward(from.clone(), message_id).await;
            match bot.forward_message(to, from, message_id).send().await {
               Ok(_) => {
                  // Переместим заказ из корзины в обработку
                  return db::order_to_ticket(user_id, rest_id, message_id).await;
               }
               Err(err) =>  { db::log(&format!("Error send_basket({}, {}, {}): {}", user_id, rest_id, message_id, err)).await;}
            }
         }
         Err(err) =>  { db::log(&format!("Error send_basket announcement({}, {}, {}): {}", user_id, rest_id, message_id, err)).await;}
      }
   }
   
   // Раз попали сюда, значит что-то пошло не так
   false
}



// Отправить сообщение ресторатору
pub async fn edit_message_to_caterer_mode(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
   // Извлечём параметры
   let (user_id, caterer_id) = cx.dialogue;

   // Используем специально выделенный экземпляр бота
   if let Some(bot) = db::BOT.get() {
      let to = ChatId::Id(i64::from(caterer_id));
         
      if let Some(text) = cx.update.text() {
         // Удалим из строки слеши
         let s = cmd::remove_slash(text).await;

         // Если строка не пустая, продолжим
         if !s.is_empty() {
            // Текст для отправки
            let user_name = if let Some(u) = cx.update.from() {&u.first_name} else {""};
            let s = format!("Сообщение от {}\n{}\n Ответить /snd{}", user_name, s, user_id);

            // Отправляем сообщение и сообщаем результат
            let res = if let Err(e) = bot.send_message(to, s).send().await {
               format!("Ошибка {}", e)
            } else {String::from("Сообщение отправлено")};

            let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
            next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, user_id), &res).await
         } else {
            // Сообщим об отмене
            let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
            next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, user_id), "Отмена").await
         }
      } else {next(cmd::Dialogue::BasketMode(user_id))}
   } else {next(cmd::Dialogue::BasketMode(user_id))}
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
