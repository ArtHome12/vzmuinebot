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
use teloxide::{payloads::SendMessageSetters, prelude::*, types::{ReplyMarkup, KeyboardButton, KeyboardMarkup, }};
use std::convert::TryFrom;

// use crate::database as db;
use crate::environment as set;
use crate::gear::*;


// FSM states
#[derive(Transition, From)]
pub enum Dialogue {
   Start(StartState), // initial state
   Command(CommandState), // await for select menu item from bottom
   Settings(SettingsState), // in settings menu
}

impl Default for Dialogue {
   fn default() -> Self {
      Self::Start(StartState { restarted: true })
   }
}

// Main menu
enum MainMenu {
   Gear,  // settings menu
   Basket,  // basket menu
   All,  // show all items
   Now,  // show opened items
}

impl TryFrom<&str> for MainMenu {
   type Error = &'static str;

   fn try_from(s: &str) -> Result<Self, Self::Error> {
      match s {
         "⚙" => Ok(MainMenu::Gear),
         _ => Err("Неизвестная команда"),
      }
   }
}

impl From<MainMenu> for String {
   fn from(c: MainMenu) -> String {
      match c {
         MainMenu::Gear => String::from("⚙"),
         MainMenu::Basket => String::from("🛒"),
         MainMenu::All => String::from("Все"),
         MainMenu::Now => String::from("Открыто"),
      }
   }
}

// Frequently used menu
pub fn one_button_markup(label: &'static str) -> ReplyMarkup {
   let keyboard = vec![vec![KeyboardButton::new(label)]];
   let keyboard = KeyboardMarkup::new(keyboard)
   .resize_keyboard(true);

   ReplyMarkup::Keyboard(keyboard)
}


pub struct StartState {
   pub restarted: bool,
}

#[teloxide(subtransition)]
async fn start(state: StartState, cx: TransitionIn<AutoSend<Bot>>, _ans: String,) -> TransitionOut<Dialogue> {
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
      KeyboardButton::new(MainMenu::Basket),
      KeyboardButton::new(MainMenu::All),
      KeyboardButton::new(MainMenu::Now),
      KeyboardButton::new(MainMenu::Gear),
   ];

   let keyboard = KeyboardMarkup::new(vec![commands])
   .resize_keyboard(true);

   let markup = ReplyMarkup::Keyboard(keyboard);

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
   // Parse text from user
   let command = MainMenu::try_from(ans.as_str());
   if command.is_err() {
      cx.answer(format!("Неизвестная команда {}. Пожалуйста, выберите одну из команд внизу (если панель с кнопками скрыта, откройте её)", ans)).await?;

      // Stay in previous state
      return next(state)
   }

   // Handle commands
   match command.unwrap() {
      MainMenu::Gear => crate::gear::enter(state, cx).await,
      _ => next(state),
   }
}
