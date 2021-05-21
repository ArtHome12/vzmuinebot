/* ===============================================================================
Restaurant menu bot.
Settings menu. 16 May 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide_macros::teloxide;
use teloxide::{prelude::*, ApiError, RequestError, payloads::SendMessageSetters,};
use reqwest::StatusCode;
use std::str::FromStr;
use strum::{AsRefStr, EnumString,};

use crate::states::*;
use crate::database as db;
use crate::node::Node;

#[derive(AsRefStr, EnumString)]
enum Command {
   #[strum(to_string = "Добавить")]
   Add, // add a new node
   #[strum(to_string = "Удалить")]
   Delete, // delete node
   #[strum(to_string = "Выход")]
   Exit, // return to start menu
   #[strum(to_string = "Назад")]
   Return, // return to parent node
   #[strum(to_string = "/pas")]
   Pass(i32), // make the specified node active
   #[strum(to_string = "Название")]
   Title,
   #[strum(to_string = "Описание")]
   Descr,
   #[strum(to_string = "Картинка")]
   Picture,
   #[strum(to_string = "Доступ")]
   Enable,
   #[strum(to_string = "Бан")]
   Ban,
   #[strum(to_string = "Управ1")]
   Owner1,
   #[strum(to_string = "Управ2")]
   Owner2,
   #[strum(to_string = "Управ3")]
   Owner3,
   #[strum(to_string = "Открытие")]
   Open,
   #[strum(to_string = "Закрытие")]
   Close,
   #[strum(to_string = "Цена")]
   Price,
   Unknown,
}

impl Command {
   fn parse(s: &str) -> Self {
      Self::from_str(s)
      .unwrap_or_else(|_| {
         // Looking for the commands with arguments
         if s.get(..4).unwrap_or_default() == Self::Pass(0).as_ref() {
            let r_part = s.get(4..).unwrap_or_default();
            Command::Pass(r_part.parse().unwrap_or_default())
         } else {
            Command::Unknown
         }
      })
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
   let cmd = Command::parse(ans.as_str());
   match cmd {
      Command::Add => {
         // Store a new child node to database
         let node = Node::new(state.node.id);
         db::insert_node(&node)
         .await
         .map_err(|s| map_req_err(s))?;

         // Reload current node
         view(state, cx).await
      }

      Command::Exit => crate::states::enter(StartState { restarted: false }, cx, ans).await,

      Command::Pass(index) => {
         // Get database id for child index
         let id = state.node.children.get(index as usize);

         // Set new node and reload or report error
         if id.is_some() {
            cx.answer(format!("Pas index={}, id={}", index, id.unwrap().id)).await?;
            let state = GearState {
               state: state.state, 
               node: id.unwrap().clone()
            };

            view(state, cx).await
         } else {
            cx.answer(format!("Неверно указан номер записи '{}', нельзя перейти", index)).await?;

            // Stay in place
            next(state)
         }
      }

      Command::Title => {
         cx.answer(format!("Текущее название '{}', введите новое или / для отмены", state.node.title))
         .reply_markup(cancel_markup())
         .await?;

         // Stay in place
         next(state)
      }

      Command::Unknown => {
         cx.answer(format!("Неизвестная команда '{}', вы находитесь в меню настроек", ans)).await?;

         // Stay in place
         next(state)
      }

      _ => {
         cx.answer(format!("Команда '{}' ещё не реализована", ans)).await?;
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
   .enumerate()
   .fold(info, |acc, n| format!("{}\n{}{} {}", acc, Command::Pass(0).as_ref(), n.0, n.1.title));

   let mut row1 = vec![
      String::from(Command::Add.as_ref()),
      String::from(Command::Title.as_ref()),
      String::from(Command::Descr.as_ref()),
   ];
   let row2 = vec![
      String::from(Command::Enable.as_ref()),
      String::from(Command::Open.as_ref()),
      String::from(Command::Close.as_ref()),
      String::from(Command::Picture.as_ref()),
   ];
   let mut row3 = vec![
      String::from(Command::Exit.as_ref()),
   ];

   // Condition-dependent menu items
   if state.state.is_admin {
      row3.insert(0, String::from(Command::Ban.as_ref()))
   }
   if state.node.id != 0 {
      row1.insert(1, String::from(Command::Delete.as_ref()));
      row3.push(String::from(Command::Return.as_ref()))
   }

   let mut keyboard = vec![row1, row2, row3];

   if state.state.is_admin {
      let row_admin = vec![
         String::from(Command::Price.as_ref()),
         String::from(Command::Owner1.as_ref()),
         String::from(Command::Owner2.as_ref()),
         String::from(Command::Owner3.as_ref()),
      ];
      keyboard.insert(2, row_admin);
   }

   let markup = kb_markup(keyboard);

   cx.answer(info)
   .reply_markup(markup)
   .await?;

   next(state)
}

