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
      None => return String::from("loc::loc error"),
   };

   let res = match s.loc.localize_no_cache(&tag) {
      Ok(cow) => cow,
      Err(e) => return format!("{}", e),
   };

   let res = match res.as_object() {
      Some(map) => map,
      None => return format!("loc: wrong json for '{}'", tag),
   };

   let res = match res.get(key.as_ref()) {
      Some(data) => data,
      None => return format!("loc: key '{}' not found", key.as_ref()),
   };

   let res = match res {
      serde_json::Value::String(res) => res,
      _ => return format!("loc: key '{}' not a string", key.as_ref()),
   };

   res.clone()
}
