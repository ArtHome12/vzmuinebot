/* ===============================================================================
Restaurant menu bot.
Menu item. 14 May 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use chrono::{NaiveTime};
use teloxide::prelude::*;
use tokio_postgres::{Row, };

use crate::states::*;


pub type Owners = [i64; 3];

pub struct Node {
   pub id: i32,  // zero for a new, not saved in database yet or for root
   pub parent: i32,
   pub children: Vec<Node>,
   pub title: String,
   pub descr: String,
   pub picture: String,
   pub enabled: bool,
   pub banned: bool,
   pub owners: Owners,
   pub open: NaiveTime,
   pub close: NaiveTime,
   pub price: i32,
}

impl From<&Row> for Node {
   fn from(row: &Row) -> Self {
      Self {
         id: row.get(0),
         parent: row.get(1),
         children: Vec::new(),
         title: row.get(2),
         descr: row.get(3),
         picture: row.get(4),
         enabled: row.get(5),
         banned: row.get(6),
         owners: [row.get(7), row.get(8), row.get(9)],
         open: row.get(10),
         close: row.get(11),
         price: row.get(12),
      }
   }
}


impl Node {
   // Fields in appropriate order for querying from database
   pub const SELECT: &'static str = "SELECT id, parent, title, descr, picture, enabled, banned, owner1, owner2, owner3, open, close, price FROM nodes WHERE ";

   pub fn new() -> Self {
      Self {
         id: 0,
         parent: 0,
         children: Default::default(),
         title: Default::default(),
         descr: Default::default(),
         picture: Default::default(),
         enabled: false,
         banned: false,
         owners: Default::default(),
         open: NaiveTime::from_hms(0, 0, 0),
         close: NaiveTime::from_hms(0, 0, 0),
         price: 0,
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