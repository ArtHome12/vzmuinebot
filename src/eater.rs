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

use chrono::{Utc};

use crate::commands as cmd;
use crate::database as db;
use crate::caterer;
use crate::eat_rest;
use crate::eat_rest_now;
use crate::basket;

pub async fn start(cx: cmd::Cx<()>, after_restart: bool) -> cmd::Res {
   
   // Различаем перезапуск и возврат из меню ресторатора
   let s = if after_restart {
      // Это первый вход пользователя после перезапуска, сообщим об этом
      let text = format!("{} начал сеанс", db::user_info(cx.update.from(), true));
      db::log(&text).await;

      // Для администратора отдельное приветствие
      if db::is_admin(cx.update.from()) {
         String::from("Начат новый сеанс. Список команд администратора в описании: https://github.com/ArtHome12/vzmuinebot")
      } else {
         String::from("Начат новый сеанс. Пожалуйста, выберите в основном меню снизу какие заведения показать.")
      }
   } else {
      String::from("Пожалуйста, выберите в основном меню снизу какие заведения показать.")
   };
   
   // Запросим настройку пользователя с режимом интерфейса и обновим время последнего входа в БД
   let our_timezone = db::TIME_ZONE.get().unwrap();
   let now = Utc::now().with_timezone(our_timezone).naive_local();
   let compact_mode = db::user_compact_interface(cx.update.from(), now).await;

   // Отображаем приветственное сообщение и меню с кнопками.
   cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), &format!("{}\nРежим интерфейса: {} /toggle", s, db::interface_mode(compact_mode)), cmd::User::main_menu_markup()).await;
    
    // Переходим в режим получения выбранного пункта в главном меню.
    next(cmd::Dialogue::UserMode(compact_mode))
}

pub async fn handle_commands(cx: cmd::Cx<bool>) -> cmd::Res {
   // Режим интерфейса
   let mut compact_mode = cx.dialogue;

   // Разбираем команду.
   match cx.update.text() {
      None => {
         let s = match cx.update.photo() {
            Some(photo_size) => format!("Вы прислали картинку с id\n{}", &photo_size[0].file_id),
            None => String::from("Текстовое сообщение, пожалуйста!"),
         };
         cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), &s, cmd::User::main_menu_markup()).await
      }
      Some(command) => {
         match cmd::User::from(command) {
            cmd::User::Category(cat_id) => {
               // Отобразим все рестораны, у которых есть в меню выбранная категория и переходим в режим выбора ресторана
               return eat_rest::next_with_info(DialogueDispatcherHandlerCx::new(cx.bot, cx.update, (compact_mode, cat_id))).await;
            }
            cmd::User::OpenedNow => {
               // Отобразим рестораны, открытые сейчас и перейдём в режим их выбора
               return eat_rest_now::next_with_info(cx).await;
            }
            cmd::User::UnknownCommand => {
               // Возможно это общая команда
               match cmd::Common::from(command) {
                  cmd::Common::Start => {
                     let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                     return start(DialogueDispatcherHandlerCx::new(bot, update, ()), false).await;
                  }
                  cmd::Common::SendMessage(caterer_id) => {
                     // Отправляем приглашение ввести строку со слешем в меню для отмены
                     cx.answer(format!("Введите сообщение (/ для отмены)"))
                     .reply_markup(cmd::Caterer::slash_markup())
                     .disable_notification(true)
                     .send()
                     .await?;

                     // Код едока
                     let user_id = cx.update.from().unwrap().id;

                     // Переходим в режим ввода
                     return next(cmd::Dialogue::MessageToCaterer(user_id, caterer_id, Box::new(cmd::Dialogue::UserMode(compact_mode)), Box::new(cmd::User::main_menu_markup())));
                  }
                  cmd::Common::UnknownCommand => {
                     let s = &format!("Вы в главном меню: неизвестная команда {}", command);
                     cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), s, cmd::User::main_menu_markup()).await
                  }
               }
            }
            cmd::User::ToggleInterface => {
               // Переключим настройку интерфейса
               db::user_toggle_interface(cx.update.from()).await;
               compact_mode = !compact_mode;
               let s = db::interface_mode(compact_mode);
               let s = &format!("Режим интерфейса изменён на '{}' (пояснение - режим с кнопками может быть удобнее, а со ссылками экономнее к трафику)", s);
               cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), s, cmd::User::main_menu_markup()).await
            }
            cmd::User::CatererMode => {
               // Ссылка на пользователя
               let user = cx.update.from();

               // Если это администратор, то выводим для него команды sudo
               if db::is_admin(user) {
                  // Получим список ресторанов с командой входа
                  let sudo_list = db::restaurant_list_sudo().await;

                  // Отправим информацию
                  cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), &format!("Выберите ресторан для входа\n{}", sudo_list), cmd::User::main_menu_markup()).await;
               } else {
                  // По коду пользователя получим код ресторана
                  match db::rest_num(user).await {
                     Ok(rest_num) => {
                        let text = format!("{} вошёл в режим ресторатора для {}", db::user_info(user, false), rest_num);
                        db::log(&text).await;
   
                        // Отображаем информацию о ресторане и переходим в режим её редактирования
                        let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                        return caterer::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, rest_num), true).await;
                     }
                     _ => {
                        // Сообщим, что доступ запрещён
                        let text = format!("{} доступ в режим ресторатора запрещён", db::user_info(user, false));
                        db::log(&text).await;

                        // Попытаемся получить id пользователя и сообщить ему.
                        let user_id = match user {
                           Some(u) => u.id.to_string(),
                           _ => String::from("ошибка id"),
                        };

                        let s = &format!("Для доступа в режим рестораторов обратитесь к {} и сообщите свой Id={}", db::CONTACT_INFO.get().unwrap(), user_id);
                        let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                        cmd::send_text(&DialogueDispatcherHandlerCx::new(bot, update, ()), s, cmd::User::main_menu_markup()).await
                     }
                  }
               }
            }
            cmd::User::Basket => {
               // Код едока
               let user_id = cx.update.from().unwrap().id;
               
               // Переходим в корзину
               return basket::next_with_info(DialogueDispatcherHandlerCx::new(cx.bot, cx.update, user_id)).await;
            }
            cmd::User::RegisterCaterer(user_id) => {
               // Проверим права
               if db::is_admin(cx.update.from()) {
                  let res = db::is_success(db::register_caterer(user_id).await);
                  cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), &format!("Регистрация или разблокировка ресторатора {}: {}", user_id, res), cmd::User::main_menu_markup()).await;
               } else {
                  cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), "Недостаточно прав", cmd::User::main_menu_markup()).await;
               }
            }
            cmd::User::HoldCaterer(user_id) => {
               // Проверим права
               if db::is_admin(cx.update.from()) {
                  let res = db::is_success(db::hold_caterer(user_id).await.is_ok());
                  cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), &format!("Блокировка ресторатора {}: {}", user_id, res), cmd::User::main_menu_markup()).await;
               } else {
                  cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), "Недостаточно прав", cmd::User::main_menu_markup()).await;
               }
            }
            cmd::User::Sudo(rest_num) => {
               // Проверим права
               if db::is_admin(cx.update.from()) {
                  return caterer::next_with_info(DialogueDispatcherHandlerCx::new(cx.bot, cx.update, rest_num), true).await;
               } else {
                  cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), "Недостаточно прав", cmd::User::main_menu_markup()).await;
               }
            }
            cmd::User::List => {
               // Проверим права
               if db::is_admin(cx.update.from()) {
                  // Получим из БД список ресторанов и отправим его
                  let res = db::restaurant_list().await;
                  cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), &res, cmd::User::main_menu_markup()).await;
               } else {
                  cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), "Недостаточно прав", cmd::User::main_menu_markup()).await;
               }
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
   next(cmd::Dialogue::UserMode(compact_mode))
}

