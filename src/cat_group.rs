/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Обработка диалога редактирования группы блюд ресторана. 02 June 2020.
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

// Показывает информацию о группе 
//
pub async fn next_with_info(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
    // Извлечём параметры
    let (rest_id, group_id) = cx.dialogue;
    
    // Запрос к БД с информацией о группе
    let rest_info = db::group_info(rest_id, group_id).await;

    // Отображаем информацию о группе и оставляем кнопки главного меню
    cx.answer(format!("{}", rest_info))
    .reply_markup(cmd::Caterer::main_menu_markup())
        .send()
        .await?;

    // Переходим (остаёмся) в режим редактирования группы.
    next(cmd::Dialogue::CatEditGroup(rest_id, group_id))
}

async fn next_with_cancel(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
    cx.answer(format!("Отмена"))
    .reply_markup(cmd::Caterer::main_menu_markup())
    .send()
    .await?;

    // Остаёмся в режиме главного меню ресторатора.
    next(cmd::Dialogue::CatererMode)
}


// Режим редактирования у ресторана rest_id группы group_id
pub async fn edit_rest_group_mode(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
     
    // Извлечём параметры
    let (rest_id, group_id) = cx.dialogue;
    
    // Разбираем команду.
     match cx.update.text() {
        None => {
            cx.answer("Текстовое сообщение, пожалуйста!").send().await?;

            // Остаёмся в режиме редактирования группы
            next(cmd::Dialogue::CatEditGroup(rest_id, group_id))
        }
        Some(command) => {
            match cmd::CatGroup::from(rest_id, group_id, command) {

                // Изменение названия группы
                cmd::CatGroup::EditTitle(rest_id, group_id) => {

                    // Отправляем приглашение ввести строку со слешем в меню для отмены
                    cx.answer(format!("Введите название (/ для отмены)"))
                    .reply_markup(cmd::Caterer::slash_markup())
                    .send()
                    .await?;

                    // Переходим в режим ввода нового названия
                    next(cmd::Dialogue::CatEditGroupTitle(rest_id, group_id))
                }

                // Изменение информации о ресторане
                cmd::CatGroup::EditInfo(rest_id, group_id) => {

                    // Отправляем приглашение ввести строку со слешем в меню для отмены
                    cx.answer(format!("Введите пояснения для группы"))
                    .reply_markup(cmd::Caterer::slash_markup())
                    .send()
                    .await?;

                    // Переходим в режим ввода информации о ресторане
                    next(cmd::Dialogue::CatEditGroupInfo(rest_id, group_id))
                }

                // Переключение активности ресторана
                cmd::CatGroup::TogglePause(rest_id, group_id) => {
                    // Запрос доп.данных не требуется, сразу переключаем активность
                    db::rest_group_toggle(rest_id, group_id).await;

                    // Покажем изменённую информацию
                    let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                    next_with_info(DialogueDispatcherHandlerCx::new(bot, update, (rest_id, group_id))).await
                }

                cmd::CatGroup::UnknownCommand => {
                    cx.answer(format!("Неизвестная команда {}", command)).send().await?;
                    next(cmd::Dialogue::CatEditGroup(rest_id, group_id))
                }
/*                _ => {
                    cx.answer(format!("В разработке")).send().await?;
                    next(cmd::Dialogue::CatererMode)
                }*/
            }
        }
    }
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


