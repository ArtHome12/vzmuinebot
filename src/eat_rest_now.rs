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
   types::{InlineKeyboardButton, InlineKeyboardMarkup, ReplyMarkup,
      CallbackQuery, ChatOrInlineMessage, ChatId, InputMedia, InputFile
   },
};
use arraylib::iter::IteratorExt;

use crate::commands as cmd;
use crate::database as db;
use crate::eater;
use crate::eat_group_now;
use crate::basket;
use crate::language as lang;

// Показывает список ресторанов с группами заданной категории
//
pub async fn next_with_info(cx: cmd::Cx<bool>) -> cmd::Res {
   // Режим меню
   let compact_mode = cx.dialogue;

   // Текущее время
   let our_timezone = db::TIME_ZONE.get().unwrap();
   let now = Utc::now().with_timezone(our_timezone).naive_local().time();
   
   // Получаем информацию из БД
   let rest_list = db::restaurant_by_now(now).await;

   // Если там пусто, то сообщим об этом
   if rest_list.is_empty() {
      let s = String::from(lang::t("ru", lang::Res::EatRestNowEmpty));
      let s = format!("Рестораны, открытые сейчас ({}):\n{}", now.format("%H:%M"), s);
      let new_cx = DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ());
      cmd::send_text(&new_cx, &s, cmd::EaterRest::markup()).await;

   } else {

      // Выводим информацию либо ссылками, либо инлайн кнопками
      if compact_mode {
         // Сформируем строку вида "название /ссылка\n"
         let s: String = rest_list.into_iter().map(|(rest_num, title)| (format!("   {} /rest{}\n", title, rest_num))).collect();
         
         // Отображаем информацию и кнопки меню
         let s = format!("Рестораны, открытые сейчас ({}):\n{}", now.format("%H:%M"), s);
         let new_cx = DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ());
         cmd::send_text(&new_cx, &s, cmd::EaterRest::markup()).await;
   
      } else {
         // Создадим кнопки
         let markup = make_markup(rest_list);

         // Отправляем сообщение с плашкой в качестве картинки
         let s = String::from(format!("Рестораны, открытые сейчас ({}):", now.format("%H:%M")));
         let new_cx = DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ());
         cmd::send_photo(&new_cx, &s, ReplyMarkup::InlineKeyboardMarkup(markup), db::default_photo_id()).await;

         // В инлайн-режиме всегда остаёмся в главном меню
         return next(cmd::Dialogue::UserMode(compact_mode));
      }
   }

   // Переходим (остаёмся) в режим выбора ресторана
   next(cmd::Dialogue::EatRestNowSelectionMode(compact_mode))
}

// Показывает сообщение об ошибке/отмене без повторного вывода информации
async fn next_with_cancel(cx: cmd::Cx<bool>, text: &str) -> cmd::Res {
   // Режим меню
   let compact_mode = cx.dialogue;

   cx.answer(text)
   .reply_markup(cmd::EaterRest::markup())
   .disable_notification(true)
   .send()
   .await?;

   // Остаёмся в прежнем режиме.
   next(cmd::Dialogue::EatRestNowSelectionMode(compact_mode))
}



// Обработчик команд
pub async fn handle_commands(cx: cmd::Cx<bool>) -> cmd::Res {
   // Режим меню
   let compact_mode = cx.dialogue;

   // Код едока
   let user_id = cx.update.from().unwrap().id;
               
   // Разбираем команду.
   match cx.update.text() {
      None => {
         next_with_cancel(cx, "Текстовое сообщение, пожалуйста!").await
      }
      Some(command) => {
         match cmd::EaterRest::from(compact_mode, command) {
            // В корзину
            cmd::EaterRest::Basket => {
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
               eat_group_now::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, (compact_mode, rest_id))).await
            }

            cmd::EaterRest::UnknownCommand => {
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
                     next(cmd::Dialogue::MessageToCaterer(user_id, caterer_id, Box::new(cmd::Dialogue::EatRestNowSelectionMode(compact_mode)), Box::new(cmd::EaterRest::markup())))
                  }
                  cmd::Common::UnknownCommand => {
                     let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                     next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, compact_mode), "Вы в меню выбора ресторана: неизвестная команда").await
                  }
               }
            }
         }
      }
   }
}

// Формирует инлайн кнопки по данным из БД
//
fn make_markup(rest_list: db::RestaurantList) -> InlineKeyboardMarkup {
   // Создадим кнопки под рестораны
   let buttons: Vec<InlineKeyboardButton> = rest_list.into_iter()
   .map(|(rest_num, title)| (InlineKeyboardButton::callback(title, format!("rng{}", db::make_key_3_int(rest_num, 0, 0)))))
   .collect();

   let (long, mut short) : (Vec<_>, Vec<_>) = buttons
   .into_iter()
   .partition(|n| n.text.chars().count() > 21);

   // Последняя непарная кнопка, если есть
   let last = if short.len() % 2 == 1 { short.pop() } else { None };

   // Сначала длинные кнопки по одной
   let markup = long.into_iter() 
   .fold(InlineKeyboardMarkup::default(), |acc, item| acc.append_row(vec![item]));

   // Короткие по две в ряд
   let markup = short.into_iter().array_chunks::<[_; 2]>()
   .fold(markup, |acc, [left, right]| acc.append_row(vec![left, right]));
   
   // Возвращаем результат
   if let Some(last_button) = last {
      markup.append_row(vec![last_button])
   } else {
      markup
   }
}

// Выводит инлайн кнопки, редактируя предыдущее сообщение
pub async fn show_inline_interface(cx: &DispatcherHandlerCx<CallbackQuery>) -> bool {
   // Текущее время
   let our_timezone = db::TIME_ZONE.get().unwrap();
   let now = Utc::now().with_timezone(our_timezone).naive_local().time();
   
   // Получаем информацию из БД
   let rest_list = db::restaurant_by_now(now).await;

   // Создадим кнопки
   let markup = make_markup(rest_list);

   // Достаём chat_id
   let message = cx.update.message.as_ref().unwrap();
   let chat_message = ChatOrInlineMessage::Chat {
      chat_id: ChatId::Id(message.chat_id()),
      message_id: message.id,
   };

   // Приготовим структуру для редактирования
   let media = InputMedia::Photo{
      media: InputFile::file_id(db::default_photo_id()),
      caption: Some( format!("Рестораны, открытые сейчас ({}):", now.format("%H:%M"))),
      parse_mode: None,
   };

   // Отправляем изменения
   match cx.bot.edit_message_media(chat_message, media)
   .reply_markup(markup)
   .send()
   .await {
      Err(e) => {
         db::log(&format!("Error eat_rest::show_inline_interface {}", e)).await;
         false
      }
      _ => true,
   }
}

