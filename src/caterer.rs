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

pub async fn caterer_mode(cx: cmd::Cx<()>) -> cmd::Res {
    // Код пользователя - код ресторана
    let rest_id = cx.update.from().unwrap().id;

    // Разбираем команду.
    match cx.update.text() {
        None => {
            cx.answer("Текстовое сообщение, пожалуйста!").send().await?;
            next(cmd::Dialogue::CatererMode)
        }
        Some(command) => {
            match cmd::Caterer::from(command) {
                // Показать основное меню
                cmd::Caterer::CatererMain => {
                    let rest_info = db::rest_info(rest_id).await;
                    cx.answer(format!("{}", rest_info))
                    .reply_markup(cmd::Caterer::main_menu_markup())
                    .send()
                    .await?;

                    // Остаёмся в режиме ресторатора
                    next(cmd::Dialogue::CatererMode)
                }

                // Выйти из режима ресторатора
                cmd::Caterer::CatererExit => {
                    return eater::start(cx).await
                }

                // Изменение названия ресторана
                cmd::Caterer::EditRestTitle => {
                    cx.answer(format!("Введите название (/ для отмены)"))
                    .reply_markup(cmd::Caterer::slash_markup())
                    .send()
                    .await?;
                    next(cmd::Dialogue::EditRestTitle)
                }
                cmd::Caterer::EditRestInfo => {
                    cx.answer(format!("Введите описание (адрес, контакты)"))
                    .reply_markup(cmd::Caterer::slash_markup())
                    .send()
                    .await?;
                    next(cmd::Dialogue::EditRestInfo)
                }
                // Переключение активности ресторана
                cmd::Caterer::ToggleRestPause => {
                    // Без запроса доп.данных переключаем активность
                    db::rest_toggle(rest_id).await;
                    next(cmd::Dialogue::CatererMode)
                }
                cmd::Caterer::EditGroup(group_id) => {
                    cx.answer(format!("К какой категории отнесём группу?"))
                    .reply_markup(cmd::Caterer::category_markup())
                    .send()
                    .await?;
                    next(cmd::Dialogue::EditGroup(group_id))
                }
                cmd::Caterer::AddGroup => {
                    cx.answer(format!("К какой категории отнесём группу?"))
                    .reply_markup(cmd::Caterer::category_markup())
                    .send()
                    .await?;
                    next(cmd::Dialogue::AddGroup)
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

pub async fn edit_rest_title_mode(cx: cmd::Cx<()>) -> cmd::Res {
    if let Some(text) = cx.update.text() {
        // Удалим из строки слеши
        let s = cmd::remove_slash(text).await;

        // Если строка не пустая, продолжим
        if !s.is_empty() {
            // Код пользователя - код ресторана
            let rest_id = cx.update.from().unwrap().id;
        
            db::rest_edit_title(rest_id, s).await;

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
}
    next(cmd::Dialogue::CatererMode)
}

pub async fn edit_rest_info_mode(cx: cmd::Cx<()>) -> cmd::Res {
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
    }
    next(cmd::Dialogue::CatererMode)
}


pub async fn edit_rest_group_mode(cx: cmd::Cx<i32>) -> cmd::Res {
    match cx.update.text() {
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
    };

    next(cmd::Dialogue::CatererMode)
}

pub async fn add_rest_group(cx: cmd::Cx<()>) -> cmd::Res {
    if let Some(text) = cx.update.text() {
         // Удалим из строки слеши
         let s = cmd::remove_slash(text).await;

         // Если строка не пустая, продолжим
         if !s.is_empty() {
            // Код пользователя - код ресторана
            let rest_id = cx.update.from().unwrap().id;
            db::rest_add_group(rest_id, s).await;

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
    }
    next(cmd::Dialogue::CatererMode)
}

pub async fn edit_rest_group_category(cx: cmd::Cx<(i32, i32)>) -> cmd::Res {
    if let Some(text) = cx.update.text() {
        // Удалим из строки слеши
        let s = cmd::remove_slash(text).await;

        // Если строка не пустая, продолжим
        if !s.is_empty() {
            // Код пользователя - код ресторана
            let rest_id = cx.update.from().unwrap().id;

            // Код категории и группы
            let (category_id, group_id) = cx.dialogue;

            db::rest_edit_group(rest_id, category_id, group_id, s).await;

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
   }
   next(cmd::Dialogue::CatererMode)

}

