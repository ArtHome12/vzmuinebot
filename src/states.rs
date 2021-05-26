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
use teloxide::{prelude::*, types::{ReplyMarkup, KeyboardButton, KeyboardMarkup, }};
use std::str::FromStr;
use strum::{AsRefStr, EnumString,};

// use crate::database as db;
use crate::environment as set;
use crate::gear::*;


// FSM states
#[derive(Transition, From)]
pub enum Dialogue {
   Start(StartState), // initial state
   Command(CommandState), // await for select menu item from bottom
   Settings(GearState), // in settings menu
   SettingsTitle(GearStateEditing), // in settings menu edit field
}

impl Default for Dialogue {
   fn default() -> Self {
      Self::Start(StartState { restarted: true })
   }
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


// Frequently used menu
pub fn cancel_markup() -> ReplyMarkup {
   kb_markup(vec![vec![String::from("/")]])
}

pub fn flag_markup() -> ReplyMarkup {
   kb_markup(vec![vec![from_flag(true), from_flag(false)]])
}


pub fn kb_markup(keyboard: Vec<Vec<String>>) -> ReplyMarkup {
   let kb:  Vec<Vec<KeyboardButton>> = keyboard.iter()
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

pub async fn enter(state: StartState, cx: TransitionIn<AutoSend<Bot>>, _ans: String,) -> TransitionOut<Dialogue> {
   // Extract user id
   let user = cx.update.from();
   if user.is_none() {
      cx.answer("Error, no user").await?;
      return next(StartState { restarted: false });
   }

   // For admin and regular users there is different interface
   let user_id = user.unwrap().id;
   let is_admin = set::is_admin_id(user_id);

   // Prepare menu
   let commands = vec![
      String::from(MainMenu::Basket.as_ref()),
      String::from(MainMenu::All.as_ref()),
      String::from(MainMenu::Now.as_ref()),
      String::from(MainMenu::Gear.as_ref()),
   ];
   let markup = kb_markup(vec![commands]);

   let info = String::from(if state.restarted { "Извините, бот был перезапущен.\n" } else {""});
   let info = info + if is_admin {
      "Список команд администратора в описании: https://github.com/ArtHome12/vzmuinebot"
   } else {
      "Добро пожаловать. Пожалуйста, нажмите на 'Все' для отображения полного списка, 'Открыто' для работающих сейчас, либо отправьте текст для поиска."
   };

   cx.answer(info)
   .reply_markup(markup)
   .disable_web_page_preview(true)
   .await?;
   next(CommandState { user_id, is_admin })
}

pub struct CommandState {
   pub user_id: i64,
   pub is_admin: bool,
}

#[teloxide(subtransition)]
async fn select_command(state: CommandState, cx: TransitionIn<AutoSend<Bot>>, ans: String,) -> TransitionOut<Dialogue> {
   // Parse and handle commands
   let cmd = MainMenu::from_str(ans.as_str()).unwrap_or(MainMenu::Unknown);
   match cmd {
      MainMenu::Gear => crate::gear::enter(state, cx).await,
      MainMenu::All => crate::node::enter(state, cx).await,
      MainMenu::Basket 
      | MainMenu::Now 
      | MainMenu::Unknown => {
         cx.answer(format!("Неизвестная команда {}. Пожалуйста, выберите одну из команд внизу (если панель с кнопками скрыта, откройте её)", ans)).await?;
         next(state)
      }
   }
}
