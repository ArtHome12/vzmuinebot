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
         String::from("Начат новый сеанс. Пожалуйста, выберите в основном меню снизу какие заведения показать.")
      }
   } else {
      String::from("Пожалуйста, выберите в основном меню снизу какие заведения показать.")
   };
   
   // Запросим настройку пользователя с режимом интерфейса и обновим время последнего входа в БД
   let now = settings::current_date_time();
   let compact_mode = db::user_compact_interface(cx.update.from(), now).await;

   // Если сессия началась с какой-то команды, то попробуем сразу её обработать
   if let Some(input) = cx.update.text() {
      // Пытаемся распознать команду как собственную или глобальную
      let known = cmd::User::from(input) != cmd::User::UnknownCommand || cmd::Common::from(input) != cmd::Common::UnknownCommand;
      if known {
         let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
         return handle_commands(DialogueDispatcherHandlerCx::new(bot, update, compact_mode)).await;
      }
   }

   // Если команды не было или она не распознана, отображаем приветственное сообщение и меню с кнопками.
   cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update.clone(), ()), &s, cmd::User::main_menu_markup()).await;
   
   // Код едока
/*   let user_id = cx.update.from().unwrap().id;

   
   if let Some(input) = cx.update.text() {
      // Команда старт может быть с аргументами
      let l_part = input.get(..6).unwrap_or_default();
      if l_part == "/start" {
         // Поробуем извлечь пробел и аргументы
         let r_part = input.get(7..).unwrap_or_default();
         if let Ok((_first, _second, _third)) = db::parse_key_3_int(r_part) {
               // Перейдём сразу в нужный ресторан
               return next(cmd::Dialogue::BasketMode(user_id));
         }
      };
   }*/

   // Переходим в режим получения выбранного пункта в главном меню
   next(cmd::Dialogue::UserMode(compact_mode))
}

pub async fn handle_commands(cx: cmd::Cx<bool>) -> cmd::Res {
   // Режим интерфейса
   let compact_mode = cx.dialogue;

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
                     // Отображаем приветственное сообщение и меню с кнопками
                     let s = "Пожалуйста, выберите в основном меню снизу какие заведения показать.";
                     cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update.clone(), ()), s, cmd::User::main_menu_markup()).await;
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
            cmd::User::Gear => {
               // Переходим в меню с шестерёнкой
               return gear::next_with_info(DialogueDispatcherHandlerCx::new(cx.bot, cx.update, compact_mode)).await;
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
   next(cmd::Dialogue::UserMode(compact_mode))
}

