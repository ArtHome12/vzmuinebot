/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Меню настроек и входа в режим ресторатора. 19 Jule 2020.
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
use crate::settings;
use crate::language as lang;
use crate::caterer;


// Показывает приветствие
pub async fn next_with_info(cx: cmd::Cx<bool>) -> cmd::Res {
   // Режим интерфейса
   let compact_mode = cx.dialogue;

   // Отображаем приветствие
   let s = format!("Режим интерфейса: {} /toggle", db::interface_mode(compact_mode));
   cx.answer(s)
   .reply_markup(cmd::Gear::bottom_markup())
   .disable_notification(true)
   .send()
   .await?;

   // Остаёмся в этом режиме.
   next(cmd::Dialogue::GearMode(compact_mode))
}

pub async fn next_with_cancel(cx: cmd::Cx<bool>, text: &str) -> cmd::Res {
   // Режим интерфейса
   let compact_mode = cx.dialogue;

   // Отображаем сообщение
   let s = format!("{}\n\nРежим интерфейса: {} /toggle", text, db::interface_mode(compact_mode));
   cx.answer(s)
   .reply_markup(cmd::Gear::bottom_markup())
   .disable_notification(true)
   .send()
   .await?;

   // Остаёмся в этом режиме.
   next(cmd::Dialogue::GearMode(compact_mode))
}

pub async fn handle_commands(cx: cmd::Cx<bool>) -> cmd::Res {
   // Режим интерфейса
   let mut compact_mode = cx.dialogue;

   // Разбираем команду.
   match cx.update.text() {
      None => {
         let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
         next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, compact_mode), "Текстовое сообщение, пожалуйста!").await
      }
      Some(command) => {
         match cmd::Gear::from(command) {
            // В главное меню
            cmd::Gear::Main => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               eater::start(DialogueDispatcherHandlerCx::new(bot, update, ()), false).await
            }

            cmd::Gear::UnknownCommand => {
               // Возможно это общая команда
               match cmd::Common::from(command) {
                  cmd::Common::Start => {
                     let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                     eater::start(DialogueDispatcherHandlerCx::new(bot, update, ()), false).await
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
                     let s = &format!("Вы в меню ⚙ настроек: неизвестная команда '{}'", command);
                     next_with_cancel(DialogueDispatcherHandlerCx::new(cx.bot, cx.update, compact_mode), s).await
                  }
               }
            }
            cmd::Gear::ToggleInterface => {
               // Переключим настройку интерфейса
               db::user_toggle_interface(cx.update.from()).await;
               compact_mode = !compact_mode;
               let s = db::interface_mode(compact_mode);
               let s = &format!("Режим интерфейса изменён на '{}' (пояснение - режим с кнопками может быть удобнее, а со ссылками экономнее к трафику)", s);
               next_with_cancel(DialogueDispatcherHandlerCx::new(cx.bot, cx.update, compact_mode), s).await
            }
            cmd::Gear::CatererMode => {
               // Ссылка на пользователя
               let user = cx.update.from();

               // Если это администратор, то выводим для него команды sudo
               if settings::is_admin(user) {
                  // Получим из БД список ресторанов и отправим его
                  match db::rest_list(db::RestListBy::All).await {
                     Some(rest_list) => {
                        // Сформируем строку вида: 1371303352 'Ресторан "два супа"' /sudo1
                        let s: String = rest_list.into_iter().map(|r| (format!("{} '{}' /sudo{}\n", r.user_id, r.title, r.num))).collect();

                        // Отправим информацию
                        cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), &format!("Выберите ресторан для входа\n{}", s), cmd::Gear::bottom_markup()).await;
                        next(cmd::Dialogue::GearMode(compact_mode))
                     }
                     None => {
                        // Если там пусто, то сообщим об этом
                        let s = String::from(lang::t("ru", lang::Res::EatRestEmpty));
                        next_with_cancel(DialogueDispatcherHandlerCx::new(cx.bot, cx.update, compact_mode), &s).await
                     }
                  }

               } else {
                  // По коду пользователя получим код ресторана
                  match db::rest_num(user).await {
                     Ok(rest_num) => {
                        let text = format!("{} вошёл в режим ресторатора для {}", db::user_info(user, false), rest_num);
                        settings::log(&text).await;
   
                        // Отображаем информацию о ресторане и переходим в режим её редактирования
                        let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                        return caterer::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, rest_num), true).await;
                     }
                     _ => {
                        // Сообщим, что доступ запрещён
                        let text = format!("{} доступ в режим ресторатора запрещён", db::user_info(user, false));
                        settings::log(&text).await;

                        // Попытаемся получить id пользователя и сообщить ему.
                        let user_id = match user {
                           Some(u) => u.id.to_string(),
                           _ => String::from("ошибка id"),
                        };

                        let s = &format!("Для доступа в режим рестораторов обратитесь к {} и сообщите свой Id={}", settings::admin_contact_info(), user_id);
                        let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                        next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, compact_mode), &s).await
                     }
                  }
               }
            }
            cmd::Gear::RegisterCaterer(user_id) => {
               // Проверим права
               let s = if settings::is_admin(cx.update.from()) {
                  let res = db::is_success(db::register_caterer(user_id).await);
                  format!("Регистрация или разблокировка ресторатора {}: {}", user_id, res)
               } else {
                  String::from("Недостаточно прав")
               };

               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, compact_mode), &s).await
         }
            cmd::Gear::HoldCaterer(user_id) => {
               let s = if settings::is_admin(cx.update.from()) {
                  let res = db::is_success(db::hold_caterer(user_id).await.is_ok());
                  format!("Блокировка ресторатора {}: {}", user_id, res)
               } else {
                  String::from("Недостаточно прав")
               };

               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, compact_mode), &s).await
            }
            cmd::Gear::Sudo(rest_num) => {
               // Проверим права
               if settings::is_admin(cx.update.from()) {
                  return caterer::next_with_info(DialogueDispatcherHandlerCx::new(cx.bot, cx.update, rest_num), true).await;
               } else {
                  let s = "Недостаточно прав";
                  let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                  next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, compact_mode), &s).await
               }
            }
            cmd::Gear::List => {
               // Проверим права
               if settings::is_admin(cx.update.from()) {
                  // Получим из БД список ресторанов и отправим его
                  match db::rest_list(db::RestListBy::All).await {
                     Some(rest_list) => {
                        // Сформируем строку вида: 1 'Ресторан "два супа"', доступен /hold1371303352
                        let s: String = rest_list.into_iter().map(|r| (format!("{} '{}', {} {}{}\n", 
                        r.num, r.title, db::enabled_to_str(r.enabled), db::enabled_to_cmd(r.enabled), r.user_id
                        ))).collect();
                        cmd::send_text(&DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ()), &s, cmd::User::main_menu_markup()).await;
                        next(cmd::Dialogue::GearMode(compact_mode))
                     }
                     None => {
                        // Если там пусто, то сообщим об этом
                        let s = String::from(lang::t("ru", lang::Res::EatRestEmpty));
                        next_with_cancel(DialogueDispatcherHandlerCx::new(cx.bot, cx.update, compact_mode), &s).await
                     }
                  }
               } else {
                  let s = "Недостаточно прав";
                  let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                  next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, compact_mode), &s).await
               }
            }
         }
      }
   }
}

