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

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

// FSM states
#[derive(Clone, From)]
pub enum State {
   Start(StartState), // initial state
   Command(CommandState), // await for select menu item from bottom
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

// impl State {
//    pub fn create_handler() -> UpdateHandler<Err> {
//       Update::filter_message()
//    }
// }

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
   RequestError::Api(ApiError::Unknown(s))
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
      })
      .collect();

   let markup = KeyboardMarkup::new(kb)
      .resize_keyboard(true);

   ReplyMarkup::Keyboard(markup)
}


#[derive(Clone)]
pub struct StartState {
   pub restarted: bool,
}

async fn start(bot: AutoSend<Bot>, msg: Message, dialogue: MyDialogue, start_state: StartState) -> HandlerResult {
   enter(bot, msg, dialogue, start_state, String::default())
   .await
}

/*
// #[async_recursion]
async fn enter(bot: AutoSend<Bot>, msg: Message, dialogue: MyDialogue) -> HandlerResult {
   bot.send_message(msg.chat.id, "Let's start! What's your full name?").await?;
   dialogue.update(State::ReceiveFullName).await?;

   // Extract user id
   let user = msg.from().unwrap()?;

   Ok(())
} */


pub async fn enter(bot: AutoSend<Bot>, msg: Message, dialogue: MyDialogue, state: StartState, ans: String,) -> HandlerResult {
   let chat_id = msg.chat.id;

   // Extract user id
   let user = msg.from();
   if user.is_none() {
      bot.send_message(chat_id, "Error, no user")
      .await?;
      dialogue.update(StartState { restarted: false }).await?;
      return Ok(());
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
            bot.send_message(chat_id, text)
            .reply_markup(main_menu_markup())
            .await?;
         }

         // We have empty ans when returns from submode and need only to change markup
         if ans.is_empty() {
            let text = "Вы в главном меню";
            bot.send_message(chat_id, text)
            .reply_markup(main_menu_markup())
            .await?;

            dialogue.update(new_state).await?;
         } else {
            // Process general commands without search if restarted (to prevent search submode commands)
/*             crate::general::update(new_state, cx, ans, !state.restarted).await
 */         }
      }
      _ => {
/*          select_command(new_state, cx, ans).await
 */      }
   }

   Ok(())
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

#[derive(Clone)]
pub struct CommandState {
   pub user_id: UserId,
   pub is_admin: bool,
}

/* async fn trans_select_command(state: CommandState, cx: TransitionIn<AutoSend<Bot>>, ans: String,) -> TransitionOut<State> {
   select_command(state, cx, ans).await
}

async fn select_command(state: CommandState, cx: TransitionIn<AutoSend<Bot>>, ans: String,) -> TransitionOut<State> {
   // Parse and handle commands
   let cmd = Command::from_str(ans.as_str()).unwrap_or(Command::Unknown);
   match cmd {
      Command::Gear => crate::gear::enter(state, cx).await,
      Command::All => crate::navigation::enter(state, WorkTime::All, cx).await,
      Command::Now => crate::navigation::enter(state, WorkTime::Now, cx).await,
      Command::Basket => crate::basket::enter(state, cx).await,
      Command::Unknown => crate::general::update(state, cx, ans, true).await,
   }
} */



pub fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {

   let message_handler = Update::filter_message()
   .branch(
      // Private message handler
      dptree::filter(|msg: Message| {
         msg.chat.is_private() // && msg.from().is_some() seems to be unnecessary
      })
      .branch(dptree::case![State::Start(start_state)].endpoint(start))


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


