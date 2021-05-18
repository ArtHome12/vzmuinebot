/* ===============================================================================
Restaurant menu bot.
Settings menu. 16 May 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide_macros::teloxide;
use teloxide::prelude::*;

use crate::states::*;
use crate::database as db;
use crate::node::Node;

pub struct GearState {
   pub state: CommandState,
}

#[teloxide(subtransition)]
async fn settings(state: GearState, cx: TransitionIn<AutoSend<Bot>>, ans: String,) -> TransitionOut<Dialogue> {
   match ans.as_str() {
      "/Add" => {
         // Add a new child node

         // Reload node
         return enter(state.state, cx).await;
      }
      "В начало" => {
         // Return to waiting for commands of the main menu
         return crate::states::enter(StartState { restarted: false }, cx, ans).await;
      }
      _ => {
         cx.answer("Неизвестная команда, вы находитесь в меню настроек")
         .await?;
      }
   };

   // Stay in place
   next(state)
}

pub async fn enter(state: CommandState, cx: TransitionIn<AutoSend<Bot>>,) -> TransitionOut<Dialogue> {

   let (node, info) = if state.is_admin {
      // Create root node
      let node = Node::default();

      // Load children
      let node = db::node(db::LoadNode::Children(node));

      (node, "Записи:\n/Add Добавить")
   } else {
      (db::node(db::LoadNode::Owner(state.user_id)), "Нет доступных настроек")
   };

   cx.answer(info)
   .reply_markup(one_button_markup("В начало"))
   .await?;

   next(GearState { state })
}