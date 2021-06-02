/* ===============================================================================
Restaurant menu bot.
Telegram user as customer. 01 Jun 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use strum::{AsRefStr, EnumString, };

#[derive(AsRefStr, EnumString)]
pub enum Delivery {
   #[strum(to_string = "Курьером")]
   Courier, // delivery by courier
   #[strum(to_string = "Самовывоз")]
   Pickup, // delivery by customer
}

pub struct Customer {
   pub name: String,
   pub contact: String,
   pub address: String,
   pub delivery: Delivery,
}

impl Customer {
   pub fn delivery_desc(&self) -> String {
      match self.delivery {
         Delivery::Courier => {
            if self.address.len() <= 1 {
               String::from("для доставки курьером задайте адрес или выберите самовывоз")
            } else if self.is_location() {
               String::from("курьером на геопозицию")
            } else {
               format!("курьером по адресу: {}", self.address)
            }
         }
         Delivery::Pickup => String::from("самовывоз"),
      }
   }

   pub fn make_location(location_id: i32) -> String {
      format!("Location{}", location_id)
   }

   pub fn is_location(&self) -> bool {
      self.address.starts_with("Location")
   }

   pub fn location_id(&self) -> Result<i32, ()> {
      if self.is_location() {
         self.address.get(8..)
         .and_then(|s| s.parse::<i32>().ok())
         .ok_or(())
      } else {
         Err(())
      }
   }
}