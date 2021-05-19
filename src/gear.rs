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
   Exit, // return to start menu
   Unknown,
}

impl From<&str> for Command {
   fn from(s: &str) -> Command {
      match s {
         "Добавить" => Command::Add,
         "В начало" => Command::Exit,
         _ => Command::Unknown,
      }
   }
}

impl From<Command> for String {
   fn from(c: Command) -> String {
      match c {
         Command::Add => String::from("Добавить"),
         Command::Exit => String::from("В начало"),
         Command::Unknown => String::from("Неизвестная команда"),
      }
   }
}


pub struct GearState {
   pub state: CommandState,
   node: Node, // current displaying node
}

fn map_req_err(s: String) -> RequestError {
   RequestError::ApiError{
      kind: ApiError::Unknown(s), 
      status_code: StatusCode::OK,
   }
}


#[teloxide(subtransition)]
async fn update(state: GearState, cx: TransitionIn<AutoSend<Bot>>, ans: String,) -> TransitionOut<Dialogue> {
   
   // Parse and handle commands
   match Command::from(ans.as_str()) {
      Command::Add => {
         // Store a new child node to database
         let node = Node::new(state.node.id);
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

   // Define start node
   let node = if state.is_admin {
      // Create root node
      let node = Node::new(0);

      // Load children
      db::node(db::LoadNode::Children(node))
   } else {
      db::node(db::LoadNode::Owner(state.user_id))
   };
   let node = node.await
   .map_err(|s| map_req_err(s))?;

   // Display
   let state = GearState { state, node };
   view(state, cx).await
}

pub async fn view(state: GearState, cx: TransitionIn<AutoSend<Bot>>,) -> TransitionOut<Dialogue> {

   let info = String::from("Записи:");
   let info = state.node.children.iter()
   .fold(info, |acc, n| format!("{}\n/ent{} {}", acc, n.id, n.title));

   let commands = vec![
      String::from(Command::Add),
      String::from(Command::Exit),
   ];
   let markup = kb_markup(vec![commands]);

   cx.answer(info)
   .reply_markup(markup)
   .await?;

   next(state)
}

