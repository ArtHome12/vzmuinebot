/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Начало диалога и обработка в режиме едока. 01 June 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use chrono::{Utc, FixedOffset};
use teloxide::{
    prelude::*, 
};


use crate::commands as cmd;
use crate::database as db;

pub async fn start(cx: cmd::Cx<()>) -> cmd::Res {
    // Отображаем приветственное сообщение и меню с кнопками.
    cx.answer("Бот перезапущен. Пожалуйста, выберите, какие заведения показать в основном меню снизу.")
        .reply_markup(cmd::User::main_menu_markup())
        .send()
        .await?;
    
    // Переходим в режим получения выбранного пункта в главном меню.
    next(cmd::Dialogue::UserMode)
}

pub async fn user_mode(cx: cmd::Cx<()>) -> cmd::Res {
    // Разбираем команду.
    match cx.update.text() {
        None => {
            cx.answer("Текстовое сообщение, пожалуйста!").send().await?;
        }
        Some(command) => {
            match cmd::User::from(command) {
                cmd::User::Water |
                cmd::User::Food |
                cmd::User::Alcohol | 
                cmd::User::Entertainment => {
                    // Отобразим все рестораны, у которых есть в меню выбранная категория.
                    let rest_list = db::restaurant_by_category_from_db(command.to_string()).await;
                    cx.answer(format!("Рестораны с меню в категории {}{}", command, rest_list))
                        .send().await?;
                }
                cmd::User::OpenedNow => {
                    let our_timezone = FixedOffset::east(7 * 3600);
                    let now = Utc::now().with_timezone(&our_timezone).format("%H:%M");
                    cx.answer(format!("Рестораны, открытые сейчас ({})\nКоманда в разработке", now)).send().await?;
                }
                cmd::User::RestaurantMenuInCategory(cat_id, rest_id) => {
                    // Отобразим категорию меню ресторана.rest_id
                    let menu_list = db::dishes_by_restaurant_and_category_from_db(cat_id.to_string(), rest_id.to_string()).await;
                    cx.answer(format!("Меню в категории {} ресторана {}{}", cat_id, rest_id, menu_list)).send().await?;
                }
                cmd::User::RestaurantOpenedCategories(rest_id) => {
                    cx.answer(format!("Доступные категории ресторана {}", rest_id)).send().await?;
                }
                cmd::User::DishInfo(dish_id) => {
                    // Отобразим информацию о выбранном блюде.
                    let dish = db::dish(dish_id.to_string()).await;
                    match dish {
                        None => {
                        }
                        Some(dish_info) => {
                            cx.answer_photo(dish_info.img)
                            .caption(format!("Цена {} тыс. ₫\n{}", dish_info.price, dish_info.desc))
                            .send()
                            .await?;
                        }
                    }
                }
                cmd::User::CatererMode => {
                    if let Some(user) = cx.update.from() {
                        if db::is_rest_owner(user.id).await {
                            // Запрос к БД
                            let rest_info = db::rest_info(user.id).await;

                            // Отображаем информацию о ресторане и добавляем кнопки меню
                            cx.answer(format!("{}\n\nUser Id={}\n{}", cmd::Caterer::WELCOME_MSG, user.id, rest_info))
                            .reply_markup(cmd::Caterer::main_menu_markup())
                                .send()
                                .await?;
                            return next(cmd::Dialogue::CatererMode);
                        } else {
                            cx.answer(format!("Для доступа в режим рестораторов обратитесь к @vzbalmashova и сообщите ей свой Id={}", user.id))
                            .send().await?;
                        }
                    }
                }
                cmd::User::Repeat => {
                    cx.answer("Команда в разработке").send().await?;
                }
                cmd::User::UnknownCommand => {
                    cx.answer(format!("Неизвестная команда {}", command)).send().await?;
                }
            }
        }
    }

    // Остаёмся в пользовательском режиме.
    next(cmd::Dialogue::UserMode)
}

