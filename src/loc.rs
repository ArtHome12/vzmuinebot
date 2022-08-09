/* ===============================================================================
Restaurant menu bot.
Localize module. 02 August 2022.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020-2022 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */


use strum::{AsRefStr };
use walkdir::WalkDir;
use once_cell::sync::{OnceCell};
use std::fs;

// Access to localize
pub static LOC: OnceCell<Locale> = OnceCell::new();

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

pub type LocaleTag = u32;

struct Lang {
   tag: String,
   map: serde_json::Map<String, serde_json::Value>,
}

pub struct Locale {
   langs: Vec<Lang>,
   def_tag: u32,
}

impl Locale {
   pub fn new() -> Self {
      let mut langs = vec![];

      // Load "tag".json from directory
      for entry in WalkDir::new("locales/").into_iter().filter_map(|e| e.ok()) {
         if entry.file_type().is_file() && entry.file_name().to_string_lossy().ends_with(".json") {

            // Extract filename as tag
            let tag = entry.file_name()
            .to_str()
            .unwrap()
            .split_once(".json")
            .unwrap()
            .0
            .to_string();

            // Open file
            if let Ok(file) = fs::File::open(entry.path()) {
               // Read data
               if let Ok(data) = serde_json::from_reader(file) {
                  // Get as JSON object
                  let json: serde_json::Value = data;
                  if let Some(map) = json.as_object() {
                     // Store
                     let lang = Lang {
                        tag,
                        map: map.to_owned(),
                     };
                     langs.push(lang);
                  } else {
                     log::error!("loc::new() wrong json '{}'", entry.path().display())
                  }
               } else {
                  log::error!("loc::new() read error '{}'", entry.path().display())
               }
            } else {
               log::error!("loc::new() open error '{}'", entry.path().display())
            }
         }
      }

      // Sorting for binary search
      langs.sort_by(|a, b| a.tag.cmp(&b.tag));

      // After sort, store default locale
      let def_tag = tag(Some("en"));

      let info = langs.iter().fold(String::from("Loaded locale:"), |acc, l| format!("{} {}", acc, l.tag));
      log::info!("{}", info);

      Self {langs, def_tag, }
   }
}

pub fn loc<'a, T>(key: Key, tag: LocaleTag, args: &[&T]) -> String
where T: std::fmt::Display
{
   let s = match LOC.get() {
      Some(s) => s,
      None => return String::from("loc::loc error"),
   };

   if tag >= s.langs.len() as u32 {
      return format!("loc::loc too big tag '{}' for langs '{}'", tag, s.langs.len())
   }
   let res = &s.langs[tag as usize].map;

   let res = match res.get(key.as_ref()) {
      Some(data) => data,
      None => return format!("loc: key '{}' not found", key.as_ref()),
   };

   let res = match res {
      serde_json::Value::String(res) => res,
      _ => return format!("loc: key '{}' not a string", key.as_ref()),
   };

   res.split("{}")
   .zip(args.iter())
   .fold(String::default(), |acc, part| format!("{}{}{}", acc, part.0, *part.1))
}

pub fn tag(tag: Option<&str>) -> LocaleTag {
   let s = match LOC.get() {
      Some(s) => s,
      None => return 0u32,
   };

   let tag = match tag {
      Some(tag) => tag,
      None => return s.def_tag,
   };

   s.langs
   .binary_search_by(|elem|
      elem.tag.as_str().cmp(tag)
   ).unwrap_or(s.def_tag as usize) as u32
}
