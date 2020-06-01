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

// Показывает информацию о ресторане 
//
async fn show_rest_info(cx: cmd::Cx<i32>) -> cmd::Res {
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

async fn show_cancel(cx: cmd::Cx<i32>) -> cmd::Res {
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
                cmd::Caterer::CatererMain(rest_id) => {
                    // Покажем информацию
                    let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                    show_rest_info(DialogueDispatcherHandlerCx::new(bot, update, rest_id)).await
                }

                // Выйти из режима ресторатора
                cmd::Caterer::CatererExit => {
                    eater::start(cx).await
                }

                // Изменение названия ресторана
                cmd::Caterer::EditRestTitle(rest_id) => {

                    // Отправляем приглашение ввести строку со слешем в меню для отмены
                    cx.answer(format!("Введите название (/ для отмены)"))
                    .reply_markup(cmd::Caterer::slash_markup())
                    .send()
                    .await?;

                    // Переходим в режим ввода нового названия ресторана
                    next(cmd::Dialogue::CatEditRestTitle(rest_id))
                }

                // Изменение информации о ресторане
                cmd::Caterer::EditRestInfo(rest_id) => {

                    // Отправляем приглашение ввести строку со слешем в меню для отмены
                    cx.answer(format!("Введите описание (адрес, контакты)"))
                    .reply_markup(cmd::Caterer::slash_markup())
                    .send()
                    .await?;

                    // Переходим в режим ввода информации о ресторане
                    next(cmd::Dialogue::CatEditRestInfo(rest_id))
                }

                // Переключение активности ресторана
                cmd::Caterer::ToggleRestPause(rest_id) => {
                    // Запрос доп.данных не требуется, сразу переключаем активность
                    db::rest_toggle(rest_id).await;

                    // Покажем изменённую информацию
                    let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                    show_rest_info(DialogueDispatcherHandlerCx::new(bot, update, rest_id)).await
                }

                // Команда редактирования групп ресторана
                cmd::Caterer::EditGroup(rest_id, group_id) => {

                    // Запрос к БД с информацией о группах
                    let rest_info = db::group_info(rest_id, group_id).await;

                    // Отображаем информацию о группе и оставляем кнопки главного меню
                    cx.answer(format!("{}", rest_info))
                    .reply_markup(cmd::Caterer::main_menu_markup())
                        .send()
                        .await?;

                    // Переходим в режим редактирования группы.
                    next(cmd::Dialogue::CatEditGroup(rest_id, group_id))
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
            show_rest_info(cx).await

        } else {
            // Сообщим об отмене
            show_cancel(cx).await
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
            show_rest_info(cx).await

        } else {
            // Сообщим об отмене
            show_cancel(cx).await
        }
    } else {
        next(cmd::Dialogue::CatererMode)
    }
}


// Режим редактирования у ресторана rest_id группы group_id
pub async fn edit_rest_group_mode(_cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
 /*   // Редактирование названия категории
    if let Some(text) = cx.update.text() {
        // Удалим из строки слеши
        let s = cmd::remove_slash(text).await;

        // Если строка не пустая, продолжим
        if !s.is_empty() {
            // Код пользователя - код ресторана
            let rest_id = cx.update.from().unwrap().id;
            db::rest_edit_info(rest_id, s).await;

            // Снова покажем главное меню
            let rest_info = db::rest_info(rest_id).await;
            cx.answer(format!("{}", rest_info))
            .reply_markup(cmd::Caterer::main_menu_markup())
            .send()
            .await?;
        } else {
            cx.answer(format!("Отмена"))
            .reply_markup(cmd::Caterer::main_menu_markup())
            .send()
            .await?;
        }
    }*/

    /*match cx.update.text() {
        None => {
            cx.answer("Текстовое сообщение, пожалуйста!").send().await?;
        }
        Some(command) => {
            let category_id:i32 = match cmd::User::from(command) {
                cmd::User::Water => 1,
                cmd::User::Food => 2,
                cmd::User::Alcohol => 3, 
                cmd::User::Entertainment => 4,
                _ => 0,
            };

                // Если категория успешно задана, переходим к вводу названия
            if category_id > 1 {
                cx.answer(format!("Введите название (/ для отмены)"))
                .reply_markup(cmd::Caterer::slash_markup())
                .send()
                .await?;

                let group_id = cx.dialogue;
                return next(cmd::Dialogue::EditGroupCategory(category_id, group_id));
            } else {
                cx.answer(format!("Отмена"))
                .reply_markup(cmd::Caterer::main_menu_markup())
                .send()
                .await?;
            }
        }
    };*/

    next(cmd::Dialogue::CatererMode)
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
            show_rest_info(cx).await

        } else {
            // Сообщим об отмене
            show_cancel(cx).await
        }
    } else {
        next(cmd::Dialogue::CatererMode)
    }
}

