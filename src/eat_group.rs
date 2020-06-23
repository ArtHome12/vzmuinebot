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
use crate::language as lang;

// Основная информацию режима
//
pub async fn next_with_info(cx: cmd::Cx<(bool, i32, i32)>) -> cmd::Res {
   // Извлечём параметры
   let (compact_mode, cat_id, rest_id) = cx.dialogue;
   
   // Получаем информацию из БД
   match db::groups_by_restaurant_and_category(rest_id, cat_id).await {
      None => {
         // Такая ситуация может возникнуть, если ресторатор удалил группу только что
         let s = String::from(lang::t("ru", lang::Res::EatGroupsEmpty));
         let s = format!("Подходящие группы:\n{}", s);
         let new_cx = DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ());
         cmd::send_text(&new_cx, &s, cmd::EaterRest::markup()).await;
      }
      Some(info) => {

         // Выводим информацию либо ссылками, либо инлайн кнопками
         if compact_mode {
            // Сформируем строку вида "название /ссылка\n"
            let s: String = if info.groups.is_empty() {
               String::from(lang::t("ru", lang::Res::EatRestEmpty))
            } else {
               info.groups.into_iter().map(|(key, value)| (format!("   {} /grou{}\n", value, key))).collect()
            };
            
            // Добавляем к информации о ресторане информацию о группах
            let s = format!("{}\n{}", info.info, s);

            // Отображаем информацию о группах ресторана. Если для ресторана задана картинка, то текст будет комментарием
            if let Some(image_id) = info.image_id {
               // Создадим графический объект
               let image = InputFile::file_id(image_id);

               // Отправляем картинку и текст как комментарий
               cx.answer_photo(image)
               .caption(s)
               .reply_markup(ReplyMarkup::ReplyKeyboardMarkup(cmd::EaterGroup::markup()))
               .disable_notification(true)
               .send()
               .await?;
            } else {
                  cx.answer(s)
                  .reply_markup(cmd::EaterGroup::markup())
                  .disable_notification(true)
                  .send()
                  .await?;
            }
            

         } else {
            show_inline_interface(DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), info, cat_id).await;

            // В инлайн-режиме всегда остаёмся в главном меню
            return next(cmd::Dialogue::UserMode(compact_mode));
         }
      }
   };

   // Переходим (остаёмся) в режим выбора группы
   next(cmd::Dialogue::EatRestGroupSelectionMode(compact_mode, cat_id, rest_id))
}

// Показывает сообщение об ошибке/отмене без повторного вывода информации
async fn next_with_cancel(cx: cmd::Cx<(bool, i32, i32)>, text: &str) -> cmd::Res {
   cx.answer(text)
   .reply_markup(cmd::EaterGroup::markup())
   .disable_notification(true)
   .send()
   .await?;

   // Извлечём параметры
   let (compact_mode, cat_id, rest_id) = cx.dialogue;

   // Остаёмся в прежнем режиме.
   next(cmd::Dialogue::EatRestGroupSelectionMode(compact_mode, cat_id, rest_id))
}



// Обработчик команд
//
pub async fn handle_selection_mode(cx: cmd::Cx<(bool, i32, i32)>) -> cmd::Res {
   // Извлечём параметры
   let (compact_mode, cat_id, rest_id) = cx.dialogue;

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
               eat_rest::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, (compact_mode, cat_id))).await
            }

            // Выбор группы
            cmd::EaterGroup::Group(group_id) => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               eat_dish::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, (compact_mode, cat_id, rest_id, group_id))).await
            }

            cmd::EaterGroup::UnknownCommand => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, (compact_mode, cat_id, rest_id)), "Вы в меню выбора группы: неизвестная команда").await
            }
         }
      }
   }
}

// Выводит инлайн кнопки
//
pub async fn show_inline_interface(_cx: cmd::Cx<()>, _rest_list: db::GroupListWithRestaurantInfo, _cat_id: i32) {
   // Создадим кнопки под рестораны
/*    let buttons: Vec<InlineKeyboardButton> = rest_list.into_iter()
   .map(|(rest_num, title)| (InlineKeyboardButton::callback(title, format!("grc{}", db::make_key_3_int(rest_num, cat_id, 0)))))  // third argument always 0
   .collect();

   // Формируем меню
   let markup = InlineKeyboardMarkup::default()
   .append_row(buttons);

   let s = String::from("Рестораны с подходящим меню:");
   cmd::send_text(&cx, &s, markup).await;
 */}
