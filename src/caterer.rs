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
};


use crate::commands as cmd;
use crate::database as db;
use crate::eater;
use crate::cat_group;

// Показывает информацию о ресторане 
//
async fn next_with_info(cx: cmd::Cx<i32>) -> cmd::Res {
    // Код ресторана
    let rest_id = cx.dialogue;

        // Получаем информацию о ресторане из БД
        let rest_info = db::rest_info(rest_id).await;

        // Отправляем её пользователю и отображаем главное меню
        cx.answer(format!("{}", rest_info))
        .reply_markup(cmd::Caterer::main_menu_markup())
        .send()
        .await?;

    // Остаёмся в режиме главного меню ресторатора.
    next(cmd::Dialogue::CatererMode)
}

async fn next_with_cancel(cx: cmd::Cx<i32>) -> cmd::Res {
    cx.answer(format!("Отмена"))
    .reply_markup(cmd::Caterer::main_menu_markup())
    .send()
    .await?;

    // Остаёмся в режиме главного меню ресторатора.
    next(cmd::Dialogue::CatererMode)
}

// Обработка команд главного меню в режиме ресторатора
//
pub async fn caterer_mode(cx: cmd::Cx<()>) -> cmd::Res {
    // Разбираем команду.
    match cx.update.text() {
        None => {
            cx.answer("Текстовое сообщение, пожалуйста!").send().await?;
            next(cmd::Dialogue::CatererMode)
        }
        Some(command) => {
            // Код ресторана
            let rest_id = cx.update.from().unwrap().id;

            match cmd::Caterer::from(rest_id, command) {

                // Показать информацию о ресторане
                cmd::Caterer::Main(rest_id) => {
                    // Покажем информацию
                    let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                    next_with_info(DialogueDispatcherHandlerCx::new(bot, update, rest_id)).await
                }

                // Выйти из режима ресторатора
                cmd::Caterer::Exit => {
                    eater::start(cx).await
                }

                // Изменение названия ресторана
                cmd::Caterer::EditTitle(rest_id) => {

                    // Отправляем приглашение ввести строку со слешем в меню для отмены
                    cx.answer(format!("Введите название (/ для отмены)"))
                    .reply_markup(cmd::Caterer::slash_markup())
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
                    next_with_info(DialogueDispatcherHandlerCx::new(bot, update, rest_id)).await
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
                    .send()
                    .await?;

                    // Переходим в режим ввода названия новой группы
                    next(cmd::Dialogue::CatAddGroup(rest_id))
                }

                cmd::Caterer::UnknownCommand => {
                    cx.answer(format!("Неизвестная команда {}", command)).send().await?;
                    next(cmd::Dialogue::CatererMode)
                }
/*                _ => {
                    cx.answer(format!("В разработке")).send().await?;
                    next(cmd::Dialogue::CatererMode)
                }*/
            }
        }
    }
}

// Изменение названия ресторана rest_id
//
pub async fn edit_rest_title_mode(cx: cmd::Cx<i32>) -> cmd::Res {
    if let Some(text) = cx.update.text() {
        // Удалим из строки слеши
        let s = cmd::remove_slash(text).await;

        // Если строка не пустая, продолжим
        if !s.is_empty() {
            // Код ресторана
            let rest_id = cx.dialogue;
        
            // Сохраним новое значение в БД
            db::rest_edit_title(rest_id, s).await;

            // Покажем изменённую информацию о ресторане
            next_with_info(cx).await

        } else {
            // Сообщим об отмене
            next_with_cancel(cx).await
        }
    } else {
        next(cmd::Dialogue::CatererMode)
    }
}

// Изменение описания ресторана rest_id
//
pub async fn edit_rest_info_mode(cx: cmd::Cx<i32>) -> cmd::Res {
    if let Some(text) = cx.update.text() {
        // Удалим из строки слеши
        let s = cmd::remove_slash(text).await;

        // Если строка не пустая, продолжим
        if !s.is_empty() {
            // Код ресторана
            let rest_id = cx.dialogue;
        
            // Сохраним новое значение в БД
            db::rest_edit_info(rest_id, s).await;

            // Покажем изменённую информацию о ресторане
            next_with_info(cx).await

        } else {
            // Сообщим об отмене
            next_with_cancel(cx).await
        }
    } else {
        next(cmd::Dialogue::CatererMode)
    }
}


pub async fn add_rest_group(cx: cmd::Cx<i32>) -> cmd::Res {
    if let Some(text) = cx.update.text() {
         // Удалим из строки слеши
         let s = cmd::remove_slash(text).await;

         // Если строка не пустая, продолжим
         if !s.is_empty() {
            // Код ресторана
            let rest_id = cx.dialogue;
        
            // Сохраним новое значение в БД
            db::rest_add_group(rest_id, s).await;

            // Покажем изменённую информацию о ресторане
            next_with_info(cx).await

        } else {
            // Сообщим об отмене
            next_with_cancel(cx).await
        }
    } else {
        next(cmd::Dialogue::CatererMode)
    }
}

