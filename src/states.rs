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

// use crate::database as db;
use crate::environment as set;
use crate::gear::*;


// FSM states
#[derive(Transition, From)]
pub enum Dialogue {
   Start(StartState), // initial state
   Command(CommandState), // await for select menu item from bottom
   Settings(GearState), // in settings menu
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
   Unknown,
}

impl From<&str> for MainMenu {
   fn from(s: &str) -> MainMenu {
      match s {
         "⚙" => MainMenu::Gear,
         "🛒" => MainMenu::Basket,
         "Все" => MainMenu::All,
         "Открыто" => MainMenu::Now,
         _ => MainMenu::Unknown,
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
         MainMenu::Unknown => String::from("Неизвестная команда"),
      }
   }
}

// Frequently used menu
pub fn one_button_markup(label: String) -> ReplyMarkup {
   kb_markup(vec![vec![label]])
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
      String::from(MainMenu::Basket),
      String::from(MainMenu::All),
      String::from(MainMenu::Now),
      String::from(MainMenu::Gear),
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
   match MainMenu::from(ans.as_str()) {
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
