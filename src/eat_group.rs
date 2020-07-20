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
   types::{InputFile, ReplyMarkup, CallbackQuery, InlineKeyboardButton, 
      ChatOrInlineMessage, InlineKeyboardMarkup, ChatId, InputMedia
   },
};
use arraylib::iter::IteratorExt;

use crate::commands as cmd;
use crate::database as db;
use crate::eater;
use crate::eat_rest;
use crate::eat_dish;
use crate::basket;
use crate::language as lang;
use crate::settings;

// Основная информация режима
pub async fn next_with_info(cx: cmd::Cx<(bool, i32, i32)>) -> cmd::Res {
   // Извлечём параметры
   let (compact_mode, cat_id, rest_num) = cx.dialogue;
   
   // Получаем информацию из БД сначала о ресторане
   match db::restaurant(db::RestBy::Num(rest_num)).await {
      None => {
         // Такая ситуация не должна возникнуть
         let s = String::from("Ошибка, информации о ресторане нет");
         let new_cx = DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ());
         cmd::send_text(&new_cx, &s, cmd::EaterRest::markup()).await;
      }
      Some(rest) => {
         // Сформируем информацию о ресторане
         let rest_info = format!("Заведение: {}\nОписание: {}\nОсновное время работы: {}-{}", rest.title, rest.info, db::str_time(rest.opening_time), db::str_time(rest.closing_time));

         // Получаем из БД список групп
         let groups_desc = match db::group_list(db::GroupListBy::Category(rest_num, cat_id)).await {
            None => {
               // Такая ситуация может возникнуть, если ресторатор скрыл группы только что
               String::from(lang::t("ru", lang::Res::EatGroupsEmpty))
            }
            Some(groups) => {
               // Сформируем строку вида "название /ссылка\n"
               groups.into_iter().map(|group| (format!("   {} /grou{}\n", group.title_with_time(rest.opening_time, rest.closing_time), group.num))).collect()
            }
         };
               
         // Формируем итоговую информацию
         let s = format!("{}\n{}", rest_info, groups_desc);

         // Отображаем информацию о группах ресторана. Если для ресторана задана картинка, то текст будет комментарием
         if let Some(image_id) = rest.image_id {
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
      }
   }

   // Переходим (остаёмся) в режим выбора группы
   next(cmd::Dialogue::EatRestGroupSelectionMode(compact_mode, cat_id, rest_num))
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
pub async fn handle_commands(cx: cmd::Cx<(bool, i32, i32)>) -> cmd::Res {
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
               // Сохраним текущее состояние для возврата
               let origin = Box::new(cmd::DialogueState{ d : cmd::Dialogue::EatRestGroupSelectionMode(compact_mode, cat_id, rest_id), m : cmd::EaterGroup::markup()});

               // Возможно это общая команда
               if let Some(res) = eater::handle_common_commands(DialogueDispatcherHandlerCx::new(cx.bot.clone(), cx.update.clone(), ()), command, origin).await {return res;}
               else {
                  let s = String::from(command);
                  let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                  next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, (compact_mode, cat_id, rest_id)), &format!("Вы в меню выбора группы: неизвестная команда '{}'", s)).await
               }
            }
         }
      }
   }
}


// Выводит инлайн кнопки
pub async fn show_inline_interface(cx: &DispatcherHandlerCx<CallbackQuery>, rest_num: i32, cat_id: i32) -> bool {

   // Получаем информацию из БД - нужен текст, картинка и кнопки
   let (text, markup, photo_id) = match db::restaurant(db::RestBy::Num(rest_num)).await {
      None => {
         // Такая ситуация не должна возникнуть

         // Кнопка назад
         let buttons = vec![InlineKeyboardButton::callback(String::from("Назад"), format!("rca{}", db::make_key_3_int(cat_id, 0, 0)))];
         let markup = InlineKeyboardMarkup::default()
         .append_row(buttons);

         // Сформированные данные
         (String::from("Ошибка, информации о ресторане нет"), markup, settings::default_photo_id())
      }
      Some(rest) => {
         // Сформируем информацию о ресторане
         let rest_info = format!("Заведение: {}\nОписание: {}\nОсновное время работы: {}-{}", rest.title, rest.info, db::str_time(rest.opening_time), db::str_time(rest.closing_time));

         // Получаем из БД список групп
         let (markup, photo_id) = match db::group_list(db::GroupListBy::Category(rest_num, cat_id)).await {
            None => {
               // Такая ситуация может возникнуть, если ресторатор скрыл группы только что
               let buttons = vec![InlineKeyboardButton::callback(String::from("Назад"), format!("rca{}", db::make_key_3_int(cat_id, 0, 0)))];
               let markup = InlineKeyboardMarkup::default()
               .append_row(buttons);
               (markup, settings::default_photo_id())
            }
            Some(groups) => {
               // Создадим кнопки
               let buttons: Vec<InlineKeyboardButton> = groups.into_iter()
               .map(|group| (InlineKeyboardButton::callback(group.title_with_time(rest.opening_time, rest.closing_time), format!("drg{}", db::make_key_3_int(rest.num, group.num, cat_id)))))
               .collect();

               // Поделим на длинные и короткие
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
            
               // Кнопка назад
               let button_back = InlineKeyboardButton::callback(String::from("Назад"), format!("rca{}", db::make_key_3_int(cat_id, 0, 0)));

               // Добавляем последнюю непарную кнопку и кнопку назад
               let markup = if let Some(last_button) = last {
                  markup.append_row(vec![last_button, button_back])
               } else {
                  markup.append_row(vec![button_back])
               };

               // Если у ресторана есть собственная картинка, вставим её, иначе плашку
               let photo_id = match rest.image_id {
                  Some(photo) => photo,
                  None => settings::default_photo_id(),
               };

               // Сформированные данные
               (markup, photo_id)
            }
         };

         (rest_info, markup, photo_id)
      }
   };

   // Достаём chat_id
   let message = cx.update.message.as_ref().unwrap();
   let chat_message = ChatOrInlineMessage::Chat {
      chat_id: ChatId::Id(message.chat_id()),
      message_id: message.id,
   };

   // Приготовим структуру для редактирования
   let media = InputMedia::Photo{
      media: InputFile::file_id(photo_id),
      caption: Some(text),
      parse_mode: None,
   };

   // Отправляем изменения
   match cx.bot.edit_message_media(chat_message, media)
   .reply_markup(markup)
   .send()
   .await {
      Err(e) => {
         settings::log(&format!("Error eat_group::show_inline_interface {}", e)).await;
         false
      }
      _ => true,
   }
}
