/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Обработка диалога редактирования группы блюд ресторана. 02 June 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use chrono::{NaiveTime};
use teloxide::{
    prelude::*, 
};


use crate::commands as cmd;
use crate::database as db;
use crate::eater;
use crate::caterer;
use crate::dish;

// Показывает информацию о группе 
//
pub async fn next_with_info(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
    // Извлечём параметры
    let (rest_id, group_id) = cx.dialogue;
    
    // Получаем информацию о группе из БД
    let info = match db::group_info(rest_id, group_id).await {
        Some(rest_info) => rest_info,
        None => format!("Ошибка db::group_info({}, {})", rest_id, group_id)
    };

    // Отображаем информацию о группе и оставляем кнопки главного меню
    cx.answer(format!("\n{}", info))
    .reply_markup(cmd::Caterer::main_menu_markup())
    .disable_notification(true)
    .send()
   .await?;

    // Переходим (остаёмся) в режим редактирования группы
    next(cmd::Dialogue::CatEditGroup(rest_id, group_id))
}

async fn next_with_cancel(cx: cmd::Cx<(i32, i32)>, text: &str) -> cmd::Res {
    cx.answer(text)
    .reply_markup(cmd::Caterer::main_menu_markup())
    .disable_notification(true)
    .send()
    .await?;

    // Извлечём параметры
    let (rest_id, group_id) = cx.dialogue;

    // Остаёмся в режиме редактирования группы
    next(cmd::Dialogue::CatEditGroup(rest_id, group_id))
}


// Режим редактирования у ресторана rest_id группы group_id
pub async fn edit_rest_group_mode(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
     
   // Извлечём параметры
   let (rest_id, group_id) = cx.dialogue;
    
   // Разбираем команду.
   match cx.update.text() {
      None => {
         let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
         next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, group_id)), "Текстовое сообщение, пожалуйста!").await
      }
      Some(command) => {
            match cmd::CatGroup::from(rest_id, group_id, command) {

               // Показать информацию о ресторане (возврат в главное меню ресторатора)
               cmd::CatGroup::Main(rest_id) => {
                  // Покажем информацию
                  let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                  caterer::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, rest_id), false).await
               }

               // Выйти из режима ресторатора
               cmd::CatGroup::Exit => {
                  let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                  eater::start(DialogueDispatcherHandlerCx::new(bot, update, ()), false).await
               }

            // Изменение названия группы
               cmd::CatGroup::EditTitle(rest_id, group_id) => {

                  // Отправляем приглашение ввести строку со слешем в меню для отмены
                  cx.answer(format!("Введите название (/ для отмены)"))
                  .reply_markup(cmd::Caterer::slash_markup())
                  .disable_notification(true)
                  .send()
                  .await?;

                  // Переходим в режим ввода нового названия
                  next(cmd::Dialogue::CatEditGroupTitle(rest_id, group_id))
               }

               // Изменение информации о группе
               cmd::CatGroup::EditInfo(rest_id, group_id) => {

                  // Отправляем приглашение ввести строку со слешем в меню для отмены
                  cx.answer(format!("Введите пояснения для группы"))
                  .reply_markup(cmd::Caterer::slash_markup())
                  .disable_notification(true)
                  .send()
                  .await?;

                  // Переходим в режим ввода информации о группе
                  next(cmd::Dialogue::CatEditGroupInfo(rest_id, group_id))
               }

               // Переключение активности группы
               cmd::CatGroup::TogglePause(rest_id, group_id) => {
                  // Запрос доп.данных не требуется, сразу переключаем активность
                  db::rest_group_toggle(rest_id, group_id).await;

                  // Покажем изменённую информацию
                  let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                  next_with_info(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, group_id))).await
               }

               // Изменить категорию группы
               cmd::CatGroup::EditCategory(rest_id, group_id) => {

                  // Отправляем приглашение ввести строку с категориями в меню для выбора
                  cx.answer(format!("Выберите категорию"))
                  .reply_markup(cmd::CatGroup::category_markup())
                  .disable_notification(true)
                  .send()
                  .await?;

                  // Переходим в режим ввода информации о ресторане
                  next(cmd::Dialogue::CatEditGroupCategory(rest_id, group_id))
               }

               // Изменить время
               cmd::CatGroup::EditTime(rest_id, group_id) => {

                  // Отправляем приглашение ввести строку с категориями в меню для выбора
                  cx.answer(format!("Введите время доступности категории"))
                  .reply_markup(cmd::Caterer::slash_markup())
                  .disable_notification(true)
                  .send()
                  .await?;

                  // Переходим в режим ввода времени
                  next(cmd::Dialogue::CatEditGroupTime(rest_id, group_id))
               }

               // Удалить группу
               cmd::CatGroup::RemoveGroup(rest_id, group_id) => {
                  // Запрос доп.данных не требуется, сразу удаяем, если это не основная.
                  if group_id > 1 && db::rest_group_remove(rest_id, group_id).await {
                     // Группы больше нет, показываем главное меню
                     let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                     caterer::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, rest_id), false).await
                  } else {
                     next_with_cancel(cx, "Нельзя удалить основную группу и группу с блюдами").await
                  }
               }

               // Добавление нового блюда
               cmd::CatGroup::AddDish(rest_id, group_id) => {

                  // Отправляем приглашение ввести строку со слешем в меню для отмены
                  cx.answer(format!("Введите название блюда (/ для отмены)"))
                  .reply_markup(cmd::Caterer::slash_markup())
                  .disable_notification(true)
                  .send()
                  .await?;

                  // Переходим в режим ввода названия блюда
                  next(cmd::Dialogue::CatAddDish(rest_id, group_id))
               }

               // Редактирование блюда
               cmd::CatGroup::EditDish(rest_id, group_id, dish_id) => {

                  // Отображаем информацию о блюде и переходим в режим её редактирования
                  let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                  dish::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, group_id, dish_id))).await
               }

               // Ошибочная команда
               cmd::CatGroup::UnknownCommand => {
                  let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                  next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, group_id)), "Вы в меню группы: неизвестная команда").await
               }
         }
      }
   }
}

// Изменение названия группы rest_id, group_id
//
pub async fn edit_title_mode(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
    
    if let Some(text) = cx.update.text() {
        // Удалим из строки слеши
        let s = cmd::remove_slash(text).await;

        // Если строка не пустая, продолжим
        if !s.is_empty() {
            // Извлечём параметры
            let (rest_id, group_id) = cx.dialogue;
        
            // Сохраним новое значение в БД
            db::rest_group_edit_title(rest_id, group_id, s).await;

            // Покажем изменённую информацию о группе
            next_with_info(cx).await

        } else {
            // Сообщим об отмене
            next_with_cancel(cx, "Отмена").await
        }
    } else {
      // Сообщим об отмене
      next_with_cancel(cx, "Отмена").await
    }
}

// Изменение описания группы rest_id, group_id
//
pub async fn edit_info_mode(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
    if let Some(text) = cx.update.text() {
        // Удалим из строки слеши
        let s = cmd::remove_slash(text).await;

        // Если строка не пустая, продолжим
        if !s.is_empty() {
            // Извлечём параметры
            let (rest_id, group_id) = cx.dialogue;
        
            // Сохраним новое значение в БД
            db::rest_group_edit_info(rest_id, group_id, s).await;

            // Покажем изменённую информацию о группе
            next_with_info(cx).await

        } else {
            // Сообщим об отмене
            next_with_cancel(cx, "Отмена").await
        }
    } else {
      // Сообщим об отмене
      next_with_cancel(cx, "Отмена").await
    }
}

// Изменение категории группы rest_id, group_id
//
pub async fn edit_category_mode(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
    if let Some(text) = cx.update.text() {
        // Попытаемся преобразовать ответ пользователя в код категории
        let cat_id = db::category_to_id(text);

        // Если категория не пустая, продолжим
        if cat_id > 0 {
            // Извлечём параметры
            let (rest_id, group_id) = cx.dialogue;
        
            // Сохраним новое значение в БД
            db::rest_group_edit_category(rest_id, group_id, cat_id).await;

            // Покажем изменённую информацию о группе
            next_with_info(cx).await

        } else {
            // Сообщим об отмене
            next_with_cancel(cx, "Неизвестная категория, отмена").await
        }
    } else {
      // Сообщим об отмене
      next_with_cancel(cx, "Отмена").await
    }
}

// Изменение времени доступности группы rest_id, group_id
//
pub async fn edit_time_mode(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
    if let Some(text) = cx.update.text() {
        // Удалим из строки слеши
        let s = cmd::remove_slash(text).await;

        // Если строка не пустая, продолжим
        if !s.is_empty() {

            // Получим первое и второе время в виде куска строки
            let part1 = s.get(..5).unwrap_or_default();
            let part2 = s.get(6..).unwrap_or_default();
        
            // Попытаемся преобразовать их во время
            if let Ok(opening_time) = NaiveTime::parse_from_str(part1, "%H:%M") {
                if let Ok(closing_time) = NaiveTime::parse_from_str(part2, "%H:%M") {

                    // Извлечём параметры
                    let (rest_id, group_id) = cx.dialogue;
                
                    // Сохраним новое значение в БД
                    db::rest_group_edit_time(rest_id, group_id, opening_time, closing_time).await;

                    // Покажем изменённую информацию о группе
                    return next_with_info(cx).await;
                }
            }
        
            // Сообщим об ошибке
            next_with_cancel(cx, "Ошибка распознавания, д.б. ЧЧ:ММ-ЧЧ-ММ").await
        } else {
            // Сообщим об отмене
            next_with_cancel(cx, "Отмена").await
        }
    } else {
      // Сообщим об отмене
      next_with_cancel(cx, "Отмена").await
    }
}


// Добавление нового блюда
//
pub async fn add_dish_mode(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
    if let Some(text) = cx.update.text() {
        // Удалим из строки слеши
        let s = cmd::remove_slash(text).await;

        // Если строка не пустая, продолжим
        if !s.is_empty() {
            // Извлечём параметры
            let (rest_id, group_id) = cx.dialogue;
        
            // Сохраним новое значение в БД
            if db::rest_add_dish(rest_id, group_id, s).await {
               // Покажем изменённую информацию о группе
               next_with_info(cx).await
            } else {
               // Сообщим об ошибке
               next_with_cancel(cx, &format!("Ошибка add_dish_mode({}, {})", rest_id, group_id)).await
            }
        } else {
            // Сообщим об отмене
            next_with_cancel(cx, "Отмена").await
        }
    } else {
      // Сообщим об отмене
      next_with_cancel(cx, "Отмена").await
    }
}

