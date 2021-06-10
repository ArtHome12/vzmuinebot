/* ===============================================================================
Restaurant menu bot.
General commands. 10 Jun 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{payloads::SendMessageSetters, prelude::*};
use strum::{AsRefStr, EnumString,};
use std::str::FromStr;

use crate::states::{CommandState, Dialogue, main_menu_markup};

#[derive(AsRefStr, EnumString)]
enum Command {
   #[strum(to_string = "/start")]
   Start,
   Unknown,
}

pub async fn update(state: CommandState, cx: TransitionIn<AutoSend<Bot>>, ans: String,) -> TransitionOut<Dialogue> {
   // Parse and handle commands
   let cmd = Command::from_str(ans.as_str()).unwrap_or(Command::Unknown);
   match cmd {
      Command::Start => {
         let text = "Добро пожаловать! Пожалуйста, выберите одну из команд внизу (если панель с кнопками скрыта, откройте её)";
         cx.answer(text)
         .reply_markup(main_menu_markup())
         .await?;
      }
      Command::Unknown => {
         let text = "Поиск в разработке";
         cx.answer(text)
         .reply_markup(main_menu_markup())
         .await?;
      },
   }
   next(state)
}
