/* ===============================================================================
Restaurant menu bot.
Menu item. 14 May 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020-2022 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use chrono::{NaiveTime};
use std::hash::{Hash, Hasher};
use teloxide::types::UserId;

use crate::environment as env;
use crate::loc::*;

#[derive(Clone)]
pub struct Owners (pub UserId, pub UserId, pub UserId);

impl Owners {

   pub const VALID_USER_ID: u64 = 10_000;

   pub fn from_int(own1: i64, own2: i64, own3: i64) -> Self {
      Self {
         0: UserId(own1 as u64),
         1: UserId(own2 as u64),
         2: UserId(own3 as u64)
      }
   }
   
   pub fn default() -> Owners {
      Owners::from_int(0, 0, 0)
   }
   
   pub fn has_valid_owner(&self) -> bool {
      self.0.0 > Owners::VALID_USER_ID || self.1.0 > Owners::VALID_USER_ID || self.2.0 > Owners::VALID_USER_ID
   }
}

#[derive(Clone)]
pub struct Node {
   pub id: i32,  // zero for a new, not saved in database yet or for root
   pub parent: i32,
   pub children: Vec<Node>,
   pub title: String,
   pub descr: String,
   pub picture: Origin,
   pub enabled: bool,
   pub banned: bool,
   pub owners: Owners,
   pub time: (NaiveTime, NaiveTime),
   pub price: usize,
}

// Picture type
#[derive(Debug, Clone)]
pub enum Origin {
   None,
   Own(String),
   Inherited(String),
}

impl From<&Origin> for Option<String> {
   fn from(origin: &Origin) -> Option<String> {
       match origin {
          Origin::None => None,
          Origin::Own(id) | Origin::Inherited(id) => Some(id.clone()),
       }
   }
}

impl Origin {
   pub fn derive(&self) -> Self {
      match self {
         Origin::None => Origin::None,
         Origin::Own(id) | Origin::Inherited(id) => Origin::Inherited(id.clone()),
      }
   }
}

// For update fields by name
#[derive(Debug, Clone)]
pub enum UpdateKind {
   Text(String),
   Picture(Origin),
   Flag(bool),
   User(UserId),
   Time(NaiveTime, NaiveTime),
   Money(usize),
}

#[derive(Debug, Clone)]
pub struct UpdateNode {
   pub kind: UpdateKind,
   pub field: String,
}

// Compare Node's by id only.
impl PartialEq for Node {
   fn eq(&self, other: &Self) -> bool { self.id == other.id }
}

impl Eq for Node {}

// Hash by id value only.
impl Hash for Node {
   fn hash<H: Hasher>(&self, h: &mut H) { self.id.hash(h); }
}

impl Node {
   pub fn new(parent: i32, tag: LocaleTag) -> Self {
      let t = NaiveTime::from_hms_opt(0, 0, 0).unwrap_or_default();

      Self {
         id: 0,
         parent,
         children: Default::default(),
         title: loc(Key::NodeDefName, tag, &[]),
         descr: String::from("-"),
         picture: Origin::None,
         enabled: false,
         banned: false,
         owners: Owners::from_int(0, 0, 0),
         time: (t, t),
         price: 0,
      }
   }

   pub fn update(&mut self, info: &UpdateNode) -> Result<(), String> {
      fn check_str(kind: &UpdateKind) -> Result<String, String> {
         match kind {
            UpdateKind::Text(res) => Ok(res.clone()),
            _ => Err(String::from("node::update type string mismatch")),
         }
      }

      fn check_picture(kind: &UpdateKind) -> Result<Origin, String> {
         match kind {
            UpdateKind::Picture(res) => Ok(res.clone()),
            _ => Err(String::from("node::update type string mismatch")),
         }
      }

      fn check_bool(kind: &UpdateKind) -> Result<bool, String> {
         if let UpdateKind::Flag(res) = kind { Ok(*res) }
         else { Err(String::from("node::update type bool mismatch")) }
      }

      fn check_owner(kind: &UpdateKind) -> Result<UserId, String> {
         if let UpdateKind::User(res) = kind { Ok(*res) }
         else { Err(String::from("node::update type user mismatch")) }
      }

      fn check_time(kind: &UpdateKind) -> Result<(NaiveTime, NaiveTime), String> {
         if let UpdateKind::Time(open, close) = kind { Ok((*open, *close)) }
         else { Err(String::from("node::update type time mismatch")) }
      }

      fn check_money(kind: &UpdateKind) -> Result<usize, String> {
         if let UpdateKind::Money(res) = kind { Ok(*res) }
         else { Err(String::from("node::update type int mismatch")) }
      }

      match info.field.as_str() {
         "title" => self.title = check_str(&info.kind)?,
         "descr" => self.descr = check_str(&info.kind)?,
         "picture" => self.picture = check_picture(&info.kind)?,
         "enabled" => self.enabled = check_bool(&info.kind)?,
         "banned" => self.banned = check_bool(&info.kind)?,
         "owner1" => self.owners.0 = check_owner(&info.kind)?,
         "owner2" => self.owners.1 = check_owner(&info.kind)?,
         "owner3" => self.owners.2 = check_owner(&info.kind)?,
         "time" => self.time = check_time(&info.kind)?,
         "price" => self.price = check_money(&info.kind)?,
         _ => return Err(format!("node::update unknown field {}", info.field)),
      }
      Ok(())
   }

   pub fn is_time_set(&self) -> bool {
      let zero = NaiveTime::from_hms_opt(0, 0, 0).unwrap_or_default();
      self.time.0 != zero || self.time.1 != zero
   }

   pub fn title_with_price(&self) -> String {
      let price = if self.price > 0 { String::from(" ") + &env::price_with_unit(self.price) }
      else { String::default() };
   
      format!("{}{}", self.title, price)
   }
}

