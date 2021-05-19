/* ===============================================================================
Restaurant menu bot.
Settings menu. 16 May 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide_macros::teloxide;
use teloxide::{ApiError, RequestError, payloads::SendMessageSetters, prelude::*};
use reqwest::StatusCode;

use crate::states::*;
use crate::database as db;
use crate::node::Node;

enum Command {
   Add, // add a new node
   Exit, // return to start menu
   Return, // return to parent node
   Pass(i32), // make the specified node active
   Title,
   Descr,
   Picture,
   Enable,
   Ban,
   Owner1,
   Owner2,
   Owner3,
   Open,
   Close,
   Price,
   Unknown,
}

impl From<&str> for Command {
   fn from(s: &str) -> Command {
      match s {
         "Добавить" => Command::Add,
         "Выход" => Command::Exit,
         "Назад" => Command::Return,
         "Название" => Command::Title,
         "Описание" => Command::Descr,
         "Картинка" => Command::Picture,
         "Доступность" => Command::Enable,
         "Бан" => Command::Ban,
         "Управ1" => Command::Owner1,
         "Управ2" => Command::Owner2,
         "Управ3" => Command::Owner3,
         "Открытие" => Command::Open,
         "Закрытие" => Command::Close,
         "Цена" => Command::Price,
         _ => {
            // Looking for the commands with arguments
            if s.get(..4).unwrap_or_default() == "/pas" {
               let r_part = s.get(4..).unwrap_or_default();
               Command::Pass(r_part.parse().unwrap_or_default())
            } else {
               Command::Unknown
            }
         }
      }
   }
}

impl From<Command> for String {
   fn from(c: Command) -> String {
      match c {
         Command::Add => String::from("Добавить"),
         Command::Exit => String::from("Выход"),
         Command::Return => String::from("Назад"),
         Command::Pass(index) => format!("/pas{}", index),
         Command::Title => String::from("Название"),
         Command::Descr => String::from("Описание"),
         Command::Picture => String::from("Картинка"),
         Command::Enable => String::from("Доступность"),
         Command::Ban => String::from("Бан"),
         Command::Owner1 => String::from("Управ1"),
         Command::Owner2 => String::from("Управ2"),
         Command::Owner3 => String::from("Управ3"),
         Command::Open => String::from("Открытие"),
         Command::Close => String::from("Закрытие"),
         Command::Price => String::from("Цена"),
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
         view(state, cx).await
      }

      Command::Exit => crate::states::enter(StartState { restarted: false }, cx, ans).await,

      Command::Pass(index) => {
         // Get database id for child index
         let id = state.node.children.get(index as usize);

         // Set new node and reload or report error
         if id.is_some() {
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
   .fold(info, |acc, n| format!("{}\n{} {}", acc, String::from(Command::Pass(n.id)), n.title));

   let row1 = vec![
      String::from(Command::Add),
      String::from(Command::Title),
      String::from(Command::Descr),
      String::from(Command::Picture),
   ];
   let row2 = vec![
      String::from(Command::Enable),
      String::from(Command::Open),
      String::from(Command::Close),
      String::from(Command::Picture),
   ];
   let mut row3 = vec![
      String::from(Command::Exit),
   ];

   // Condition-dependent menu items
   if state.state.is_admin {
      row3.insert(0, String::from(Command::Ban))
   }
   if state.node.id != 0 {
      row3.push(String::from(Command::Return))
   }

   let mut keyboard = vec![row1, row2, row3];

   if state.state.is_admin {
      let row_admin = vec![
         String::from(Command::Price),
         String::from(Command::Owner1),
         String::from(Command::Owner2),
         String::from(Command::Owner3),
      ];
      keyboard.insert(2, row_admin);
   }

   let markup = kb_markup(keyboard);

   cx.answer(info)
   .reply_markup(markup)
   .await?;

   next(state)
}

