/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Режим едока, выбор ресторана, открытого сейчас. 10 June 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use chrono::{Utc};
use teloxide::{
   prelude::*, 
};

use crate::commands as cmd;
use crate::database as db;
use crate::eater;
use crate::eat_group_now;
use crate::basket;

// Показывает список ресторанов с группами заданной категории
//
pub async fn next_with_info(cx: cmd::Cx<()>) -> cmd::Res {
   // Текущее время
   let our_timezone = db::TIME_ZONE.get().unwrap();
   let now = Utc::now().with_timezone(our_timezone).naive_local().time();
   
   // Получаем информацию из БД
   let rest_list = db::restaurant_by_now_from_db(now).await;

   // Отображаем информацию и кнопки меню
   cx.answer(format!("Рестораны, открытые сейчас ({}):\n{}", now.format("%H:%M"), rest_list))
   .reply_markup(cmd::EaterRest::markup())
   .disable_notification(true)
   .send()
      .await?;

   // Переходим (остаёмся) в режим выбора ресторана
   next(cmd::Dialogue::EatRestNowSelectionMode)
}

// Показывает сообщение об ошибке/отмене без повторного вывода информации
async fn next_with_cancel(cx: cmd::Cx<()>, text: &str) -> cmd::Res {
   cx.answer(text)
   .reply_markup(cmd::EaterRest::markup())
   .disable_notification(true)
   .send()
   .await?;

   // Остаёмся в прежнем режиме.
   next(cmd::Dialogue::EatRestNowSelectionMode)
}



// Обработчик команд
//
pub async fn handle_selection_mode(cx: cmd::Cx<()>) -> cmd::Res {
   // Разбираем команду.
   match cx.update.text() {
      None => {
         next_with_cancel(cx, "Текстовое сообщение, пожалуйста!").await
      }
      Some(command) => {
         match cmd::EaterRest::from(command) {
            // В корзину
            cmd::EaterRest::Basket => {
               // Код едока
               let user_id = cx.update.from().unwrap().id;
               
               // Переходим в корзину
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               return basket::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, user_id)).await;
            }

            // В главное меню
            cmd::EaterRest::Main => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               eater::start(DialogueDispatcherHandlerCx::new(bot, update, ()), false).await
            }

            // Выбор ресторана
            cmd::EaterRest::Restaurant(rest_id) => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               eat_group_now::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, rest_id)).await
            }

            cmd::EaterRest::UnknownCommand => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, ()), "Вы в меню выбора ресторана: неизвестная команда").await
            }
         }
      }
   }
}
