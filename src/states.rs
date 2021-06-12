/* ===============================================================================
Restaurant menu bot.
Dialogue FSM. 14 May 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use derive_more::From;
use teloxide_macros::{Transition, teloxide, };
use teloxide::{prelude::*, ApiError, RequestError,
   types::{ReplyMarkup, KeyboardButton, KeyboardMarkup, }
};
use reqwest::StatusCode;
use std::str::FromStr;
use strum::{AsRefStr, EnumString,};
use async_recursion::async_recursion;

use crate::environment as env;
use crate::gear::*;
use crate::basket::*;
use crate::general::MessageState;

// FSM states
#[derive(Transition, From)]
pub enum Dialogue {
   Start(StartState), // initial state
   Command(CommandState), // await for select menu item from bottom
   Settings(GearState), // in settings menu
   SettingsSubmode(GearStateEditing), // in settings menu edit field
   Basket(BasketState), // in basket menu
   BasketSubmode(BasketStateEditing),
   GeneralMessage(MessageState), // general commands, enter text of message to send
}

impl Default for Dialogue {
   fn default() -> Self {
      Self::Start(StartState { restarted: true })
   }
}

// Main menu
#[derive(AsRefStr, EnumString)]
enum Command {
   #[strum(to_string = "⚙")]
   Gear,  // settings menu
   #[strum(to_string = "🛒")]
   Basket,  // basket menu
   #[strum(to_string = "Все")]
   All,  // show all items
   #[strum(to_string = "Открыто")]
   Now,  // show opened items
   Unknown,
}

// Convert for flag value
pub fn to_flag(text: String) -> Result<bool, String> {
   match text.as_str() {
      "Вкл." => Ok(true),
      "Выкл." => Ok(false),
      _ => Err(format!("Ожидается Вкл. или Выкл., получили {}", text)),
   }
}

pub fn from_flag(flag: bool) -> String {
   if flag { String::from("Вкл.") }
   else { String::from("Выкл.") }
}

pub fn map_req_err(s: String) -> RequestError {
   RequestError::ApiError{
      kind: ApiError::Unknown(s),
      status_code: StatusCode::OK,
   }
}

pub enum WorkTime {
   All,  // show all nodes
   Now,  // considering work time
   AllFrom(i32), // like all but from the specified node id
}

// Frequently used menu
pub fn cancel_markup() -> ReplyMarkup {
   kb_markup(vec![vec![String::from("/")]])
}

pub fn flag_markup() -> ReplyMarkup {
   kb_markup(vec![vec![from_flag(true), from_flag(false)]])
}

// Construct keyboard from strings
pub fn kb_markup(keyboard: Vec<Vec<String>>) -> ReplyMarkup {
   let kb: Vec<Vec<KeyboardButton>> = keyboard.iter()
   .map(|row| {
      row.iter()
      .map(|label| KeyboardButton::new(label))
      .collect()
   }).collect();

   let markup = KeyboardMarkup::new(kb)
   .resize_keyboard(true);

   ReplyMarkup::Keyboard(markup)
}


pub struct StartState {
   pub restarted: bool,
}

#[teloxide(subtransition)]
async fn start(state: StartState, cx: TransitionIn<AutoSend<Bot>>, _ans: String,) -> TransitionOut<Dialogue> {
   enter(state, cx, _ans).await
}

#[async_recursion]
pub async fn enter(state: StartState, cx: TransitionIn<AutoSend<Bot>>, ans: String,) -> TransitionOut<Dialogue> {
   // Extract user id
   let user = cx.update.from();
   if user.is_none() {
      cx.answer("Error, no user").await?;
      return next(StartState { restarted: false });
   }

   // For admin and regular users there is different interface
   let user_id = user.unwrap().id;
   let is_admin = env::is_admin_id(user_id);
   let new_state = CommandState { user_id, is_admin };

   // Try to execute command and if it impossible notify about restart
   let cmd = Command::from_str(ans.as_str()).unwrap_or(Command::Unknown);
   match cmd {
      Command::Unknown => {

         // Report about a possible restart and loss of context
         if state.restarted {
            let text = "Извините, бот был перезапущен";
            cx.answer(text)
            .reply_markup(main_menu_markup())
            .await?;
         }

         // Process general commands
         crate::general::update(new_state, cx, ans).await
      }
      _ => {
         select_command(new_state, cx, ans).await
      }
   }
}

pub fn main_menu_markup() -> ReplyMarkup {
   let commands = vec![
      String::from(Command::Basket.as_ref()),
      String::from(Command::All.as_ref()),
      String::from(Command::Now.as_ref()),
      String::from(Command::Gear.as_ref()),
   ];
   kb_markup(vec![commands])
}

pub struct CommandState {
   pub user_id: i64,
   pub is_admin: bool,
}

#[teloxide(subtransition)]
async fn trans_select_command(state: CommandState, cx: TransitionIn<AutoSend<Bot>>, ans: String,) -> TransitionOut<Dialogue> {
   select_command(state, cx, ans).await
}

async fn select_command(state: CommandState, cx: TransitionIn<AutoSend<Bot>>, ans: String,) -> TransitionOut<Dialogue> {
   // Parse and handle commands
   let cmd = Command::from_str(ans.as_str()).unwrap_or(Command::Unknown);
   match cmd {
      Command::Gear => crate::gear::enter(state, cx).await,
      Command::All => crate::inline::enter(state, WorkTime::All, cx).await,
      Command::Now => crate::inline::enter(state, WorkTime::Now, cx).await,
      Command::Basket => crate::basket::enter(state, cx).await,
      Command::Unknown => crate::general::update(state, cx, ans).await,
   }
}
