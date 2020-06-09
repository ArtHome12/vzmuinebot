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
use crate::caterer;
use crate::eat_rest;

pub async fn start(cx: cmd::Cx<()>, after_restart: bool) -> cmd::Res {
   
   // Различаем перезапуск и возврат из меню ресторатора
   let s = if after_restart {
      String::from("Бот перезапущен. Пожалуйста, выберите в основном меню снизу какие заведения показать.")
   } else {
      String::from("Пожалуйста, выберите в основном меню снизу какие заведения показать.")
   };
   
   // Отображаем приветственное сообщение и меню с кнопками.
    cx.answer(s)
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
               cmd::User::Category(cat_id) => {
                  // Отобразим все рестораны, у которых есть в меню выбранная категория и переходим в режим выбора ресторана
                  let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                  return eat_rest::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, cat_id)).await;
               }
               cmd::User::OpenedNow => {
                  let our_timezone = FixedOffset::east(7 * 3600);
                  let now = Utc::now().with_timezone(&our_timezone).format("%H:%M");
                  cx.answer(format!("Рестораны, открытые сейчас ({})\nКоманда в разработке", now)).send().await?;
               }
/*               cmd::User::RestaurantGroupsInCategory(rest_num) => {
                  // Отобразим категорию меню ресторана.rest_id
                  let group_list = db::groups_by_restaurant_and_category(rest_id, cat_id).await;
                  cx.answer(format!("Разделы меню ресторана {}{}:", rest_id, group_list)).send().await?;
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
               }*/
               cmd::User::CatererMode => {
                  // Код пользователя
                  let user_id: i32 = match cx.update.from() {
                     Some(user) => user.id,
                     None => 0,
                  };

                  // По коду пользователя получим код ресторана, если 0 то доступ запрещён
                  let rest_num = db::rest_num(user_id).await;

                  if rest_num >0 {
                     // Отображаем информацию о ресторане и переходим в режим её редактирования
                     let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                     return caterer::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, rest_num), true).await;
                  } else {
                     cx.answer(format!("Для доступа в режим рестораторов обратитесь к @vzbalmashova и сообщите ей свой Id={}", user_id))
                     .send().await?;
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

