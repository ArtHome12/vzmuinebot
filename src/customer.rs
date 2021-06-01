/* ===============================================================================
Restaurant menu bot.
Telegram user as customer. 01 Jun 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use std::fmt;
use strum::{AsRefStr, EnumString, };

#[derive(AsRefStr, EnumString)]
pub enum Delivery {
   #[strum(to_string = "Курьером")]
   Courier, // delivery by courier
   #[strum(to_string = "Самовывоз")]
   Pickup, // delivery by customer
}

pub enum Address {
   Text(String),
   Map(String),
}

impl fmt::Display for Address {
   fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      let res = match self {
         Address::Text(s) => s.clone(),
         Address::Map(s) => String::from("точка на карте"),
      };

      write!(f, "{}", res)
   }
}


pub struct Customer {
   pub name: String,
   pub contact: String,
   pub address: Option<Address>,
   pub delivery: Delivery,
}

impl Customer {
   pub fn delivery_desc(&self) -> String {
      match self.delivery {
         Delivery::Courier => {
            match &self.address { 
               Some(addr) => format!("курьером по адресу: {}", addr),
               None => format!("для доставки курьером задайте адрес или выберите самовывоз"),
            }
         }
         Delivery::Pickup => String::from("самовывоз"),
      }
   }
}