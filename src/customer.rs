/* ===============================================================================
Restaurant menu bot.
Telegram user as customer. 01 Jun 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020-2022 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::types::{MessageId,};
use crate::loc::*;

#[derive(Clone)]
pub enum Delivery {
   Courier, // delivery by courier
   Pickup, // delivery by customer
}

impl Delivery {
   pub fn from_str(s: &str, tag: LocaleTag) -> Result<Self, ()> {
      if s == loc(Key::CustomerDeliveryCourier, tag, &[]) { Ok(Self::Courier) }
      else if s == loc(Key::CustomerDeliveryPickup, tag, &[]) { Ok(Self::Pickup) }
      else { Err(()) }
   }

   pub fn to_string(&self, tag: LocaleTag) -> String {
      match self {
         Self::Courier => loc(Key::CustomerDeliveryCourier, tag, &[]),
         Self::Pickup => loc(Key::CustomerDeliveryPickup, tag, &[]),
      }
   }
}

#[derive(Clone)]
pub struct Customer {
   pub name: String,
   pub contact: String,
   pub address: String,
   pub delivery: Delivery,
}

impl Customer {
   pub fn delivery_desc(&self, tag: LocaleTag) -> String {
      match self.delivery {
         Delivery::Courier => {
            if self.address.len() <= 1 {
               // "for delivery by courier, enter the address or choose pickup"
               loc(Key::CustomerDeliveryDesk1, tag, &[])
            } else if self.is_location() {
               // "courier for geolocation"
               loc(Key::CustomerDeliveryDesk2, tag, &[])
            } else {
               // "courier to the address: {}"
               loc(Key::CustomerDeliveryDesk3, tag, &[&self.address])
            }
         }
         // "pickup"
         Delivery::Pickup => loc(Key::CustomerDeliveryDesk4, tag, &[]),
      }
   }

   pub fn make_location(location_id: MessageId) -> String {
      format!("Location{}", location_id.0)
   }

   pub fn is_location(&self) -> bool {
      self.address.starts_with("Location")
   }

   pub fn location_id(&self) -> Result<MessageId, ()> {
      if self.is_location() {
         self.address.get(8..)
         .and_then(
            |s| s.parse::<i32>()
            .ok()
            .map(|id| MessageId(id))
         )
         .ok_or(())
      } else {
         Err(())
      }
   }
}