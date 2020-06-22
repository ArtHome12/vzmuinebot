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
   types::{InlineKeyboardButton, InlineKeyboardMarkup},
};
use std::collections::HashMap;

use crate::commands as cmd;
use crate::database as db;
use crate::eater;
use crate::eat_group;
use crate::basket;
use crate::language as lang;


// Показывает список ресторанов с группами заданной категории
//
pub async fn next_with_info(cx: cmd::Cx<(bool, i32)>) -> cmd::Res {
   // Извлечём параметры
   let (compact_mode, cat_id) = cx.dialogue;
   let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
   let new_cx = DialogueDispatcherHandlerCx::new(bot, update, compact_mode);
   
   // Получаем информацию из БД
   let rest_list: HashMap<i32, String> = db::restaurant_by_category_from_db(cat_id).await;

   // Если там пусто, то сообщим об этом
   if rest_list.is_empty() {
      let s = String::from(lang::t("ru", lang::Res::EatRestEmpty));
      let s = format!("Рестораны с подходящим меню:\n{}", s);
      cmd::send_text(&new_cx, &s, cmd::User::main_menu_markup()).await;

   } else {

      // Выводим информацию либо ссылками, либо инлайн кнопками
      if compact_mode {
         // Сформируем строку вида "название /ссылка\n"
         let s: String = if rest_list.is_empty() {
            String::from(lang::t("ru", lang::Res::EatRestEmpty))
         } else {
            rest_list.into_iter().map(|(rest_num, title)| (format!("   {} /rest{}\n", title, rest_num))).collect()
         };
         
         // Отображаем информацию и кнопки меню
         let s = format!("Рестораны с подходящим меню:\n{}", s);
         cmd::send_text(&new_cx, &s, cmd::User::main_menu_markup()).await;
   
      } else {
         // Создадим кнопки под рестораны
         let buttons: Vec<InlineKeyboardButton> = rest_list.into_iter()
         .map(|(rest_num, title)| (InlineKeyboardButton::callback(title, format!("grc{}", db::make_key_3_int(rest_num, cat_id, 0)))))  // third argument always 0
         .collect();

         // Формируем меню
         let markup = InlineKeyboardMarkup::default()
         .append_row(buttons);

         let s = String::from("Рестораны с подходящим меню:");
         cmd::send_text(&new_cx, &s, markup).await;
      }
   }

   // Переходим (остаёмся) в режим выбора ресторана
   next(cmd::Dialogue::EatRestSelectionMode(compact_mode, cat_id))
}

// Показывает сообщение об ошибке/отмене без повторного вывода информации
async fn next_with_cancel(cx: cmd::Cx<(bool, i32)>, text: &str) -> cmd::Res {
   cx.answer(text)
   .reply_markup(cmd::EaterRest::markup())
   .disable_notification(true)
   .send()
   .await?;

   // Код категории
   let (compact_mode, cat_id) = cx.dialogue;

   // Остаёмся в прежнем режиме.
   next(cmd::Dialogue::EatRestSelectionMode(compact_mode, cat_id))
}


// Обработчик команд
//
pub async fn handle_selection_mode(cx: cmd::Cx<(bool, i32)>) -> cmd::Res {
   // Код категории
   let (compact_mode, cat_id) = cx.dialogue;

   // Разбираем команду.
   match cx.update.text() {
      None => {
         next_with_cancel(cx, "Текстовое сообщение, пожалуйста!").await
      }
      Some(command) => {
         match cmd::EaterRest::from(compact_mode, command) {
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
            cmd::EaterRest::Restaurant(compact_mode, rest_id) => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               eat_group::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, (compact_mode, cat_id, rest_id))).await
            }

            cmd::EaterRest::UnknownCommand => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, (compact_mode, cat_id)), "Вы в меню выбора ресторана: неизвестная команда").await
            }
         }
      }
   }
}
