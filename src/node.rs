/* ===============================================================================
Restaurant menu bot.
Menu item. 14 May 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use std::sync::{Arc, Weak};
use chrono::{NaiveTime};
use teloxide::prelude::*;

use crate::states::*;


pub type Owners = [i64; 3];

pub struct Node {
   pub id: i32,  // zero for a new, not saved in database yet or for root
   pub parent: Weak<Node>,
   pub children: Vec<Arc<Node>>,
   pub title: String,
   pub descr: String,
   pub picture: String,
   pub enabled: bool,
   pub banned: bool,
   pub owners: Owners,
   pub open: NaiveTime,
   pub close: NaiveTime,
   price: i32,
}

impl Default for Node {
   fn default() -> Self {
      Self {
         open: NaiveTime::from_hms(0, 0, 0),
         close: NaiveTime::from_hms(0, 0, 0),
         ..Default::default()
      }
   }
}

impl Node {
   pub fn new_root() -> Self {
      Self {
         ..Default::default()
      }
   }
}

pub async fn enter(state: CommandState, cx: TransitionIn<AutoSend<Bot>>,) -> TransitionOut<Dialogue> {
   let info = "Записей нет";

   cx.answer(info)
   .await?;

   // Stay in place
   next(state)
}