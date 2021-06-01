/* ===============================================================================
Restaurant menu bot.
Basket menu. 01 Jun 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::types::ReplyMarkup;
use teloxide_macros::teloxide;
use teloxide::{prelude::*, payloads::SendMessageSetters,};
use std::str::FromStr;
use strum::{AsRefStr, EnumString, EnumMessage, };
use enum_default::EnumDefault;

use crate::states::*;
use crate::database as db;
use crate::customer::*;

// ============================================================================
// [Main entry]
// ============================================================================
// Main commands
#[derive(AsRefStr, EnumString)]
enum Command {
   #[strum(to_string = "Очистить")]
   Clear, // add a new node
   #[strum(to_string = "Выход")]
   Exit, // return to start menu
   Edit(EditCmd),
   Unknown,
}

// Main commands
#[derive(AsRefStr, EnumString, EnumMessage, EnumDefault)]
enum EditCmd {
   #[strum(to_string = "Имя", message = "name")] // Button caption and db field name
   Name,
   #[strum(to_string = "Контакт", message = "contact")]
   Contact,
   #[strum(to_string = "Адрес", message = "address")]
   Address,
   #[strum(to_string = "Доставка", message = "delivery")]
   Delivery,
}

impl Command {
   fn parse(s: &str) -> Self {
      // Try as edit subcommand
      if let Ok(edit) = EditCmd::from_str(s) {
         Self::Edit(edit)
      } else {
         // Try as main command
         Self::from_str(s)
         .unwrap_or(Command::Unknown)
      }
   }
}


pub struct BasketState {
   pub state: CommandState,
   pub customer: Customer,
}

#[teloxide(subtransition)]
async fn update(state: BasketState, cx: TransitionIn<AutoSend<Bot>>, ans: String) -> TransitionOut<Dialogue> {

   // Parse and handle commands
   let cmd = Command::parse(ans.as_str());
   match cmd {
      Command::Clear => {
         cx.answer(format!("В разработке"))
         .reply_markup(markup())
         .await?;

         // Stay in place
         next(state)
      }

      Command::Exit => crate::states::enter(StartState { restarted: false }, cx, String::default()).await,

      Command::Edit(cmd) => enter_edit(BasketStateEditing { state, cmd }, cx).await,

      Command::Unknown => {
         cx.answer(format!("Неизвестная команда '{}', вы находитесь в корзине", ans))
         .reply_markup(markup())
         .await?;

         // Stay in place
         next(state)
      }
   }
}

pub async fn enter(state: CommandState, cx: TransitionIn<AutoSend<Bot>>,) -> TransitionOut<Dialogue> {

   // Load user info
   let customer = db::user(state.user_id).await
   .map_err(|s| map_req_err(s))?;

   // Display
   let state = BasketState { state, customer };
   view(state, cx).await
}

pub async fn view(state: BasketState, cx: TransitionIn<AutoSend<Bot>>,) -> TransitionOut<Dialogue> {

   // Start with info about user
   let info = format!("Ваши данные, {}:\nКонтакт для связи: {}\nСпособ доставки: {}",
      state.customer.name,
      state.customer.contact,
      state.customer.delivery_desc()
   );

   // Add info about orders

   cx.answer(info)
   .reply_markup(markup())
   .await?;

   next(state)
}

fn markup() -> ReplyMarkup {
   let row1 = vec![
      String::from(EditCmd::Name.as_ref()),
      String::from(EditCmd::Contact.as_ref()),
      String::from(EditCmd::Address.as_ref()),
      String::from(EditCmd::Delivery.as_ref()),
   ];
   let row2 = vec![
      String::from(Command::Clear.as_ref()),
      String::from(Command::Exit.as_ref()),
   ];

   let keyboard = vec![row1, row2];
   kb_markup(keyboard)
}

// ============================================================================
// [Fields editing mode]
// ============================================================================
pub struct BasketStateEditing {
   state: BasketState,
   cmd: EditCmd,
}

#[teloxide(subtransition)]
async fn update_edit(mut state: BasketStateEditing, cx: TransitionIn<AutoSend<Bot>>, ans: String) -> TransitionOut<Dialogue> {
   async fn do_update(state: &mut BasketStateEditing, ans: String) -> Result<String, String> {
      let res = if ans == String::from("/") {
         String::from("Отмена, значение не изменено")
      } else {
         // Store new value

         String::from("Новое значение сохранено")

         /* state.update.kind = match state.update.kind {
            UpdateKind::Text(_) => UpdateKind::Text(ans),
            UpdateKind::Picture(_) => {
               // Delete previous if new id too short
               let id = if ans.len() >= 3 { Some(ans) } else { None };
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
         } */
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

async fn enter_edit(state: BasketStateEditing, cx: TransitionIn<AutoSend<Bot>>) -> TransitionOut<Dialogue> {
   let (info, markup) = match state.cmd {
      EditCmd::Name => (format!("Пожалуйста, {}, укажите как курьер может к Вам обращаться или нажмите / для отмены", state.state.customer.name), cancel_markup()),
      EditCmd::Contact => (format!("Пожалуйста, укажите как курьер может c Вами связаться (текущее значение '{}') или нажмите / для отмены", state.state.customer.contact), cancel_markup()),
      EditCmd::Address => (format!("Текущее значение '{}', введите новый адрес, отправьте геопозицию или нажмите / для отмены", state.state.customer.name), cancel_markup()),
      EditCmd::Delivery => (format!("Текущее значение '{}', укажите как курьер может к Вам обращаться или нажмите / для отмены", state.state.customer.name), delivery_markup()),
   };

   cx.answer(info)
   .reply_markup(markup)
   .await?;

   next(state)
}

pub fn delivery_markup() -> ReplyMarkup {
   kb_markup(vec![vec![
      String::from(Delivery::Courier.as_ref()),
      String::from(Delivery::Pickup.as_ref())
   ]])
}
