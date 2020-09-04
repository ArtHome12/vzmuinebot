/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Начало диалога и обработка в режиме едока. 01 June 2020.
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
use crate::eat_rest;
use crate::eat_rest_now;
use crate::basket;
use crate::settings;
use crate::gear;
use crate::eat_dish;

pub async fn start(cx: cmd::Cx<()>, after_restart: bool) -> cmd::Res {
   
   // Различаем перезапуск и возврат из меню ресторатора
   let s = if after_restart {
      // Это первый вход пользователя после перезапуска, сообщим об этом
      let text = format!("{} начал сеанс", db::user_info(cx.update.from(), true));
      settings::log(&text).await;

      // Для администратора отдельное приветствие
      if settings::is_admin(cx.update.from()) {
         String::from("Начат новый сеанс. Список команд администратора в описании: https://github.com/ArtHome12/vzmuinebot")
      } else {
         String::from("Начат новый сеанс. Пожалуйста, выберите в основном меню снизу какие заведения показать, либо отправьте часть названия блюда для поиска.")
      }
   } else {
      String::from("Пожалуйста, выберите в основном меню снизу какие заведения показать, либо отправьте часть названия блюда для поиска.")
   };
   
   // Если сессия началась с какой-то команды, то попробуем сразу её обработать
   if let Some(input) = cx.update.text() {
      // Пытаемся распознать команду как собственную или глобальную
      let known = cmd::User::from(input) != cmd::User::UnknownCommand || cmd::Common::from(input) != cmd::Common::UnknownCommand;
      if known {
         let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
         return handle_commands(DialogueDispatcherHandlerCx::new(bot, update, ())).await;
      }
   }

   // Если команды не было или она не распознана, отображаем приветственное сообщение и меню с кнопками.
   cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update.clone(), ()), &s, cmd::User::main_menu_markup()).await;
   
   // Переходим в режим получения выбранного пункта в главном меню
   next(cmd::Dialogue::UserMode)
}

pub async fn handle_commands(cx: cmd::Cx<()>) -> cmd::Res {

   // Разбираем команду
   match cx.update.text() {
      None => {
         let s = match cx.update.photo() {
            Some(photo_size) => {
               let image_id = photo_size[0].file_id.to_owned();
               let s = format!("Вы прислали картинку с id\n{}", &image_id);

               // Если картинка получена от админа, предложим установить её как картинку категории
               if settings::is_admin(cx.update.from()) {
                  let s = format!("{}\nВы можете установить её как главную для бота через переменную окружения либо выберите внизу категорию, чтобы сделать её по-умолчанию для данной категории", s);
                  cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update.clone(), ()), &s, cmd::CatGroup::category_markup()).await;
                  return next(cmd::Dialogue::UserModeEditCatImage(image_id))
               } else {
                  // Иначе просто сообщим её идентификатор
                  s
               }
            }
            None => String::from("Текстовое сообщение, пожалуйста!"),
         };
         cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), &s, cmd::User::main_menu_markup()).await
      }
      Some(command) => {
         match cmd::User::from(command) {
            cmd::User::Category(cat_id) => {
               // Отобразим все рестораны, у которых есть в меню выбранная категория и переходим в режим выбора ресторана
               return eat_rest::next_with_info(DialogueDispatcherHandlerCx::new(cx.bot, cx.update, cat_id)).await;
            }
            cmd::User::OpenedNow => {
               // Отобразим рестораны, открытые сейчас и перейдём в режим их выбора
               return eat_rest_now::next_with_info(cx).await;
            }
            cmd::User::UnknownCommand => {
               // Сохраним текущее состояние для возврата
               let origin = Box::new(cmd::DialogueState{ d : cmd::Dialogue::UserMode, m : cmd::User::main_menu_markup()});

               // Возможно это общая команда
               if let Some(res) = handle_common_commands(DialogueDispatcherHandlerCx::new(cx.bot.clone(), cx.update.clone(), ()), command, origin).await {return res;}
               else {
                  let s = &format!("Вы в главном меню: неизвестная команда {}", command);
                  cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), s, cmd::User::main_menu_markup()).await
               }
            }
            cmd::User::Gear => {
               // Переходим в меню с шестерёнкой
               return gear::next_with_info(DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ())).await;
            }
            cmd::User::Basket => {
               // Код едока
               let user_id = cx.update.from().unwrap().id;
               
               // Переходим в корзину
               return basket::next_with_info(DialogueDispatcherHandlerCx::new(cx.bot, cx.update, user_id)).await;
            }
            cmd::User::ChatId => {
               // Отправим информацию о чате
               let id = cx.chat_id();
               cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), &format!("Chat id={}", id), cmd::User::main_menu_markup()).await;
            }
         }
      }
   }

   // Остаёмся в пользовательском режиме.
   next(cmd::Dialogue::UserMode)
}

// Обработка глобальных команд
pub async fn handle_common_commands(cx: cmd::Cx<()>, command: &str, origin : Box<cmd::DialogueState>) -> Option<cmd::Res> {

   async fn goto(cx: cmd::Cx<()>, rest_num: i32, group_num: i32, dish_num: i32)-> cmd::Res {

      // Запросим настройку пользователя с режимом интерфейса и обновим время последнего входа в БД
      let compact_mode = db::user_compact_interface(cx.update.from()).await;

      // Название ресторана
      let rest_name = if let Some(rest) = db::restaurant(db::RestBy::Num(rest_num)).await {rest.title} else {String::from("ошибка получения названия")};

      // Приветственное сообщение и меню с кнопками (иначе нижнего меню не будет в инлайн-режиме)
      let s = format!("Добро пожаловать в {}!", rest_name);

         // Режим "со ссылками"
      if compact_mode {
         // Приветственное сообщение с правильным меню
         cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot.clone(), cx.update.clone(), ()), &s, cmd::EaterGroup::markup()).await;

         // Если третий аргумент нулевой, надо отобразить группу
         if dish_num == 0 {
            let new_cx = DialogueDispatcherHandlerCx::new(cx.bot, cx.update, (0, rest_num, group_num));
            eat_dish::next_with_info(new_cx).await
         } else {
            // Отображаем сразу блюдо
            let new_cx = DialogueDispatcherHandlerCx::new(cx.bot, cx.update, (0, rest_num, group_num));
            let res = eat_dish::show_dish(eat_dish::DishMode::Compact(&new_cx, dish_num)).await;
            if let Err(e) = res.as_ref() {
               settings::log(&format!("Error handle_common_commands1 goto({}, {}, {}): {}", rest_num, group_num, dish_num, e)).await;
            }

            // Остаёмся в режиме выбора блюд
            res
         }
      } else {
         // Приветственное сообщение с правильным меню
         cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot.clone(), cx.update.clone(), ()), &s, cmd::User::main_menu_markup()).await;

         // Режим с инлайн-кнопками
         if dish_num == 0 {
            let new_cx = DialogueDispatcherHandlerCx::new(cx.bot, cx.update, (0, rest_num, group_num));
            if !eat_dish::force_inline_interface(new_cx).await {
               settings::log(&format!("Error handle_common_commands2 goto({}, {}, {})", rest_num, group_num, dish_num)).await;
            }
         } else {
            let new_cx = DialogueDispatcherHandlerCx::new(cx.bot, cx.update, (rest_num, group_num, dish_num));
            let res = eat_dish::show_dish(eat_dish::DishMode::Inline(&new_cx)).await;
            if let Err(e) = res {
               settings::log(&format!("Error handle_common_commands3 goto({}, {}, {}): {}", rest_num, group_num, dish_num, e)).await;
            }
         }

         // Всегда в главном меню
         next(cmd::Dialogue::UserMode)
      }
   }

   match cmd::Common::from(command) {
      cmd::Common::Start => {
         // Отображаем приветственное сообщение и меню с кнопками
         let s = "Пожалуйста, выберите в основном меню снизу какие заведения показать.";
         cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update.clone(), ()), s, cmd::User::main_menu_markup()).await;

         Some(next(cmd::Dialogue::UserMode))
      },
      cmd::Common::StartArgs(first, second, third) => Some(goto(cx, first, second, third).await),
      cmd::Common::Goto(first, second, third) => Some(goto(cx, first, second, third).await),
      cmd::Common::SendMessage(caterer_id) => {
         // Отправляем приглашение ввести строку со слешем в меню для отмены
         let res = cx.answer(format!("Введите сообщение (/ для отмены)"))
         .reply_markup(cmd::Caterer::slash_markup())
         .disable_notification(true)
         .send()
         .await;

         if let Ok(_) = res {
            // Код едока
            let user_id = cx.update.from().unwrap().id;

            // Переходим в режим ввода
            Some(next(cmd::Dialogue::MessageToCaterer(user_id, caterer_id, origin)))
         } else {None}
      },
      cmd::Common::UnknownCommand => {
         // Попробуем поискать блюда по заданной строке
         match db::dish_list(db::DishesBy::Find(format!("%{}%", command))).await {
            None => {
               None
            }
            Some(dishes) => {
               // Сформируем строку вида "название /ссылка\n"
               let dishes_desc = dishes.into_iter().map(|dish| (format!("{} /goto{}\n", dish.title_with_price(), db::make_key_3_int(dish.rest_num, dish.group_num, dish.num)))).collect::<String>();

               // Отправляем пользователю результат
               let res = cx.answer(dishes_desc)
               .reply_markup(origin.m)
               .disable_notification(true)
               .send()
               .await;

               if let Ok(_) = res {
                  // Остаёмся в исходном режиме
                  Some(next(origin.d))
               } else {None}
            }
         }
      },
   }
}

// Изменение категории группы rest_id, group_id
pub async fn edit_cat_image(cx: cmd::Cx<String>) -> cmd::Res {
   if let Some(text) = cx.update.text() {
       // Попытаемся преобразовать ответ пользователя в код категории
       let cat_id = db::category_to_id(text);

       // Если категория не пустая, продолжим
       if cat_id > 0 {
           // Извлечём параметры
           let image_id = cx.dialogue;
       
           // Сохраним новое значение в БД
           db::save_cat_image(cat_id, image_id).await;

         // Покажем группу
         return eat_rest::next_with_info(DialogueDispatcherHandlerCx::new(cx.bot, cx.update, cat_id)).await;

       } else {
           // Сообщим об отмене
           cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), "Неизвестная категория, отмена изменения", cmd::User::main_menu_markup()).await
       }
   } else {
     // Сообщим об отмене
     cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), "Отмена изменения", cmd::User::main_menu_markup()).await
   }

   // Остаёмся в пользовательском режиме.
   next(cmd::Dialogue::UserMode)
}

