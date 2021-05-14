/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Обработка диалога редактирования блюда ресторана. 03 June 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{
    prelude::*,
    types::{InputFile, ReplyMarkup, ParseMode, },
};

use crate::commands as cmd;
use crate::database as db;
use crate::eater;
use crate::caterer;
use crate::cat_group;
use crate::settings;

// Показывает информацию о блюде 
//
pub async fn next_with_info(cx: cmd::Cx<(i32, i32, i32)>) -> cmd::Res {
   // Извлечём параметры
   let (rest_num, group_num, dish_num) = cx.dialogue;

   // Получаем информацию из БД
   let (info, dish_image_id) = match db::dish(db::DishBy::All(rest_num, group_num, dish_num)).await {
      Some(dish) => (dish.info_for_caterer(), dish.image_id),
      None => (String::from("Информация недоступна"), None)
   };

   // Отображаем информацию о блюде и оставляем кнопки главного меню. Если для блюда задана картинка, то текст будет комментарием
   if let Some(image_id) = dish_image_id {
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

   // Переходим (остаёмся) в режим редактирования блюда
   next(cmd::Dialogue::CatEditDish(rest_num, group_num, dish_num))
}

async fn next_with_cancel(cx: cmd::Cx<(i32, i32, i32)>, text: &str) -> cmd::Res {
    cx.answer(text)
    .reply_markup(cmd::Caterer::main_menu_markup())
    .disable_notification(true)
    .send()
    .await?;

    // Извлечём параметры
    let (rest_num, group_num, dish_num) = cx.dialogue;

    // Остаёмся в режиме редактирования блюда
    next(cmd::Dialogue::CatEditDish(rest_num, group_num, dish_num))
}

// Режим редактирования у ресторана rest_id группы group_id
pub async fn handle_commands(cx: cmd::Cx<(i32, i32, i32)>) -> cmd::Res {
   // Извлечём параметры
   let (rest_num, group_num, dish_num) = cx.dialogue;
   
   // Разбираем команду
   match cx.update.text() {
      None => {
         let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
         next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, (rest_num, group_num, dish_num)), "Текстовое сообщение, пожалуйста!").await
      }
      Some(command) => {
         match cmd::CatDish::from(rest_num, group_num, dish_num, command) {

            // Показать информацию о ресторане (возврат в главное меню ресторатора)
            cmd::CatDish::Main(rest_id) => {
               // Покажем информацию
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               caterer::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, rest_id), false).await
            }

            // Выйти из режима ресторатора
            cmd::CatDish::Exit => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               eater::start(DialogueDispatcherHandlerCx::new(bot, update, ()), false).await
            }

            // Изменение названия блюда
            cmd::CatDish::EditTitle(rest_num, group_num, dish_num) => {

               // Отправляем приглашение ввести строку со слешем в меню для отмены
               cx.answer(format!("Введите название (/ для отмены)"))
               .reply_markup(cmd::Caterer::slash_markup())
               .disable_notification(true)
               .send()
               .await?;

               // Переходим в режим ввода нового названия
               next(cmd::Dialogue::CatEditDishTitle(rest_num, group_num, dish_num))
            }

            // Изменение информации о блюде
            cmd::CatDish::EditInfo(rest_num, group_num, dish_num) => {

               // Отправляем приглашение ввести строку со слешем в меню для отмены
               cx.answer(format!("Введите пояснения для блюда (пояснения короче 3 символов клиентам показываться не будут)"))
               .reply_markup(cmd::Caterer::slash_markup())
               .disable_notification(true)
               .send()
               .await?;

               // Переходим в режим ввода информации о блюде
               next(cmd::Dialogue::CatEditDishInfo(rest_num, group_num, dish_num))
            }

            // Переключение активности блюда
            cmd::CatDish::TogglePause(rest_num, group_num, dish_num) => {
               // Запрос доп.данных не требуется, сразу переключаем активность
               db::rest_dish_toggle(rest_num, group_num, dish_num).await;

               // Покажем изменённую информацию
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_info(DialogueDispatcherHandlerCx::new(bot, update, (rest_num, group_num, dish_num))).await
            }

            // Изменить группу блюда
            cmd::CatDish::EditGroup(rest_num, group_num, dish_num) => {

               // Отправляем приглашение ввести строку с категориями в меню для выбора
               cx.answer(format!("Введите номер группы для переноса блюда из текущей группы в другую"))
               .reply_markup(cmd::Caterer::main_menu_markup())
               .disable_notification(true)
               .send()
               .await?;

               // Переходим в режим ввода информации о блюде
               next(cmd::Dialogue::CatEditDishGroup(rest_num, group_num, dish_num))
            }

            // Изменить цену блюда
            cmd::CatDish::EditPrice(rest_num, group_num, dish_num) => {

               // Отправляем приглашение ввести строку с категориями в меню для выбора
               cx.answer(format!("Введите сумму в тыс. донгов"))
               .reply_markup(cmd::Caterer::main_menu_markup())
               .disable_notification(true)
               .send()
               .await?;

               // Переходим в режим ввода информации о блюде
               next(cmd::Dialogue::CatEditDishPrice(rest_num, group_num, dish_num))
            }

            // Изменить картинку
            cmd::CatDish::EditImage(rest_num, group_num, dish_num) => {

               // Отправляем приглашение ввести строку с категориями в меню для выбора
               cx.answer(format!("Загрузите картинку"))
               .reply_markup(cmd::Caterer::main_menu_markup())
               .disable_notification(true)
               .send()
               .await?;

               // Переходим в режим ввода картинки блюда
               next(cmd::Dialogue::CatEditDishImage(rest_num, group_num, dish_num))
            }

            // Удалить блюдо
            cmd::CatDish::Remove(rest_num, group_num, dish_num) => {

               // Удаяем
               db::rest_dish_remove(rest_num, group_num, dish_num).await;

               // Сообщение в лог
               let text = format!("{} удалил блюдо {}", db::user_info(cx.update.from(), false), db::make_key_3_int(rest_num, group_num, dish_num));
               settings::log(&text).await;

               // Блюда больше нет, показываем меню группы
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               cat_group::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, (rest_num, group_num))).await
            }

            // Рекламировать блюдо
            cmd::CatDish::Promote(rest_num, group_num, dish_num) => {
               // Получаем информацию из БД
               let (info, dish_image_id) = match db::dish(db::DishBy::Active(rest_num, group_num, dish_num)).await {
                  Some(dish) => (dish.info_for_eater(), dish.image_id),
                  None => (String::from("Информация недоступна для клиента, возможно позиция скрыта"), None)
               };

               // Добавляем гиперссылку
               let info = format!("{}\n{}{}", info, settings::link(), db::make_key_3_int(rest_num, group_num, dish_num));

               // Отображаем информацию о блюде и оставляем кнопки главного меню. Если для блюда задана картинка, то текст будет комментарием
               if let Some(image_id) = dish_image_id {
                  // Создадим графический объект
                  let image = InputFile::file_id(image_id);

                  // Отправляем картинку и текст как комментарий
                  cx.answer_photo(image)
                  .caption(info)
                  .parse_mode(ParseMode::HTML)
                  .reply_markup(ReplyMarkup::ReplyKeyboardMarkup(cmd::Caterer::main_menu_markup()))
                  .disable_notification(true)
                  .send()
                  .await?;
               } else {
                  cx.answer(info)
                  .parse_mode(ParseMode::HTML)
                  .reply_markup(cmd::Caterer::main_menu_markup())
                  .disable_notification(true)
                  .send()
                  .await?;
               }
               // Остаёмся в прежнем режиме
               next(cmd::Dialogue::CatEditDish(rest_num, group_num, dish_num))
            }

            // Ошибочная команда
            cmd::CatDish::UnknownCommand => {
               // Сохраним текущее состояние для возврата
               let origin = Box::new(cmd::DialogueState{ d : cmd::Dialogue::CatEditDish(rest_num, group_num, dish_num), m : cmd::Caterer::main_menu_markup()});

               // Возможно это общая команда
               if let Some(res) = eater::handle_common_commands(DialogueDispatcherHandlerCx::new(cx.bot.clone(), cx.update.clone(), ()), command, origin).await {return res;}
               else {
                  let s = String::from(command);
                  let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                  next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, (rest_num, group_num, dish_num)), &format!("Вы в меню блюда: неизвестная команда '{}'", s)).await
               }
            }
         }
      }
   }
}

// Изменение названия rest_id, dish_id
//
pub async fn edit_title_mode(cx: cmd::Cx<(i32, i32, i32)>) -> cmd::Res {
    
    if let Some(text) = cx.update.text() {
        // Удалим из строки слеши
        let s = cmd::remove_slash(text).await;

        // Если строка не пустая, продолжим
        if !s.is_empty() {
            // Извлечём параметры
            let (rest_num, group_num, dish_num) = cx.dialogue;
        
            // Сохраним новое значение в БД
            db::rest_dish_edit_title(rest_num, group_num, dish_num, s).await;

            // Покажем изменённую информацию о группе
            return next_with_info(cx).await;
        }
    } 
    // Сообщим об отмене
    next_with_cancel(cx, "Отмена ввода названия").await
}

// Изменение описания rest_id, dish_id
//
pub async fn edit_info_mode(cx: cmd::Cx<(i32, i32, i32)>) -> cmd::Res {
    if let Some(text) = cx.update.text() {
        // Удалим из строки слеши
        let s = cmd::remove_slash(text).await;

        // Если строка не пустая, продолжим
        if !s.is_empty() {
            // Извлечём параметры
            let (rest_num, group_num, dish_num) = cx.dialogue;
        
            // Сохраним новое значение в БД
            db::rest_dish_edit_info(rest_num, group_num, dish_num, s).await;

            // Покажем изменённую информацию о группе
            return next_with_info(cx).await;
      }
    } 
    // Сообщим об отмене
    next_with_cancel(cx, "Отмена ввода описания").await
}

// Изменение группы блюда rest_id, dish_id
//
pub async fn edit_dish_group_mode(cx: cmd::Cx<(i32, i32, i32)>) -> cmd::Res {
    if let Some(text) = cx.update.text() {
        // Попытаемся преобразовать ответ пользователя в код группы
        let new_group_id = text.parse::<i32>().unwrap_or_default();

        // Если группа не пустая, продолжим
        if new_group_id > 0 {
            // Извлечём параметры
            let (rest_num, group_num, dish_num) = cx.dialogue;
        
            // Сохраним новое значение в БД
            if db::rest_dish_edit_group(rest_num, group_num, dish_num, new_group_id).await {
               // Покажем изменённую информацию о группе
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               cat_group::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, (rest_num, group_num))).await
            } else {
                // Сообщим об ошибке
                next_with_cancel(cx, "Ошибка, возможно нет группы с таким кодом, отмена изменения группы").await
            }
        } else {
            // Сообщим об ошибке
            next_with_cancel(cx, "Должно быть число 1 или больше, отмена изменения группы").await
        }
    } else {
        next_with_cancel(cx, "Ошибка, отмена изменения группы").await
    }
}

// Изменение цены rest_id, dish_id
pub async fn edit_price_mode(cx: cmd::Cx<(i32, i32, i32)>) -> cmd::Res {
    if let Some(text) = cx.update.text() {
        // Попытаемся преобразовать ответ пользователя в число
        let price = text.parse::<i32>().unwrap_or_default();

        // Извлечём параметры
        let (rest_num, group_num, dish_num) = cx.dialogue;
        
        // Сохраним новое значение в БД
        db::rest_dish_edit_price(rest_num, group_num, dish_num, price).await;
    }
    // Покажем изменённую информацию о группе
    next_with_info(cx).await
}

// Изменение картинки
pub async fn edit_image_mode(cx: cmd::Cx<(i32, i32, i32)>) -> cmd::Res {
    if let Some(photo_size) = cx.update.photo() {
        // Попытаемся преобразовать ответ пользователя в число
        let image = &photo_size[0].file_id;

        // Извлечём параметры
        let (rest_num, group_num, dish_num) = cx.dialogue;
        
        // Сохраним новое значение в БД
        db::rest_dish_edit_image(rest_num, group_num, dish_num, image).await;
    }
    // Покажем изменённую информацию о группе
    next_with_info(cx).await
}

