/* ===============================================================================
Restaurant menu bot.
General commands. 10 Jun 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide_macros::teloxide;
use teloxide::{prelude::*, types::ParseMode, };
use strum::{AsRefStr, EnumString,};
use std::str::FromStr;

use crate::states::*;
use crate::search;

#[derive(AsRefStr, EnumString)]
pub enum Command {
   #[strum(to_string = "/start")]
   Start,
   #[strum(to_string = "/msg")]
   Message(i64),
   #[strum(to_string = "/get")]
   Goto(i32),
   Unknown,
}

impl Command {
   fn parse(s: &str) -> Self {
      // Try as main command
      Self::from_str(s)
      .unwrap_or_else(|_| {
         // Looking for the commands with arguments
         let l_part = s.get(..4).unwrap_or_default();
         let r_part = s.get(4..).unwrap_or_default();

         if l_part == Self::Message(0).as_ref() {
            Command::Message(r_part.parse().unwrap_or_default())
         } else if l_part == Self::Goto(0).as_ref() {
               Command::Goto(r_part.parse().unwrap_or_default())
         } else {
            Command::Unknown
         }
      })
   }
}

pub struct MessageState {
   pub state: CommandState,
   pub receiver: i64,
}



pub async fn update(state: CommandState, cx: TransitionIn<AutoSend<Bot>>, ans: String,) -> TransitionOut<Dialogue> {
   // Parse and handle commands
   let cmd = Command::parse(ans.as_str());
   match cmd {
      Command::Start => {
         let text = "Добро пожаловать. Пожалуйста, нажмите на 'Все' для отображения полного списка, 'Открыто' для работающих сейчас (если панель с кнопками скрыта, раскройте её), либо отправьте текст для поиска.";
         cx.answer(text)
         .reply_markup(main_menu_markup())
         .await?;
      }
      Command::Message(receiver) => return enter_input(MessageState {state, receiver }, cx).await,
      Command::Goto(node_id) => {
         let text = "Переход в разработке";
         cx.answer(text)
         .reply_markup(main_menu_markup())
         .await?;
      }
      Command::Unknown => {
         let search_result = search::search(&ans).await
         .map_err(|s| map_req_err(s))?;

         // Add hint if results too short
         let hint = if search_result.len() < 30 { " <i>Подсказка - используйте подстановочные символы, например '%блок%' позволит найти 'запечённые яблоки'</i>" } else { "" };

         let text = format!("Результаты поиска по {}.{}\n{}", ans, hint, search_result);
         cx.reply_to(text)
         .reply_markup(main_menu_markup())
         .parse_mode(ParseMode::Html)
         .await?;
      },
   }
   next(state)
}

async fn enter_input(state: MessageState, cx: TransitionIn<AutoSend<Bot>>) -> TransitionOut<Dialogue> {
   let text = "Введите сообщение для отправки (/ для отмены)";
   cx.answer(text)
   .reply_markup(cancel_markup())
   .await?;

   next(state)
}

#[teloxide(subtransition)]
async fn update_input(state: MessageState, cx: TransitionIn<AutoSend<Bot>>, ans: String) -> TransitionOut<Dialogue> {
   let info = if ans == String::from("/") {
      "Отмена, сообщение не отправлено"
   } else {
      // Forward message to receiver
      let msg_id = cx.update.id;
      let msg = cx.requester.forward_message(state.receiver, cx.update.chat.id, msg_id).await?;

      // Add info with qoute
      let text = format!("Ответить {}{}", Command::Message(0).as_ref(), state.state.user_id);
      cx.requester.send_message(state.receiver, &text)
      .reply_to_message_id(msg.id)
      .await?;

      "Cообщение отправлено"
   };

   // Report result and return to main menu
   cx.answer(info)
   .reply_markup(main_menu_markup())
   .await?;
   
   next(state.state)
}
