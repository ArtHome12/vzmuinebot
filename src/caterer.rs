/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Обработка диалога владельца ресторана. 01 June 2020.
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
use crate::cat_group;

// Показывает информацию о ресторане 
//
pub async fn next_with_info(cx: cmd::Cx<i32>, show_welcome: bool) -> cmd::Res {
   async fn hint(image_id : &Option<String>) -> String {
      match image_id {
         Some(_) => String::default(),
         _ => String::from("\nИзначально всё заполнено значениями по-умолчанию, отредактируйте их."),
      }
   }
   
   
   // Код ресторана
   let rest_id = cx.dialogue;

   // Получаем информацию о ресторане из БД
   let (info, image_id) = match db::rest_info(rest_id).await {
      Some(rest_info) => rest_info,
      None => (format!("Ошибка db::rest_info({})", rest_id), None)
   };

   // Дополним, при необходимости, приветствием
   let welcome_msg = if show_welcome {
      // Добавляем подсказку только если ещё нет картинки
      format!("Добро пожаловать в режим ввода меню!{}\nUser Id={}, {}", hint(&image_id).await, cx.update.from().unwrap().id, rest_id)
   } else {
      String::default()
   };

   // Итоговая информация
   let info = format!("{}\n{}", welcome_msg, info);

   // Отправляем описание пользователю, если есть картинка, то отправим описание как комментарий к ней
   if let Some(image_id) = image_id {
      // Создадим графический объект
      let image = InputFile::file_id(image_id);

      // Отправляем картинку и текст как комментарий
      cx.answer_photo(image)
      .caption(info)
      .reply_markup(ReplyMarkup::ReplyKeyboardMarkup(cmd::Caterer::main_menu_markup()))
      .disable_notification(true)
      .send()
      .await?;
   } else {
      cx.answer(info)
      .reply_markup(cmd::Caterer::main_menu_markup())
      .disable_notification(true)
      .send()
      .await?;
   }

   // Остаёмся в режиме главного меню ресторатора.
   next(cmd::Dialogue::CatererMode(rest_id))
}

async fn next_with_cancel(cx: cmd::Cx<i32>, text: &str) -> cmd::Res {
    cx.answer(text)
    .reply_markup(cmd::Caterer::main_menu_markup())
    .disable_notification(true)
    .send()
    .await?;

   // Код ресторана
   let rest_id = cx.dialogue;

   // Остаёмся в режиме главного меню ресторатора.
    next(cmd::Dialogue::CatererMode(rest_id))
}

// Обработка команд главного меню в режиме ресторатора
//
pub async fn caterer_mode(cx: cmd::Cx<i32>) -> cmd::Res {
   // Код ресторана
   let rest_id = cx.dialogue;

   // Разбираем команду.
   match cx.update.text() {
      None => {
         let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
         next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, rest_id), "Текстовое сообщение, пожалуйста!").await
      }
      Some(command) => {
         match cmd::Caterer::from(rest_id, command) {

            // Показать информацию о ресторане
            cmd::Caterer::Main(rest_id) => {
               // Покажем информацию
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_info(DialogueDispatcherHandlerCx::new(bot, update, rest_id), false).await
            }

            // Выйти из режима ресторатора
            cmd::Caterer::Exit => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               eater::start(DialogueDispatcherHandlerCx::new(bot, update, ()), false).await
            }

            // Передать управление рестораном
            cmd::Caterer::TransferOwnership(rest_id, user_id) => {
               // Проверим права
               if db::is_admin(cx.update.from()) {
                  let res = db::is_success(db::transfer_ownership(rest_id, user_id).await);
                  let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                  next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, rest_id), &format!("Передача управления новому ресторатору {}: {}", user_id, res)).await
               } else {
                  let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                  next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, rest_id), "Недостаточно прав").await
               }
            }

            // Изменение названия ресторана
            cmd::Caterer::EditTitle(rest_id) => {
               // Отправляем приглашение ввести строку со слешем в меню для отмены
               cx.answer(format!("Введите название (/ для отмены)"))
               .reply_markup(cmd::Caterer::slash_markup())
               .disable_notification(true)
               .send()
               .await?;

               // Переходим в режим ввода нового названия ресторана
               next(cmd::Dialogue::CatEditRestTitle(rest_id))
            }

            // Изменение информации о ресторане
            cmd::Caterer::EditInfo(rest_id) => {
               // Отправляем приглашение ввести строку со слешем в меню для отмены
               cx.answer(format!("Введите описание (адрес, контакты)"))
               .reply_markup(cmd::Caterer::slash_markup())
               .disable_notification(true)
               .send()
               .await?;

               // Переходим в режим ввода информации о ресторане
               next(cmd::Dialogue::CatEditRestInfo(rest_id))
            }

            // Переключение активности ресторана
            cmd::Caterer::TogglePause(rest_id) => {
               // Запрос доп.данных не требуется, сразу переключаем активность
               db::rest_toggle(rest_id).await;

               // Покажем изменённую информацию
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_info(DialogueDispatcherHandlerCx::new(bot, update, rest_id), false).await
            }

            // Изменить картинку
            cmd::Caterer::EditImage(rest_id) => {

               // Отправляем приглашение ввести строку с категориями в меню для выбора
               cx.answer(format!("Загрузите картинку"))
               .reply_markup(cmd::Caterer::main_menu_markup())
               .disable_notification(true)
               .send()
               .await?;

               // Переходим в режим ввода картинки ресторана
               next(cmd::Dialogue::CatEditRestImage(rest_id))
            }

            // Команда редактирования групп ресторана
            cmd::Caterer::EditGroup(rest_id, group_id) => {
               // Отображаем информацию о группе и переходим в режим её редактирования
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               cat_group::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, group_id))).await
            }

            // Команда добвления новой группы
            cmd::Caterer::AddGroup(rest_id) => {

               // Отправляем приглашение ввести строку со слешем в меню для отмены
               cx.answer(format!("Введите название (/ для отмены)"))
               .reply_markup(cmd::Caterer::slash_markup())
               .disable_notification(true)
               .send()
               .await?;

               // Переходим в режим ввода названия новой группы
               next(cmd::Dialogue::CatAddGroup(rest_id))
            }

            cmd::Caterer::UnknownCommand => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, rest_id), "Вы в главном меню: неизвестная команда").await
            }
         }
      }
   }
}

// Изменение названия ресторана rest_id
//
pub async fn edit_rest_title_mode(cx: cmd::Cx<i32>) -> cmd::Res {
   // Код ресторана
   let rest_id = cx.dialogue;
        
   if let Some(text) = cx.update.text() {
      // Удалим из строки слеши
      let s = cmd::remove_slash(text).await;

      // Если строка не пустая, продолжим
      if !s.is_empty() {
         // Сохраним новое значение в БД
         if db::rest_edit_title(rest_id, s).await {
            // Покажем изменённую информацию о ресторане
            next_with_info(cx, false).await
         } else {
            // Сообщим об ошибке
            next_with_cancel(cx, &format!("Ошибка rest_edit_title({})", rest_id)).await
         }
      } else {
         // Сообщим об отмене
         next_with_cancel(cx, "Отмена").await
      }
   } else {
      next(cmd::Dialogue::CatererMode(rest_id))
   }
}

// Изменение описания ресторана rest_id
//
pub async fn edit_rest_info_mode(cx: cmd::Cx<i32>) -> cmd::Res {
   // Код ресторана
   let rest_id = cx.dialogue;

   if let Some(text) = cx.update.text() {
      // Удалим из строки слеши
      let s = cmd::remove_slash(text).await;

      // Если строка не пустая, продолжим
      if !s.is_empty() {
         // Сохраним новое значение в БД
         if db::rest_edit_info(rest_id, s).await {
            // Покажем изменённую информацию о ресторане
            next_with_info(cx, false).await
         } else {
            // Сообщим об ошибке
            next_with_cancel(cx, &format!("Ошибка edit_rest_info_mode({})", rest_id)).await
         }
      } else {
         // Сообщим об отмене
         next_with_cancel(cx, "Отмена").await
      }
   } else {
      next(cmd::Dialogue::CatererMode(rest_id))
   }
}

// Изменение картинки
//
pub async fn edit_rest_image_mode(cx: cmd::Cx<i32>) -> cmd::Res {
   if let Some(photo_size) = cx.update.photo() {
       // Попытаемся преобразовать ответ пользователя в идентификатор фото
       let image = &photo_size[0].file_id;

      // Код ресторана
      let rest_id = cx.dialogue;

       // Сохраним новое значение в БД
       db::rest_edit_image(rest_id, image).await;
   }

   // Покажем изменённую информацию о ресторане
   next_with_info(cx, false).await
}


pub async fn add_rest_group(cx: cmd::Cx<i32>) -> cmd::Res {
   // Код ресторана
   let rest_id = cx.dialogue;

   if let Some(text) = cx.update.text() {
      // Удалим из строки слеши
      let s = cmd::remove_slash(text).await;

      // Если строка не пустая, продолжим
      if !s.is_empty() {
         // Сохраним новое значение в БД
         if db::rest_add_group(rest_id, s).await{
            // Покажем изменённую информацию о ресторане
            next_with_info(cx, false).await
         } else {
            // Сообщим об ошибке
            next_with_cancel(cx, &format!("Ошибка rest_add_group({})", rest_id)).await
         }
      } else {
         // Сообщим об отмене
         next_with_cancel(cx, "Отмена").await
      }
   } else {
      next(cmd::Dialogue::CatererMode(rest_id))
   }
}

