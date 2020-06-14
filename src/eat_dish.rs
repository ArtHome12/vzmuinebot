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
   types::{InputFile, ReplyMarkup},
};

use crate::commands as cmd;
use crate::database as db;
use crate::eater;
use crate::eat_group;
use crate::eat_group_now;

// Основную информацию режима
//
pub async fn next_with_info(cx: cmd::Cx<(i32, i32, i32)>) -> cmd::Res {
   // Извлечём параметры
   let (cat_id, rest_id, group_id) = cx.dialogue;
   
   // Получаем информацию из БД
   let group_list = match db::dishes_by_restaurant_and_group_from_db(rest_id, group_id).await {
      Some(info) => info,
      None => format!("Ошибка db::dishes_by_restaurant_and_group_from_db({}, {})", rest_id, group_id)
   };

   // Отображаем информацию и кнопки меню
   cx.answer(group_list)
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

               // Попасть сюда могли двумя путями и это видно по коду категории
               if cat_id > 0 {
                  eat_group::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, (cat_id, rest_id))).await
               } else {
                  eat_group_now::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, rest_id)).await
               }
            }

            // Выбор блюда
            cmd::EaterDish::Dish(dish_num) => {
               // Получаем информацию из БД
               let (info, dish_image_id) = match db::eater_dish_info(rest_id, group_id, dish_num).await {
                  Some(dish_info) => dish_info,
                  None => (format!("Ошибка db::eater_dish_info({}, {}, {})", rest_id, group_id, dish_num), None)
               };

               // Идентифицируем пользователя
               let user_id = cx.update.from().unwrap().id;

               // Запросим из БД, сколько этих блюд пользователь уже выбрал
               let ordered_amount = 0;//db::amount_in_basket(rest_id, group_id, dish_num, user_id).await;

               // Создаём инлайн кнопки с указанием количества блюд
               let inline_keyboard = cmd::EaterDish::inline_markup(&db::make_dish_key(rest_id, group_id, dish_num), ordered_amount);

               // Отображаем информацию о блюде и оставляем кнопки главного меню. Если для блюда задана картинка, то текст будет комментарием
               if let Some(image_id) = dish_image_id {
                  // Создадим графический объект
                  let image = InputFile::file_id(image_id);

                  // Отправляем картинку и текст как комментарий
                  cx.answer_photo(image)
                  .caption(info)
                  .reply_markup(ReplyMarkup::InlineKeyboardMarkup(inline_keyboard))
                  .send()
                  .await?;
                  
                  next(cmd::Dialogue::EatRestGroupDishSelectionMode(cat_id, rest_id, group_id))
               } else {
                  cx.answer(info)
                  .reply_markup(inline_keyboard)
                  .send()
                  .await?;

                  next(cmd::Dialogue::EatRestGroupDishSelectionMode(cat_id, rest_id, group_id))
               }
            }

            cmd::EaterDish::UnknownCommand => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, (cat_id, rest_id, group_id)), "Вы в меню выбора блюда: неизвестная команда").await
            }
         }
      }
   }
}
