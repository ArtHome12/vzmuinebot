/* ===============================================================================
Restaurant menu bot.
Settings menu. 16 May 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide_macros::{teloxide, };
use teloxide::{payloads::SendMessageSetters, prelude::*, };

use crate::states::*;

pub struct GearState {
   pub state: CommandState,
}

#[teloxide(subtransition)]
async fn settings(state: GearState, cx: TransitionIn<AutoSend<Bot>>, ans: String,) -> TransitionOut<Dialogue> {
   /* let info = if ans == "/" {
      String::from("Настройки не изменёны")
   } else {
      // Save to database
      // db::update_user_descr(state.state.user_id, &ans).await;

      format!("Ваши новые настройки {} сохранены", ans)
   };

   cx.answer(info)
   .reply_markup(one_button_markup("В начало"))
   .await?;

   next(StartState { restarted: false }) */

   // Return to waiting for commands of the main menu
   crate::states::enter(StartState { restarted: false }, cx, ans).await
}

pub async fn enter(state: CommandState, cx: TransitionIn<AutoSend<Bot>>,) -> TransitionOut<Dialogue> {
   let info = if state.is_admin {
      "Записи:\n/Add Добавить"
   } else {
      "Нет доступных настроек"
   };

   cx.answer(info)
   .reply_markup(one_button_markup("В начало"))
   .await?;

   next(GearState { state })
}