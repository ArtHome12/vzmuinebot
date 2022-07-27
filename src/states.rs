/* ===============================================================================
Restaurant menu bot.
Dialogue FSM. 14 May 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020-2022 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use derive_more::From;
use teloxide::{prelude::*, ApiError, RequestError,
   types::{ReplyMarkup, KeyboardButton, KeyboardMarkup, User, UserId,},
   dispatching::{dialogue::{self, InMemStorage}, UpdateHandler, UpdateFilterExt, },
};

use reqwest::StatusCode;
use std::str::FromStr;
use strum::{AsRefStr, EnumString,};
use async_recursion::async_recursion;

use crate::environment as env;
use crate::database as db;
use crate::gear::*;
use crate::basket::*;
use crate::general::MessageState;

pub type MyDialogue = Dialogue<State, InMemStorage<State>>;
pub type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

// FSM states
#[derive(Clone, From)]
pub enum State {
   Start(StartState), // initial state
   Command(MainState), // await for select menu item from bottom
   Settings(GearState), // in settings menu
   SettingsSubmode(GearStateEditing), // in settings menu edit field
   Basket(BasketState), // in basket menu
   BasketSubmode(BasketStateEditing),
   GeneralMessage(MessageState), // general commands, enter text of message to send
}

impl Default for State {
   fn default() -> Self {
      Self::Start(StartState { restarted: true })
   }
}

#[derive(Copy, Clone, PartialEq)]
pub struct StartState {
   pub restarted: bool,
}

#[derive(Copy, Clone, PartialEq)]
pub struct MainState {
   pub prev_state: StartState,
   pub user_id: UserId,
   pub is_admin: bool,
}


// Main menu
#[derive(AsRefStr, EnumString)]
enum MainMenu {
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

pub enum WorkTime {
   All,  // show all nodes
   Now,  // considering work time
   AllFrom(i32), // like all but from the specified node id
}


pub fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {

   let message_handler = Update::filter_message()
   .branch(
      // Private message handler
      dptree::filter(|msg: Message| {
         msg.chat.is_private() // && msg.from().is_some() seems to be unnecessary
      })
      .branch(dptree::case![State::Start(state)].endpoint(start))
      .branch(dptree::case![State::Command(state)].endpoint(command))
      .branch(dptree::case![State::Basket(state)].endpoint(crate::basket::update))
      .branch(dptree::case![State::BasketSubmode(state)].endpoint(crate::basket::update_edit))
   );
 
   /* let callback_query_handler = Update::filter_callback_query().chain(
       dptree::case![State::ReceiveProductChoice { full_name }]
           .endpoint(receive_product_selection),
   ); */

   dialogue::enter::<Update, InMemStorage<State>, State, _>()
   .branch(message_handler)
   // .branch(callback_query_handler)
}


async fn start(bot: AutoSend<Bot>, msg: Message, dialogue: MyDialogue, state: StartState) -> HandlerResult {
   // Extract user id
   let user = msg.from();
   if user.is_none() {
      let chat_id = msg.chat.id;
      bot.send_message(chat_id, "Error, no user")
      .await?;
      dialogue.update(StartState { restarted: false }).await?;
      return Ok(());
   }

   let user = user.unwrap();
   let user_id = user.id;
   let new_state = MainState { prev_state: state, user_id, is_admin: false };

   // Insert or update info about user
   update_last_seen_full(user).await?;

   command(bot, msg, dialogue, new_state)
   .await
}

pub async fn reload(bot: AutoSend<Bot>, msg: Message, dialogue: MyDialogue, state: MainState) -> HandlerResult {

   dialogue.update(state).await?;

   let text = "Вы в главном меню";
   let chat_id = msg.chat.id;
   bot.send_message(chat_id, text)
   .reply_markup(main_menu_markup())
   .await?;
   
   Ok(())
}

// #[async_recursion]
pub async fn command(bot: AutoSend<Bot>, msg: Message, dialogue: MyDialogue, state: MainState) -> HandlerResult {
   let chat_id = msg.chat.id;

   // For admin and regular users there is different interface
   let user_id = state.user_id;
   let new_state = MainState {
      prev_state: StartState { restarted: false },
      user_id,
      is_admin: env::is_admin_id(user_id), // reload permissions every time
   };

   // Update FSM
   if state != new_state {
      dialogue.update(new_state.to_owned()).await?;
   }

   // Try to execute command and if it impossible notify about restart
   let text = msg.text().unwrap_or_default();
   let cmd = MainMenu::from_str(text).unwrap_or(MainMenu::Unknown);
   match cmd {
      MainMenu::Basket => {crate::basket::enter(bot, msg, dialogue, new_state).await?;},
      MainMenu::All => {crate::navigation::enter(bot, msg, dialogue, new_state, WorkTime::All).await?;},
      MainMenu::Now => {crate::navigation::enter(bot, msg, dialogue, new_state, WorkTime::Now).await?;},
      // MainMenu::Gear => crate::gear::enter(bot, msg, dialogue, new_state).await,

      MainMenu::Unknown => {

         // Report about a possible restart and loss of context
         if state.prev_state.restarted {
            let text = "Извините, бот был перезапущен";
            bot.send_message(chat_id, text)
            .reply_markup(main_menu_markup())
            .await?;
         } else {

            // Process general commands without search if restarted (to prevent search submode commands)
            crate::general::update(bot, dialogue, text, new_state, !state.prev_state.restarted).await?;
         }
      }
      _ => {}
   };

   // Update user last seen time
   update_last_seen(user_id).await?;

   Ok(())
}

pub fn main_menu_markup() -> ReplyMarkup {
   let commands = vec![
      String::from(MainMenu::Basket.as_ref()),
      String::from(MainMenu::All.as_ref()),
      String::from(MainMenu::Now.as_ref()),
      String::from(MainMenu::Gear.as_ref()),
   ];
   kb_markup(vec![commands])
}


async fn update_last_seen_full(user: &User) -> Result<(), String> {
   let user_id = user.id.0;

   // Collect info about the new user and store in database
   let name = if let Some(last_name) = &user.last_name {
      format!("{} {}", user.first_name, last_name)
   } else {user.first_name.clone()};

   let contact = if let Some(username) = &user.username {
      format!(" @{}", username)
   } else {String::from("-")};

   db::user_insert(user_id, name, contact).await?;
   Ok(())
}


async fn update_last_seen(user_id: UserId) -> Result<(), String> {
   let user_id = user_id.0;
   db::user_update_last_seen(user_id).await?;
   Ok(())
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
   RequestError::Api(ApiError::Unknown(s))
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
      })
      .collect();

   let markup = KeyboardMarkup::new(kb)
      .resize_keyboard(true);

   ReplyMarkup::Keyboard(markup)
}


