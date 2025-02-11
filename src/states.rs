/* ===============================================================================
Restaurant menu bot.
Dialogue FSM. 14 May 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020-2022 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use derive_more::From;
use teloxide::{prelude::*,
   types::{ReplyMarkup, KeyboardButton, KeyboardMarkup, User, UserId,},
   dispatching::{dialogue::{self, InMemStorage}, UpdateHandler, UpdateFilterExt, },
};

use crate::environment as env;
use crate::database as db;
use crate::gear::*;
use crate::cart::*;
use crate::general::MessageState;
use crate::loc::*;

pub type MyDialogue = Dialogue<State, InMemStorage<State>>;
pub type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

// FSM states
#[derive(Clone, From)]
pub enum State {
   Start(StartState), // initial state
   Command(MainState), // await for select menu item from bottom
   Gear(GearState), // in settings menu
   GearSubmode(GearStateEditing), // in settings menu edit field
   Cart(CartState), // in cart menu
   CartSubmode(CartStateEditing),
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
   pub tag: LocaleTag,
}


// Main menu
enum MainMenu {
   All,  // show all items
   Gear,  // settings menu
   Cart,  // cart menu
   Open,  // show opened items
   Unknown,
}

impl MainMenu {
   fn parse(s: &str, tag: LocaleTag) -> Self {
      if s == loc(Key::StatesMainMenuAll, tag, &[]) { Self::All }
      else if s == loc(Key::StatesMainMenuGear, tag, &[]) { Self::Gear }
      else if s == loc(Key::StatesMainMenuCart, tag, &[]) { Self::Cart }
      else if s == loc(Key::StatesMainMenuOpen, tag, &[]) { Self::Open }
      else { Self::Unknown }
   }
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
      dptree::filter(|msg: Message| { msg.chat.is_private() })
      .branch(dptree::case![State::Start(state)].endpoint(start))
      .branch(dptree::case![State::Command(state)].endpoint(command))
      .branch(dptree::case![State::Cart(state)].endpoint(crate::cart::update))
      .branch(dptree::case![State::CartSubmode(state)].endpoint(crate::cart::update_edit))
      .branch(dptree::case![State::Gear(state)].endpoint(crate::gear::update))
      .branch(dptree::case![State::GearSubmode(state)].endpoint(crate::gear::update_edit))
      .branch(dptree::case![State::GeneralMessage(state)].endpoint(crate::general::update_input))
   )
   .branch(dptree::entry().endpoint(chat_message_handler));

   let callback_query_handler = Update::filter_callback_query().endpoint(callback);

   dialogue::enter::<Update, InMemStorage<State>, State, _>()
   .branch(message_handler)
   .branch(callback_query_handler)
}


async fn start(bot: Bot, msg: Message, dialogue: MyDialogue, state: StartState) -> HandlerResult {

   // Determine the language of the user
   let locale = msg.from.as_ref().and_then(|user| user.language_code.as_deref());
   let locale = tag(locale);

   // Extract user id
   let user = msg.from.as_ref();
   if user.is_none() {
      let chat_id = msg.chat.id;
      bot.send_message(chat_id, "Error, no user")
      .await?;
      dialogue.update(StartState { restarted: false }).await?;
      return Ok(());
   }

   let user = user.unwrap();
   let user_id = user.id;
   let new_state = MainState { prev_state: state, user_id, is_admin: false, tag: locale };

   // Insert or update info about user
   update_last_seen_full(user).await?;

   command(bot, msg, dialogue, new_state)
   .await
}

pub async fn reload(bot: Bot, msg: Message, dialogue: MyDialogue, state: MainState) -> HandlerResult {
   let tag = state.tag;

   dialogue.update(state).await?;

   let text =  loc(Key::StatesMainMenu, tag, &[]); // You are in the main menu
   let chat_id = msg.chat.id;
   bot.send_message(chat_id, text)
   .reply_markup(main_menu_markup(tag))
   .await?;
   
   Ok(())
}

// #[async_recursion]
pub async fn command(bot: Bot, msg: Message, dialogue: MyDialogue, state: MainState) -> HandlerResult {
   // Determine the language of the user
   let locale = msg.from.as_ref().and_then(|user| user.language_code.as_deref());
   let tag = tag(locale);

   let chat_id = msg.chat.id;

   // For admin and regular users there is different interface
   let user_id = state.user_id;
   let new_state = MainState {
      prev_state: StartState { restarted: false },
      user_id,
      is_admin: env::is_admin_id(user_id), // reload permissions every time
      tag,
   };

   // Update FSM
   if state != new_state {
      dialogue.update(new_state.to_owned()).await?;
   }

   // Try to execute command and if it impossible notify about restart
   let text = msg.text().unwrap_or_default();
   let cmd = MainMenu::parse(text, tag);
   match cmd {
      MainMenu::Cart => crate::cart::enter(bot, msg, dialogue, new_state).await?,
      MainMenu::All => crate::navigation::enter(bot, msg, new_state, WorkTime::All).await?,
      MainMenu::Open => crate::navigation::enter(bot, msg, new_state, WorkTime::Now).await?,
      MainMenu::Gear => crate::gear::enter(bot, msg, dialogue, new_state).await?,

      MainMenu::Unknown => {

         // Report about a possible restart and loss of context
         if state.prev_state.restarted {
            let text =  loc(Key::StatesBotRestarted, tag, &[]); // Sorry, the bot has been restarted
            bot.send_message(chat_id, text)
            .reply_markup(main_menu_markup(tag))
            .await?;
         } else {

            // Process general commands without search if restarted (to prevent search submode commands)
            crate::general::update(bot, msg, dialogue, new_state).await?;
         }
      }
   };

   // Update user last seen time
   update_last_seen(user_id).await?;

   Ok(())
}

pub async fn chat_message_handler(bot: Bot, msg: Message) -> HandlerResult {

   // For chat messages react only command for printout group id (need for identify service chat)
   if let Some(input) = msg.text() {
      match input.get(..5).unwrap_or_default() {
         "/chat" => {
            let chat_id = msg.chat.id;
            let text = format!("Chat id={}", chat_id);
            bot.send_message(chat_id, text).await?;
         }
         _ => (),
      }
   }

   Ok(())
}

pub async fn callback(bot: Bot, q: CallbackQuery) -> HandlerResult {
   let user_id = q.from.id;

   // Determine the language of the user
   let locale = q.from.language_code.as_deref();
   let tag = tag(locale);

   let res = crate::callback::update(bot.to_owned(), q.to_owned(), tag).await;

   // Notify user about possible error
   if let Err(e) = res {
      // Sending a response that is shown in a pop-up window
      let text = loc(Key::StatesCallback, tag, &[]); // "Error, start over"
      bot.answer_callback_query(q.id)
      .text(&text)
      .await
      .map_err(|err| format!("inline::update {} {}", text, err))?;

      // Send full text of error
      bot.send_message(q.from.id, format!("{}\n{}", text, e)).await?;

      // For default handler
      return Err(e);
   }

   // Update user last seen time
   update_last_seen(user_id).await?;

   Ok(())
}


pub fn main_menu_markup(tag: LocaleTag) -> ReplyMarkup {
   let commands = vec![
      loc(Key::StatesMainMenuCart, tag, &[]),
      loc(Key::StatesMainMenuAll, tag, &[]),
      loc(Key::StatesMainMenuOpen, tag, &[]),
      loc(Key::StatesMainMenuGear, tag, &[]),
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
pub fn to_flag(text: &String, tag: LocaleTag) -> Result<bool, String> {
   if text == &loc(Key::StatesOn, tag, &[]) { // On
      Ok(true)
   } else if text == &loc(Key::StatesOff, tag, &[]) { // Off
      Ok(false)
   } else {
      Err(loc(Key::StatesWrongSwitch, tag, &[text])) // Expected On or Off, got {}
   }
}

pub fn from_flag(flag: bool, tag: LocaleTag) -> String {
   if flag { loc(Key::StatesOn, tag, &[]) } // On
   else { loc(Key::StatesOff, tag, &[]) } // Off
}

// Frequently used menu
pub fn cancel_markup(tag: LocaleTag) -> ReplyMarkup {
   kb_markup(vec![vec![loc(Key::CommonCancel, tag, &[])]])
}

pub fn flag_markup(tag: LocaleTag) -> ReplyMarkup {
   kb_markup(vec![vec![from_flag(true, tag), from_flag(false, tag)]])
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
      .resize_keyboard();

   ReplyMarkup::Keyboard(markup)
}


