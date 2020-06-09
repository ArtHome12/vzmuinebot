/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Режим едока, выбор ресторана при известной группе. 09 June 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{
   prelude::*, 
};

use crate::commands as cmd;
use crate::database as db;
use crate::eater;

// Показывает список ресторанов с группами заданной категории
//
pub async fn next_with_info(cx: cmd::Cx<i32>) -> cmd::Res {
   // Извлечём параметры
   let cat_id = cx.dialogue;
   
   // Получаем информацию из БД
   let rest_list = db::restaurant_by_category_from_db(cat_id).await;

   // Отображаем информацию и кнопки меню
   cx.answer(format!("Рестораны с подходящим меню:\n{}", rest_list))
   .reply_markup(cmd::EaterRest::markup())
       .send()
       .await?;

   // Переходим (остаёмся) в режим выбора ресторана
   next(cmd::Dialogue::EatRestSelectionMode(cat_id))
}

// Показывает сообщение об ошибке/отмене без повторного вывода информации
async fn next_with_cancel(cx: cmd::Cx<i32>, text: &str) -> cmd::Res {
   cx.answer(text)
   .reply_markup(cmd::EaterRest::markup())
   .send()
   .await?;

   // Код категории
   let cat_id = cx.dialogue;

   // Остаёмся в прежнем режиме.
   next(cmd::Dialogue::EatRestSelectionMode(cat_id))
}



// Обработчик команд
//
pub async fn rest_selection_mode(cx: cmd::Cx<i32>) -> cmd::Res {
   // Код категории
   let cat_id = cx.dialogue;

   // Разбираем команду.
   match cx.update.text() {
      None => {
         next_with_cancel(cx, "Текстовое сообщение, пожалуйста!").await
      }
      Some(command) => {
         match cmd::EaterRest::from(command) {

            // В главное меню
            cmd::EaterRest::Main => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               eater::start(DialogueDispatcherHandlerCx::new(bot, update, ()), false).await
            }

            // Выбор ресторана
            cmd::EaterRest::Restaurant(rest_id) => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, cat_id), "Команда в разработке").await
            }

            cmd::EaterRest::UnknownCommand => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, cat_id), "Вы в меню выбора ресторана: неизвестная команда").await
            }
         }
      }
   }
}
