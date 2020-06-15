/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Режим едока, выбор группы после выбора ресторана. 09 June 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{
   prelude::*, 
   types::{InputFile, ReplyMarkup},
};

use crate::commands as cmd;
use crate::database as db;
use crate::eater;
use crate::eat_rest;
use crate::eat_dish;
use crate::basket;

// Основную информацию режима
//
pub async fn next_with_info(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
   // Извлечём параметры
   let (cat_id, rest_id) = cx.dialogue;
   
   // Получаем информацию из БД
   let (info, rest_image_id) = match db::groups_by_restaurant_and_category(rest_id, cat_id).await {
      Some(dish_info) => dish_info,
      None => (format!("Ошибка db::groups_by_restaurant_and_category({}, {})", rest_id, cat_id), None)
   };

    // Отображаем информацию о блюде и оставляем кнопки главного меню. Если для блюда задана картинка, то текст будет комментарием
    if let Some(image_id) = rest_image_id {
      // Создадим графический объект
      let image = InputFile::file_id(image_id);

      // Отправляем картинку и текст как комментарий
      cx.answer_photo(image)
      .caption(info)
      .reply_markup(ReplyMarkup::ReplyKeyboardMarkup(cmd::EaterGroup::markup()))
      .send()
      .await?;
   } else {
         cx.answer(info)
         .reply_markup(cmd::EaterGroup::markup())
         .send()
         .await?;
   }

   // Переходим (остаёмся) в режим выбора группы
   next(cmd::Dialogue::EatRestGroupSelectionMode(cat_id, rest_id))
}

// Показывает сообщение об ошибке/отмене без повторного вывода информации
async fn next_with_cancel(cx: cmd::Cx<(i32, i32)>, text: &str) -> cmd::Res {
   cx.answer(text)
   .reply_markup(cmd::EaterGroup::markup())
   .send()
   .await?;

   // Извлечём параметры
   let (cat_id, rest_id) = cx.dialogue;

   // Остаёмся в прежнем режиме.
   next(cmd::Dialogue::EatRestGroupSelectionMode(cat_id, rest_id))
}



// Обработчик команд
//
pub async fn handle_selection_mode(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
   // Извлечём параметры
   let (cat_id, rest_id) = cx.dialogue;

   // Разбираем команду.
   match cx.update.text() {
      None => {
         next_with_cancel(cx, "Текстовое сообщение, пожалуйста!").await
      }
      Some(command) => {
         match cmd::EaterGroup::from(command) {
            // В корзину
            cmd::EaterGroup::Basket => {
               // Код едока
               let user_id = cx.update.from().unwrap().id;
               
               // Переходим в корзину
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               return basket::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, user_id)).await;
            }

            // В главное меню
            cmd::EaterGroup::Main => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               eater::start(DialogueDispatcherHandlerCx::new(bot, update, ()), false).await
            }

            // В предыдущее меню
            cmd::EaterGroup::Return => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               eat_rest::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, cat_id)).await
            }

            // Выбор группы
            cmd::EaterGroup::Group(group_id) => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               eat_dish::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, (cat_id, rest_id, group_id))).await
            }

            cmd::EaterGroup::UnknownCommand => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, (cat_id, rest_id)), "Вы в меню выбора группы: неизвестная команда").await
            }
         }
      }
   }
}
