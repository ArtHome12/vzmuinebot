/* ===============================================================================
Restaurant menu bot.
Localize module. 02 August 2022.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020-2022 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */


use strum::AsRefStr;
use walkdir::WalkDir;
use once_cell::sync::OnceCell;
use std::fs;

// Access to localize
pub static LOC: OnceCell<Locale> = OnceCell::new();

#[derive(AsRefStr)]
pub enum Key {
   CommonTimeFormat,
   CommonCancel,
   CommonEditCancel,
   CommonEditConfirm,

   CartCommandClear,
   CartCommandExit,
   CartCommandReload,
   CartCommandEditName,
   CartCommandEditContact,
   CartCommandEditAddress,
   CartCommandEditDelivery,
   CartView1,
   CartView2,
   CartView3,
   CartUpdate,
   CartMakeOwnerText1,
   CartMakeOwnerText2,
   CartMakeOwnerText4,
   CartOrderMarkup,
   CartEnterEdit1,
   CartEnterEdit2,
   CartEnterEdit3,
   CartEnterEdit4,
   CartEnterEdit5,
   CartEnterEdit6,
   CartEnterEdit7,
   CartUpdateEdit,
   CartAddressMarkup,

   CallbackCancel,
   CallbackNext,
   CallbackConfirm,
   CallbackAdded,
   CallbackRemoved,
   CallbackAll,
   CallbackOpen,

   CustomerDeliveryCourier,
   CustomerDeliveryPickup,
   CustomerDeliveryDesk1,
   CustomerDeliveryDesk2,
   CustomerDeliveryDesk3,
   CustomerDeliveryDesk4,

   GearAdd,
   GearDelete,
   GearExit,
   GearReturn,
   GearEditTitle,
   GearEditDescr,
   GearEditPicture,
   GearEditAdvert,
   GearEditEnable,
   GearEditBan,
   GearEditOwner1,
   GearEditOwner2,
   GearEditOwner3,
   GearEditTime,
   GearEditPrice,
   GearEnter,
   GearUpdateGoto,
   GearUpdateDelete1,
   GearUpdateDelete2,
   GearUpdateDelete3,
   GearUpdateEdit,
   GearUpdateUnknown,
   GearView1,
   GearView2,
   GearSendAdvert,
   GearUpdateEdit1,
   GearUpdateEdit2,
   GearEnterEdit1,
   GearEnterEdit2,
   GearEnterEdit3,
   GearEnterEdit4,
   GearEnterEdit5,

   GeneralUpdate1,
   GeneralUpdate2,
   GeneralUpdate3,
   GeneralUpdate4,
   GeneralUpdate5,
   GeneralEnterInput,
   GeneralUpdateInput1,
   GeneralUpdateInput2,
   GeneralUpdateInput3,

   NavigationEnter1,
   NavigationEnter2,
   NavigationEnter3,
   NavigationEnter4,
   NavigationView1,
   NavigationView2,
   NavigationView3,
   NavigationNodeText1,
   NavigationNodeText2,
   NavigationMarkup1,
   NavigationMarkup2,
   NavigationMarkup3,
   NavigationMarkup4,

   NodeDefName,

   RegUpdateStatus,
   RegMakeTicket1,
   RegMakeTicket2,
   RegMakeTicket3,
   RegMakeTicket4,
   RegMakeTicket5,
   RegMakeTicket6,
   RegMakeTicket7,
   RegConfirmTicket,

   StatesMainMenuGear,
   StatesMainMenuCart,
   StatesMainMenuAll,
   StatesMainMenuOpen,
   StatesCallback,
   StatesMainMenu,
   StatesBotRestarted,
   StatesWrongSwitch,
   StatesOn,
   StatesOff,

   TicketOwner1,
   TicketCustomer1,
   TicketOwner2,
   TicketCustomer2,
   TicketOwner3,
   TicketCustomer3,
   TicketOwner4,
   TicketCustomer4,
   TicketOwner5,
   TicketCustomer5,
   TicketOwner6,
   TicketCustomer6,
   TicketOwner7,
   TicketCustomer7,
   TicketMessage,
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
   pub fn new(def_tag: &str) -> Self {
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
      let def_tag = tag(Some(def_tag));

      let info = langs.iter().fold(String::from("Loaded locale:"), |acc, l| format!("{} {}", acc, l.tag));
      log::info!("{}", info);

      Self {langs, def_tag, }
   }
}

pub type Args<'a> = &'a[&'a(dyn std::fmt::Display + Sync)];

pub fn loc(key: Key, tag: LocaleTag, args: Args) -> String
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

   // If there are no arguments, just output the string or substitute values
   let mut res = res.to_owned();
   for arg in args.iter() {
      res = res.replacen("{}", &arg.to_string(), 1);
   }
   res
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
