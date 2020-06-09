/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Режим едока, выбор блюда после выбора группы и ресторана. 09 June 2020.
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
use crate::eat_group;

// Основную информацию режима
//
pub async fn next_with_info(cx: cmd::Cx<(i32, i32, i32)>) -> cmd::Res {
   // Извлечём параметры
   let (cat_id, rest_id, group_id) = cx.dialogue;
   
   // Получаем информацию из БД
   let group_list = db::dishes_by_restaurant_and_group_from_db(rest_id, group_id).await;

   // Отображаем информацию и кнопки меню
   cx.answer(format!("Подходящие разделы меню:\n{}", group_list))
   .reply_markup(cmd::EaterDish::markup())
       .send()
       .await?;

   // Переходим (остаёмся) в режим выбора ресторана
   next(cmd::Dialogue::EatRestGroupDishSelectionMode(cat_id, rest_id, group_id))
}

// Показывает сообщение об ошибке/отмене без повторного вывода информации
async fn next_with_cancel(cx: cmd::Cx<(i32, i32, i32)>, text: &str) -> cmd::Res {
   cx.answer(text)
   .reply_markup(cmd::EaterDish::markup())
   .send()
   .await?;

   // Извлечём параметры
   let (cat_id, rest_id, group_id) = cx.dialogue;

   // Остаёмся в прежнем режиме.
   next(cmd::Dialogue::EatRestGroupDishSelectionMode(cat_id, rest_id, group_id))
}



// Обработчик команд
//
pub async fn handle_selection_mode(cx: cmd::Cx<(i32, i32, i32)>) -> cmd::Res {
   // Извлечём параметры
   let (cat_id, rest_id, group_id) = cx.dialogue;

   // Разбираем команду.
   match cx.update.text() {
      None => {
         next_with_cancel(cx, "Текстовое сообщение, пожалуйста!").await
      }
      Some(command) => {
         match cmd::EaterDish::from(command) {

            // В главное меню
            cmd::EaterDish::Main => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               eater::start(DialogueDispatcherHandlerCx::new(bot, update, ()), false).await
            }

            // В предыдущее меню
            cmd::EaterDish::Return => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               return eat_group::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, (cat_id, rest_id))).await;
            }

            // Выбор блюда
            cmd::EaterDish::Dish(group_id) => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, (cat_id, rest_id, group_id)), "Команда в разработке").await
            }

            cmd::EaterDish::UnknownCommand => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, (cat_id, rest_id, group_id)), "Вы в меню выбора ресторана: неизвестная команда").await
            }
         }
      }
   }
}
