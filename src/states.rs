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

#[derive(Clone)]
pub struct StartState {
   pub restarted: bool,
}

#[derive(Clone)]
pub struct MainState {
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


      // .endpoint(|msg: Message, bot: AutoSend<Bot>| async move {

         // Insert new user or update his last seen time
         // let user = msg.from();
         // update_last_seen(user)
         // .await
         // .map_err(|s| map_req_err(s))?;


         // For admin and regular users there is different interface
         // dptree::filter(|msg: Message| {
         //    msg.from().map(|user| env::is_admin_id(user.id.0)).unwrap_or_default()
         // })
         //    .endpoint(|msg: Message, bot: AutoSend<Bot>| async move {
         //       bot.send_message(msg.chat.id, "This is admin.").await?;
         //       respond(())
         // })

      //    Ok(())
      // })
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
      bot.send_message(chat_id, "Error, no user")
      .await?;
      dialogue.update(StartState { restarted: false }).await?;
      return Ok(());
   }

   let new_state = MainState { user_id, is_admin: false };

   command(bot, msg, dialogue, new_state)
   .await
}

// #[async_recursion]
pub async fn command(bot: AutoSend<Bot>, msg: Message, dialogue: MyDialogue, state: MainState) -> HandlerResult {
   let chat_id = msg.chat.id;

   // For admin and regular users there is different interface
   let user_id = state.user_id;
   let is_admin = env::is_admin_id(user_id); // reload permissions
   let new_state = MainState { user_id, is_admin };

   // Try to execute command and if it impossible notify about restart
   let text = msg.text().unwrap_or_default();
   let cmd = MainMenu::from_str(text).unwrap_or(MainMenu::Unknown);
   match cmd {
      MainMenu::Basket => {crate::basket::enter(bot, msg, dialogue, new_state).await?;},
      // Command::All => crate::navigation::enter(new_state, WorkTime::All, cx).await,
      // Command::Now => crate::navigation::enter(new_state, WorkTime::Now, cx).await,
      // Command::Gear => crate::gear::enter(bot, msg, dialogue, new_state).await,

      MainMenu::Unknown => {

         // Report about a possible restart and loss of context
         if state.restarted {
            let text = "Извините, бот был перезапущен";
            bot.send_message(chat_id, text)
            .reply_markup(main_menu_markup())
            .await?;
         }

         /* // We have empty ans when returns from submode and need only to change markup
         if ans.is_empty() {
            let text = "Вы в главном меню";
            bot.send_message(chat_id, text)
            .reply_markup(main_menu_markup())
            .await?;

         } else {
            // Process general commands without search if restarted (to prevent search submode commands)
            crate::general::update(new_state, cx, ans, !state.restarted).await*/

            dialogue.update(new_state).await?;
         }
      _ => {dialogue.update(new_state).await?;}
   };

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


async fn update_last_seen(user: Option<&User>) -> Result<(), String> {
   if user.is_none() {
      return Err(String::from("states update_last_seen() user is none"));
   }

   let user = user.unwrap();
   let user_id = user.id.0;
   let successful = db::user_update_last_seen(user_id).await?;

   // If unsuccessful, then there is no such user
   if !successful {
      // Collect info about the new user and store in database
      let name = if let Some(last_name) = &user.last_name {
         format!("{} {}", user.first_name, last_name)
      } else {user.first_name.clone()};

      let contact = if let Some(username) = &user.username {
         format!(" @{}", username)
      } else {String::from("-")};

      db::user_insert(user_id, name, contact).await?;
   }
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


