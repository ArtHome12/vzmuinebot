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
   let user = cx.update.from();
   let basket_info = db::user_basket_info(user).await;
   let eater_info = if let Some(info) = basket_info {
      let method = if info.pickup {String::from("самовывоз")} else {String::from("курьером по адресу")};
      format!("Ваши контактные данные (для редактирования жмите на ссылки рядом): {} /edit_name\nКонтакт: {} /edit_contact\nАдрес: {} /edit_address\nМетод доставки: {} /toggle", info.name, info.contact, info.address, method)
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
      let s = format!("{}\n\nОбщая сумма заказа {}. Вы можете самостоятельно скопировать сообщения с заказом и переслать напрямую в заведение или в независимую доставку, а потом очистить корзину. Либо нажать на кнопку под заказом (перепроверьте ваши контактные данные)", eater_info, db::price_with_unit(grand_total));
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

         // Для колбека по id ресторатора узнаем его имя
         let caption = String::from("Отправить");
         let data = format!("bas{}", db::make_key_3_int(basket.rest_id, 0, 0));

         cx.answer(s)
         .reply_markup(cmd::Basket::inline_markup(caption, data))
         .disable_notification(true)
         .send()
         .await?;
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
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, user_id), "Вы в меню корзина: неизвестная команда").await
            }

            // Редактировать контакт
            cmd::Basket::EditContact => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, user_id), "Вы в меню корзина: неизвестная команда").await
            }

            // Редактировать адрес
            cmd::Basket::EditAddress => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, user_id), "Вы в меню корзина: неизвестная команда").await
            }

            // Переключить способ доставки
            cmd::Basket::TogglePickup => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, user_id), "Вы в меню корзина: неизвестная команда").await
            }
         }
      }
   }
}

// Отправляет сообщение ресторатору с корзиной пользователя
pub async fn send_basket(rest_id: i32, user_id: i32, message_id: i32) -> bool {
   if let Some(chat) = db::TELEGRAM_LOG_CHAT.get() {
      // Откуда и куда
      let from = ChatId::Id(i64::from(user_id));
      let to = ChatId::Id(i64::from(rest_id));

      match chat.bot.forward_message(to, from, message_id).send().await {
         Ok(_) => {return true; }
         Err(err) =>  { db::log(&format!("Error send_basket({}, {}, {}): {}", user_id, rest_id, message_id, err)).await;}
      }
   }
   
   // Раз попали сюда, значит что-то пошло не так
   false
}