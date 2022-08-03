/* ===============================================================================
Restaurant menu bot.
Localize module. 02 August 2022.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020-2022 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */


use strum::{AsRefStr, EnumString, EnumMessage, };
use localize::Localizer;
use once_cell::sync::{OnceCell};

// Access to localize
pub static LOC: OnceCell<Loc> = OnceCell::new();

#[derive(AsRefStr, Debug)]
pub enum Key {
   BasketCommandClear,
   BasketCommandExit,
   BasketCommandDelete,
   BasketCommandReload,
   BasketCommandEditName,
   BasketCommandEditContact,
   BasketCommandEditAddress,
   BasketCommandEditDelivery,
   BasketView1,
   BasketView2,
   BasketView3,
   BasketUpdate,
   BasketMakeOwnerText1,
   BasketMakeOwnerText2,
   BasketMakeOwnerText3,
   BasketMakeOwnerText4,
   BasketOrderMarkup,
   BasketEnterEdit1,
   BasketEnterEdit2,
   BasketEnterEdit3,
   BasketEnterEdit4,
   BasketEnterEdit5,
   BasketEnterEdit6,
   BasketEnterEdit7,
   BasketUpdateEdit1,
   BasketUpdateEdit2,
   BasketUpdateEdit3,
   BasketUpdateEdit4,
   BasketAddressMarkup,
}

pub struct Loc<'a> {
   loc: Localizer<'a>,
}

impl<'a> Loc<'a> {
   pub fn new() -> Self {
      let loc = Localizer::new("locales/").precache_all();
      Self {loc}
   }
}

pub fn loc<'a, T>(key: Key, tag: &'a str, args: &[&T]) -> String
where T: ToString
{
   let s = match LOC.get() {
      Some(s) => s,
      _ => return String::from("loc::loc error"),
   };

   let res = s.loc.localize_no_cache(&tag)
      .map_err(|e| format!("{}", e));
   let res = res.and_then(|res| {
      match res.as_ref() {
         serde_json::Value::String(str_ref) => Ok(str_ref.clone()),
         // _ => Err(String::from("not a string")),
         _ => Err(format!("not a string {:?}", res.as_ref())),
      }
   });

   match res {
      Ok(res) => {
         res
      }
      Err(e) => format!("{}: {}", key.as_ref(), e)
   }
}
