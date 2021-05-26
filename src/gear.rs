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
use strum::{AsRefStr, EnumString, EnumMessage, };
use chrono::{NaiveTime};
use enum_default::EnumDefault;

use crate::states::*;
use crate::database as db;
use crate::node::*;

// ============================================================================
// [Main entry]
// ============================================================================
// Main commands
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
   Edit(EditCmd),
   Unknown,
}

// Main commands
#[derive(AsRefStr, EnumString, EnumMessage, EnumDefault)]
enum EditCmd {
   #[strum(to_string = "Название", message = "title")] // Button caption and db field name
   Title,
   #[strum(to_string = "Описание", message = "descr")]
   Descr,
   #[strum(to_string = "Картинка", message = "picture")]
   Picture,
   #[strum(to_string = "Доступ", message = "enabled")]
   Enable,
   #[strum(to_string = "Бан", message = "banned")]
   Ban,
   #[strum(to_string = "ID 1", message = "owner1")]
   Owner1,
   #[strum(to_string = "ID 2", message = "owner2")]
   Owner2,
   #[strum(to_string = "ID 3", message = "owner3")]
   Owner3,
   #[strum(to_string = "Время", message = "open, close")]
   Time,
   #[strum(to_string = "Цена", message = "price")]
   Price,
}

impl Command {
   fn parse(s: &str) -> Self {
      // Try as edit subcommand
      if let Ok(edit) = EditCmd::from_str(s) {
         Self::Edit(edit)
      } else {
         // Try as main command
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
}

pub struct GearState {
   pub state: CommandState,
   stack: Vec<Node>, // from start to current displaying node
}

fn map_req_err(s: String) -> RequestError {
   RequestError::ApiError{
      kind: ApiError::Unknown(s), 
      status_code: StatusCode::OK,
   }
}

#[teloxide(subtransition)]
async fn update(mut state: GearState, cx: TransitionIn<AutoSend<Bot>>, ans: String) -> TransitionOut<Dialogue> {
   async fn do_return(mut state: GearState, cx: TransitionIn<AutoSend<Bot>>) -> TransitionOut<Dialogue> {
      // Extract current node from stack
      state.stack.pop().unwrap();

      // Go if there are still nodes left
      if !state.stack.is_empty() {
         view(state, cx).await
      } else {
         crate::states::enter(StartState { restarted: false }, cx, String::default()).await
      }
   }
   
   // Parse and handle commands
   let cmd = Command::parse(ans.as_str());
   match cmd {
      Command::Add => {
         // Extract current node from stack
         let mut node = state.stack.pop().unwrap();

         // Store a new child node in database
         let child = Node::new(node.id);
         db::insert_node(&child)
         .await
         .map_err(|s| map_req_err(s))?;

         // Update current node and push to stack
         node.children.push(child);
         state.stack.push(node);

         // Show
         view(state, cx).await
      }

      Command::Exit => crate::states::enter(StartState { restarted: false }, cx, String::default()).await,

      Command::Return => do_return(state, cx).await,

      Command::Pass(index) => {
         // Peek current node from stack
         let node = state.stack.last().unwrap();

         // Get database id for child index, starting from zero
         let child = node.children.get((index - 1) as usize);

         // Set new node or report error
         if child.is_some() {
            // Load children
            let node = child.unwrap().clone(); // Clone child node as an independent element
            let node = db::node(db::LoadNode::Children(node)).await
            .map_err(|s| map_req_err(s))?;

            // Push new node and show
            state.stack.push(node);
            view(state, cx).await
         } else {
            cx.answer(format!("Неверно указан номер записи '{}', нельзя перейти", index)).await?;

            // Stay in place
            next(state)
         }
      }

      Command::Delete => {
         // Root/start node cannot to delete
         if state.stack.len() <= 1 {
            cx.answer("Нельзя удалить начальный узел").await?;

            // Stay in place
            return next(state);
         }

         // Peek current node from stack
         let node = state.stack.last().unwrap();

         // Delete record if it has no children
         let children_num = node.children.len();
         if children_num > 0 {
            cx.answer(format!("У записи '{}' есть {} дочерних, для защиты от случайного удаления большого объёма информации удалите сначала их", node.title, children_num)).await?;

            // Stay in place
            next(state)
         } else {
            db::delete_node(node.id)
            .await
            .map_err(|s| map_req_err(s))?;

            cx.answer(format!("Запись '{}' удалена, переходим на уровень выше", node.title)).await?;
            do_return(state, cx).await
         }
      }

      Command::Edit(cmd) => {
         // Editing node
         let node = state.stack.last().unwrap();

         // Underlying data
         let kind = match cmd {
            EditCmd::Title => UpdateKind::Text(node.title.clone()),
            EditCmd::Descr => UpdateKind::Text(node.descr.clone()),
            EditCmd::Picture => UpdateKind::Picture("".into()),
            EditCmd::Enable => UpdateKind::Flag(node.enabled),
            EditCmd::Ban => UpdateKind::Flag(node.banned),
            EditCmd::Owner1 => UpdateKind::Int(node.owners[0]),
            EditCmd::Owner2 => UpdateKind::Int(node.owners[1]),
            EditCmd::Owner3 => UpdateKind::Int(node.owners[3]),
            EditCmd::Time => UpdateKind::Time(node.time.0, node.time.1),
            EditCmd::Price => UpdateKind::Money(node.price),
         };

         // Appropriate database field name
         let field = String::from(cmd.get_message().unwrap());

         // Move to editing mode
         let state = GearStateEditing {
            state,
            update: UpdateNode { kind, field, }
         };
         enter_edit(state, cx).await
      }

      Command::Unknown => {
         cx.answer(format!("Неизвестная команда '{}', вы находитесь в меню настроек", ans)).await?;

         // Stay in place
         next(state)
      }
   }
}

pub async fn enter(state: CommandState, cx: TransitionIn<AutoSend<Bot>>,) -> TransitionOut<Dialogue> {

   // Define start node
   let mode = if state.is_admin {
      // Root node
      db::LoadNode::Id(0)
   } else {
      // Find node for owner
      db::LoadNode::Owner(state.user_id)
   };

   let node = db::node(mode).await
      .map_err(|s| map_req_err(s))?;

   // Load children
   let node = db::node(db::LoadNode::Children(node)).await
   .map_err(|s| map_req_err(s))?;

   // Display
   let state = GearState { state, stack: vec![node] };
   view(state, cx).await
}

pub async fn view(state: GearState, cx: TransitionIn<AutoSend<Bot>>,) -> TransitionOut<Dialogue> {

   // Collect path from the beginning
   let mut title = state.stack
   .iter()
   .skip(1)
   .fold(String::default(), |acc, n| acc + "/" + &n.title);

   // Add descr if set
   let node = state.stack.last();
   if let Some(node) = node {
      if node.descr.len() >= 3 {
         title = title + "\nОписание: " + node.descr.as_str();
      }
   }

   let info = state.stack
   .last().unwrap()
   .children.iter()
   .enumerate()
   .fold(title, |acc, n| format!("{}\n{}{} {}", acc, Command::Pass(0).as_ref(), n.0 + 1, n.1.title));

   let mut row1 = vec![
      String::from(Command::Add.as_ref()),
      String::from(EditCmd::Title.as_ref()),
      String::from(EditCmd::Descr.as_ref()),
   ];
   let row2 = vec![
      String::from(EditCmd::Enable.as_ref()),
      String::from(EditCmd::Time.as_ref()),
      String::from(EditCmd::Picture.as_ref()),
   ];
   let mut row3 = vec![
      String::from(Command::Exit.as_ref()),
   ];

   // Condition-dependent menu items
   if state.stack.len() > 1 {
      row1.insert(1, String::from(Command::Delete.as_ref()));
      row3.push(String::from(EditCmd::Price.as_ref()));
      row3.push(String::from(Command::Return.as_ref()));
   }

   let mut keyboard = vec![row1, row2, row3];

   if state.state.is_admin {
      let row_admin = vec![
         String::from(EditCmd::Ban.as_ref()),
         String::from(EditCmd::Owner1.as_ref()),
         String::from(EditCmd::Owner2.as_ref()),
         String::from(EditCmd::Owner3.as_ref()),
      ];
      keyboard.insert(2, row_admin);
   }

   let markup = kb_markup(keyboard);

   cx.answer(info)
   .reply_markup(markup)
   .await?;

   next(state)
}


// ============================================================================
// [Fields editing mode]
// ============================================================================
pub struct GearStateEditing {
   pub state: GearState,
   update: UpdateNode,
}

#[teloxide(subtransition)]
async fn update_edit(mut state: GearStateEditing, cx: TransitionIn<AutoSend<Bot>>, ans: String) -> TransitionOut<Dialogue> {
   async fn do_update(state: &mut GearStateEditing, ans: String) -> Result<String, String> {
      let res = if ans == String::from("/") {
         String::from("Отмена, значение не изменено")
      } else {
         // Store new value
         state.update.kind = match state.update.kind {
            UpdateKind::Text(_) => UpdateKind::Text(ans),
            UpdateKind::Picture(_) => {
               // Delete previous if new id too short
               let id = if ans.len() >= 3 { ans } else { String::default() };
               UpdateKind::Picture(id)
            }
            UpdateKind::Flag(_) => {
               let flag = to_flag(ans)?;
               UpdateKind::Flag(flag)
            }
            UpdateKind::Int(_) => {
               let res = ans.parse::<i64>();
               if let Ok(int) = res {
                  UpdateKind::Int(int)
               } else {
                  return Ok(format!("Ошибка, не удаётся '{}' преобразовать в число, значение не изменено", ans))
               }
            }
            UpdateKind::Time(_, _) => {
               let part1 = ans.get(..5).unwrap_or_default();
               let part2 = ans.get(6..).unwrap_or_default();
               let part1 = NaiveTime::parse_from_str(part1, "%H:%M");
               let part2 = NaiveTime::parse_from_str(part2, "%H:%M");

               if part1.is_ok() && part2.is_ok() {
                  UpdateKind::Time(part1.unwrap(), part2.unwrap())
               } else {
                  return Ok(format!("Ошибка, не удаётся '{}' преобразовать во время работы типа '07:00-21:00', значение не изменено", ans))
               }
            }
            UpdateKind::Money(_) => {
               let res = ans.parse::<i32>();
               if let Ok(int) = res {
                  UpdateKind::Money(int)
               } else {
                  return Ok(format!("Ошибка, не удаётся '{}' преобразовать в число, значение не изменено", ans))
               }
            }
         };
   
         // Peek current node
         let node = state.state.stack.last_mut().unwrap();
   
         // Update database
         let node_id = node.id;
         db::update_node(node_id, &state.update).await?;
   
         // If change in databse is successful, update the stack
         node.update(&state.update)?;
         
         let len = state.state.stack.len();
         if len > 1 {
            let parent = state.state.stack.get_mut(len - 2).unwrap();
            for child in &mut parent.children {
               if child.id == node_id {
                  child.update(&state.update)?;
                  break;
               }
            }
         }
   
         String::from("Новое значение сохранено")
      };
      Ok(res)
   }

   // Report result
   let info = do_update(&mut state, ans)
   .await
   .map_err(|s| map_req_err(s))?;

   cx.answer(info)
   .await?;

   // Reload node
   view(state.state, cx).await
}

async fn enter_edit(state: GearStateEditing, cx: TransitionIn<AutoSend<Bot>>) -> TransitionOut<Dialogue> {
   let (info, markup) = match &state.update.kind {
      UpdateKind::Text(old_val) => (format!("Текущее значение '{}', введите новое или / для отмены", old_val), cancel_markup()),
      UpdateKind::Picture(_) => (String::from("Отправьте изображение или один любой символ для удаления предыдущего или / для отмены"), cancel_markup()),
      UpdateKind::Flag(old_val) => (format!("Текущее значение '{}', выберите новое", from_flag(*old_val)), flag_markup()),
      UpdateKind::Int(old_val) => (format!("Текущее значение user id='{}', введите новое или / для отмены", old_val), cancel_markup()),
      UpdateKind::Time(open, close) => (format!("Текущее время '{}-{}', введите новое или / для отмены", open.format("%H:%M"), close.format("%H:%M")), cancel_markup()),
      UpdateKind::Money(old_val) => (format!("Текущее значение '{}', введите новое или / для отмены", old_val), cancel_markup()),
   };

   cx.answer(info)
   .reply_markup(markup)
   .await?;

   next(state)
}

