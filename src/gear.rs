/* ===============================================================================
Restaurant menu bot.
Settings menu. 16 May 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide_macros::teloxide;
use teloxide::{prelude::*, RequestError, ApiError, };
use reqwest::StatusCode;

use crate::states::*;
use crate::database as db;
use crate::node::Node;

enum Command {
   Add, // add a new node
   Exit,  // return to start menu
   Unknown,
}

impl From<&str> for Command {
   fn from(s: &str) -> Command {
      match s {
         "/Add" => Command::Add,
         "В начало" => Command::Exit,
         _ => Command::Unknown,
      }
   }
}

impl From<Command> for String {
   fn from(c: Command) -> String {
      match c {
         Command::Add => String::from("/Add"),
         Command::Exit => String::from("В начало"),
         Command::Unknown => String::from("Неизвестная команда"),
      }
   }
}


pub struct GearState {
   pub state: CommandState,
   node: Option<Node>, // current displaying node
}

fn map_req_err(s: String) -> RequestError {
   RequestError::ApiError{
      kind: ApiError::Unknown(s), 
      status_code: StatusCode::OK,
   }
}


#[teloxide(subtransition)]
async fn gear(state: GearState, cx: TransitionIn<AutoSend<Bot>>, ans: String,) -> TransitionOut<Dialogue> {
   
   // Parse and handle commands
   match Command::from(ans.as_str()) {
      Command::Add => {
         // Store a new child node to database
         let parent_id = state.node.map_or(0, |n| n.id);
         let node = Node::new(parent_id);
         db::insert_node(&node)
         .await
         .map_err(|s| map_req_err(s))?;

         // Reload current node
         enter(state.state, cx).await
      }

      Command::Exit => crate::states::enter(StartState { restarted: false }, cx, ans).await,

      Command::Unknown => {
         cx.answer(format!("Неизвестная команда '{}', вы находитесь в меню настроек", ans)).await?;

         // Stay in place
         next(state)
      }
   }
}

pub async fn enter(state: CommandState, cx: TransitionIn<AutoSend<Bot>>,) -> TransitionOut<Dialogue> {

   let (node, info) = if state.is_admin {
      // Create root node
      let node = Node::new(0);

      // Load children
      let node = db::node(db::LoadNode::Children(node));

      (node, format!("Записи:\n{} Добавить", String::from(Command::Add)))
   } else {
      (db::node(db::LoadNode::Owner(state.user_id)), String::from("Нет доступных настроек"))
   };
   let node = node.await
   .map_err(|s| map_req_err(s))?;

   cx.answer(info)
   .reply_markup(one_button_markup(String::from(Command::Exit)))
   .await?;

   next(GearState { state, node: Some(node) })
}