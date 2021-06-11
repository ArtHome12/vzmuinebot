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

use crate::states::{CommandState, Dialogue, main_menu_markup, cancel_markup};

#[derive(AsRefStr, EnumString)]
pub enum Command {
   #[strum(to_string = "/start")]
   Start,
   #[strum(to_string = "/msg")]
   Message(i64),
   Unknown,
}

impl Command {
   fn parse(s: &str) -> Self {
      // Try as main command
      Self::from_str(s)
      .unwrap_or_else(|_| {
         // Looking for the commands with arguments
         if s.get(..4).unwrap_or_default() == Self::Message(0).as_ref() {
            let r_part = s.get(4..).unwrap_or_default();
            Command::Message(r_part.parse().unwrap_or_default())
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
         let text = "Добро пожаловать! Пожалуйста, выберите одну из команд внизу (если панель с кнопками скрыта, откройте её)";
         cx.answer(text)
         .reply_markup(main_menu_markup())
         .await?;
      }
      Command::Message(receiver) => return enter_input(MessageState {state, receiver }, cx).await,
      Command::Unknown => {
         let text = "Поиск в разработке";
         cx.answer(text)
         .reply_markup(main_menu_markup())
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
      String::from("Отмена, сообщение не отправлено")
   } else {
      // Collect info about sender and prepare message text
      let user = cx.update.from();
      let sender_name = if let Some(user) = user {
         user.full_name()
      } else {
         String::from("'нет имени'")
      };

      let text = format!("Сообщение от {}, ответить {}{}\n\n", sender_name, Command::Message(0).as_ref(), state.state.user_id);

      // Send message to receiver
      cx.requester.send_message(state.receiver, &text)
      .parse_mode(ParseMode::Html)
      .await?;

      text
   };

   // Report result and return to main menu
   cx.answer(info)
   .reply_markup(main_menu_markup())
   .await?;
   
   next(state.state)
}
