/* ===============================================================================
Restaurant menu bot.
Settings menu. 16 May 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020-2022 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{prelude::*, payloads::SendMessageSetters,
   types::{InputMedia, InputFile, InputMediaPhoto, ParseMode, ReplyMarkup, }
};
use strum::AsRefStr;
use chrono::{NaiveTime};

use crate::states::*;
use crate::database as db;
use crate::node::*;
use crate::environment as env;
use crate::loc::*;


// ============================================================================
// [Main entry]
// ============================================================================
// Main commands
enum Command {
   Add, // add a new node
   Delete, // delete node
   Exit, // return to start menu
   Return, // return to parent node
   Goto(i32), // make the specified node active
   Edit(EditCmd),
   Unknown,
}

const GOTO: &str = "/got";

// Main commands
#[derive(AsRefStr, )]
enum EditCmd {
   #[strum(to_string = "title")] // DB field name
   Title,
   #[strum(to_string = "descr")]
   Descr,
   #[strum(to_string = "picture")]
   Picture,
   Advert,
   #[strum(to_string = "enabled")]
   Enable,
   #[strum(to_string = "banned")]
   Ban,
   #[strum(to_string = "owner1")]
   Owner1,
   #[strum(to_string = "owner2")]
   Owner2,
   #[strum(to_string = "owner3")]
   Owner3,
   #[strum(to_string = "time")] // really in db there open and close fields
   Time,
   #[strum(to_string = "price")]
   Price,
}

impl Command {
   fn parse(s: &str, tag: LocaleTag) -> Self {

      // Try as command without arguments
      if s == loc(Key::GearAdd, tag, &[]) { Self::Add }
      else if s == loc(Key::GearDelete, tag, &[]) { Self::Delete }
      else if s == loc(Key::GearExit, tag, &[]) { Self::Exit }
      else if s == loc(Key::GearReturn, tag, &[]) { Self::Return }
      else if s == loc(Key::GearEditTitle, tag, &[]) { Self::Edit(EditCmd::Title) }
      else if s == loc(Key::GearEditDescr, tag, &[]) { Self::Edit(EditCmd::Descr) }
      else if s == loc(Key::GearEditPicture, tag, &[]) { Self::Edit(EditCmd::Picture) }
      else if s == loc(Key::GearEditAdvert, tag, &[]) { Self::Edit(EditCmd::Advert) }
      else if s == loc(Key::GearEditEnable, tag, &[]) { Self::Edit(EditCmd::Enable) }
      else if s == loc(Key::GearEditBan, tag, &[]) { Self::Edit(EditCmd::Ban) }
      else if s == loc(Key::GearEditOwner1, tag, &[]) { Self::Edit(EditCmd::Owner1) }
      else if s == loc(Key::GearEditOwner2, tag, &[]) { Self::Edit(EditCmd::Owner2) }
      else if s == loc(Key::GearEditOwner3, tag, &[]) { Self::Edit(EditCmd::Owner3) }
      else if s == loc(Key::GearEditTime, tag, &[]) { Self::Edit(EditCmd::Time) }
      else if s == loc(Key::GearEditPrice, tag, &[]) { Self::Edit(EditCmd::Price) }
      else {
         // Looking for the commands with arguments
         if s.get(..4).unwrap_or_default() == GOTO {
            let r_part = s.get(4..).unwrap_or_default();
            Command::Goto(r_part.parse().unwrap_or_default())
         } else {
            Command::Unknown
         }
      }
   }
}

#[derive(Clone)]
pub struct GearState {
   pub prev_state: MainState,
   stack: Vec<Node>, // from start to current displaying node
}

pub async fn enter(bot: Bot, msg: Message, dialogue: MyDialogue, state: MainState) -> HandlerResult {

   // Define start node
   let mode = if state.is_admin {
      // Root node
      db::LoadNode::Id(0)
   } else {
      // Find node for owner
      db::LoadNode::Owner(state.user_id)
   };

   // Load node with children
   let node = db::node(mode).await?;

   // Display
   if node.is_some() {
      let new_state = GearState { prev_state: state, stack: vec![node.unwrap()] };
      view(bot, msg, &new_state).await?;
      dialogue.update(new_state).await?;
   } else {
      let contact = env::admin_contact_info();
      // "To access the input mode, refer to '{}' and give it id={}"
      let text = loc(Key::GearEnter, state.tag, &[&contact, &state.user_id]);
      let chat_id = msg.chat.id;
      bot.send_message(chat_id, text).await?;
   }
   Ok(())
}


pub async fn update(bot: Bot, msg: Message, dialogue: MyDialogue, state: GearState) -> HandlerResult {

   async fn do_return(bot: Bot, msg: Message, dialogue: MyDialogue, state: GearState) -> HandlerResult {
      // Extract current node from stack
      let mut new_state = state.clone();
      new_state.stack.pop().unwrap();

      // Go if there are still nodes left
      if !state.stack.is_empty() {
         view(bot, msg, &new_state).await?;
         dialogue.update(new_state).await?
      } else {
         crate::states::reload(bot, msg, dialogue, state.prev_state).await?
      }
      Ok(())
   }

   // === main body

   let chat_id = msg.chat.id;
   let tag = state.prev_state.tag;

   // Parse and handle commands
   let cmd_text = msg.text().unwrap_or_default();
   let cmd = Command::parse(cmd_text, tag);
   match cmd {
      Command::Add => {
         // Extract current node from stack
         let mut new_state = state.clone();
         let node = new_state.stack.last_mut().unwrap();

         // Store a new child node in database with updating id
         let mut child = Node::new(node.id, tag);
         db::node_insert(&mut child).await?;

         // Update current node and push to stack
         node.children.push(child);

         // Show
         view(bot, msg, &new_state).await?;
         dialogue.update(new_state).await?;
         Ok(())
      }

      Command::Exit => crate::states::reload(bot, msg, dialogue, state.prev_state).await,

      Command::Return => do_return(bot, msg, dialogue, state).await,

      Command::Goto(index) => {
         // Peek current node from stack
         let node = state.stack.last().unwrap();

         // Get database id for child index, starting from zero
         let child = node.children.get((index - 1) as usize);

         // Set new node or report error
         if child.is_some() {
            // Load children
            let node = child.unwrap().clone(); // Clone child node as an independent element
            let node = db::node(db::LoadNode::Children(node)).await?
            .unwrap();

            // Push new node and show
            let mut new_state = state.clone();
            new_state.stack.push(node);
            view(bot, msg, &new_state).await?;
            dialogue.update(new_state).await?;
         } else {
            // "Invalid position number '{}', cannot navigate"
            let text = loc(Key::GearUpdateGoto, tag, &[&index]);
            bot.send_message(chat_id, text)
            .reply_markup(markup(&state, tag))
            .await?;
         }

         // Stay in place
         Ok(())
      }

      Command::Delete => {
         let len = state.stack.len();

         // Root/start node cannot to delete
         if len <= 1 {
            // "Cannot delete start node"
            let text = loc(Key::GearUpdateDelete1, tag, &[]);
            bot.send_message(chat_id, text)
            .reply_markup(markup(&state, tag))
            .await?;
            return Ok(())
         }

         // Peek current node from stack
         let mut new_state = state.clone();
         let stack = &mut new_state.stack;
         let node = stack.last().unwrap();

         // Delete record if it has no children
         let children_num = node.children.len();
         if children_num > 0 {
            // "Record '{}' has {} children, to protect against accidental deletion of a large amount of information, delete them first"
            let text = loc(Key::GearUpdateDelete2, tag, &[&node.title, &children_num]);
            bot.send_message(chat_id, text)
            .reply_markup(markup(&state, tag))
            .await?;
            Ok(())
         } else {
            // Delete from database
            let node_id = node.id;
            db::node_delete(node_id).await?;

            // "Record '{}' removed, go one level up"
            let text = loc(Key::GearUpdateDelete3, tag, &[&node.title]);
            bot.send_message(chat_id, text).await?;

            // Delete from stack
            if len > 1 {
               let parent = stack.get_mut(len - 2).unwrap();
               parent.children.retain(|child| child.id != node_id);
            }

            // Change dialogue state and go up 
            do_return(bot, msg, dialogue, new_state).await
         }
      }

      Command::Edit(cmd) => {
         // Editing node
         let node = state.stack.last().unwrap();

         // Underlying data
         let kind = match cmd {
            EditCmd::Title => UpdateKind::Text(node.title.clone()),
            EditCmd::Descr => {
               // "Hint - if the description has only one character, it is not displayed"
               let text = loc(Key::GearUpdateEdit, tag, &[]);
               bot.send_message(chat_id, text).await?;
               UpdateKind::Text(node.descr.clone())
            }
            EditCmd::Picture => UpdateKind::Picture(node.picture.clone()),
            EditCmd::Advert => return send_advert(bot, msg, state).await,
            EditCmd::Enable => UpdateKind::Flag(node.enabled),
            EditCmd::Ban => UpdateKind::Flag(node.banned),
            EditCmd::Owner1 => UpdateKind::User(node.owners.0),
            EditCmd::Owner2 => UpdateKind::User(node.owners.1),
            EditCmd::Owner3 => UpdateKind::User(node.owners.2),
            EditCmd::Time => UpdateKind::Time(node.time.0, node.time.1),
            EditCmd::Price => UpdateKind::Money(node.price),
         };

         // Appropriate database field name
         let field = String::from(cmd.as_ref());

         // Move to editing mode
         let new_state = GearStateEditing {
            prev_state: state,
            update: UpdateNode { kind, field, }
         };
         enter_edit(bot, msg, &new_state).await?;
         dialogue.update(new_state).await?;
         Ok(())
      }

      Command::Unknown => {
         // "It is not clear what to attribute '{}' to. Select a command from the bottom menu first"
         let text = loc(Key::GearUpdateUnknown, tag, &[&cmd_text]);
         bot.send_message(chat_id, text)
         .reply_markup(markup(&state, tag))
         .await?;
         Ok(())
      }
   }
}


async fn view(bot: Bot, msg: Message, state: &GearState) -> HandlerResult {
   let tag = state.prev_state.tag;

   // Collect path from the beginning
   let mut title = state.stack
   .iter()
   .skip(1)
   .fold(String::default(), |acc, n| acc + "/" + &n.title);

   // Add descr if set
   let node = state.stack.last().unwrap();
   if node.descr.len() > 1 {
      title = title + "\n" + EditCmd::Descr.as_ref() + ": " + node.descr.as_str();
   }

   // Add price
   let price = node.price;
   if price > 0 {
      title = format!("{}\n{}: {}", title, EditCmd::Price.as_ref(), env::price_with_unit(price));
   }

   // Add other info
   let tf = loc(Key::CommonTimeFormat, tag, &[]);
   title = format!("{}\n{}: {}, {}: {}\n{}: {}-{}\n{}: {}",
      title,
      loc(Key::GearEditEnable, tag, &[]), from_flag(node.enabled, tag),
      loc(Key::GearEditBan, tag, &[]), from_flag(node.banned, tag),
      loc(Key::GearEditTime, tag, &[]), node.time.0.format(&tf), node.time.1.format(&tf),
      loc(Key::GearEditPicture, tag, &[]), if let Origin::Own(_) = node.picture {
         // "available"
         loc(Key::GearView1, tag, &[])
      } else  {
         // "missing"
         loc(Key::GearView2, tag, &[])
      }
   );

   // List of subnodes width goto command
   let text = state.stack
   .last().unwrap()
   .children.iter()
   .enumerate()
   .fold(title, |acc, n| format!("{}\n{}{} {}", acc, GOTO, n.0 + 1, n.1.title));

   let chat_id = msg.chat.id;
   bot.send_message(chat_id, text)
   .reply_markup(markup(&state, tag))
   .await?;

   Ok(())
}

fn markup(state: &GearState, tag: LocaleTag) -> ReplyMarkup {
   let mut row1 = vec![
      loc(Key::GearAdd, tag, &[]),
      loc(Key::GearEditTitle, tag, &[]),
      loc(Key::GearEditDescr, tag, &[]),
   ];
   let row2 = vec![
      loc(Key::GearEditEnable, tag, &[]),
      loc(Key::GearEditTime, tag, &[]),
      loc(Key::GearEditPicture, tag, &[]),
      loc(Key::GearEditAdvert, tag, &[]),
   ];
   let mut row3 = vec![
      loc(Key::GearExit, tag, &[]),
   ];

   // Condition-dependent menu items
   if state.stack.len() > 1 {
      row1.insert(1, loc(Key::GearDelete, tag, &[]));
      row3.push(loc(Key::GearEditPrice, tag, &[]));
      row3.push(loc(Key::GearReturn, tag, &[]));
   }

   let mut keyboard = vec![row1, row2, row3];

   if state.prev_state.is_admin {
      let row_admin = vec![
         loc(Key::GearEditBan, tag, &[]),
         loc(Key::GearEditOwner1, tag, &[]),
         loc(Key::GearEditOwner2, tag, &[]),
         loc(Key::GearEditOwner3, tag, &[]),
      ];
      keyboard.insert(2, row_admin);
   }

   kb_markup(keyboard)
}

struct TitleAndPicture {
   title: String,
   picture: String,
}

async fn send_advert(bot: Bot, msg: Message, state: GearState) -> HandlerResult {

   fn do_create_pair(node: &Node) -> Option<TitleAndPicture> {
      if let Origin::Own(id) = &node.picture {
         let title = node.title_with_price();
         let picture = id.clone();
         Some(TitleAndPicture{ title, picture })
      } else { None }
   }

   // === main body
   let tag = state.prev_state.tag;
   let chat_id = msg.chat.id;
   // "You can use the message below for forwarding or take only a link from it, when opened, customers will go directly to this post"
   let text = loc(Key::GearSendAdvert, tag, &[]);
   bot.send_message(chat_id, text).await?;

   // Peek current node
   let node = state.stack.last().unwrap();

   // Take photo of the current node and from child nodes, up to ten
   let mut pictures = Vec::with_capacity(10);
   if let Some(pair) = do_create_pair(node) {
      pictures.push(pair);
   }

   // Add children picture if exists
   node.children.iter()
   .filter_map(|node| do_create_pair(node) )
   .take(10 - pictures.len())
   .for_each(|f| pictures.push(f));

   // Three ways to send depending on the number of pictures
   let text = format!("{}\n{}\n{}{}", node.title_with_price(), node.descr, env::link(), node.id);
   let markup = markup(&state, tag);
   match pictures.len() {
      0 => {
         bot.send_message(chat_id, text)
         .reply_markup(markup)
         .parse_mode(ParseMode::Html)
         .disable_web_page_preview(true)
         .await?;
      }
      1 => {
         let picture = &pictures[0].picture;
         bot.send_photo(chat_id, InputFile::file_id(picture))
         .caption(text)
         .reply_markup(markup)
         .parse_mode(ParseMode::Html)
         .await?;
      }
      _ => {
         let media_group = pictures.iter()
         .map(|f| {
            let photo = InputMediaPhoto {
               media : InputFile::file_id(f.picture.clone()), 
               caption: Some(f.title.clone()),
               parse_mode: None,
               caption_entities: None,
               has_spoiler: true,
            };
            InputMedia::Photo(photo)
         });
      
         // Message with pictures
         bot.send_media_group(chat_id, media_group)
         .await?;

         // Text message
         bot.send_message(chat_id, text)
         .reply_markup(markup)
         .parse_mode(ParseMode::Html)
         .disable_web_page_preview(true)
         .await?;
      }
   }

   Ok(())
}


// ============================================================================
// [Fields editing mode]
// ============================================================================
#[derive(Clone)]
pub struct GearStateEditing {
   prev_state: GearState,
   update: UpdateNode,
}

pub async fn update_edit(bot: Bot, msg: Message, dialogue: MyDialogue, state: GearStateEditing) -> HandlerResult {
   async fn do_update(state: &mut GearStateEditing, input: String, tag: LocaleTag) -> Result<String, String> {
      let res = if input == loc(Key::CommonCancel, tag, &[]) {
         // "Cancel, value not changed"
         loc(Key::CommonEditCancel, tag, &[])
      } else {
         // Store new value
         state.update.kind = match state.update.kind {
            UpdateKind::Text(_) => UpdateKind::Text(input.to_owned()),
            UpdateKind::Picture(_) => {
               // Delete previous if new id too short
               let id = if input.len() >= 3 { Origin::Own(input.to_owned()) } else { Origin::None };
               UpdateKind::Picture(id)
            }
            UpdateKind::Flag(_) => {
               let flag = to_flag(&input, tag)?;
               UpdateKind::Flag(flag)
            }
            UpdateKind::User(_) => {
               let res = input.parse::<u64>();
               if let Ok(owner) = res {
                  UpdateKind::User(UserId(owner))
               } else {
                  // "Error, unable to convert '{}' to number, value not changed"
                  let text = loc(Key::GearUpdateEdit1, tag, &[&input]);
                  return Ok(text)
               }
            }
            UpdateKind::Time(_, _) => {
               let part1 = input.get(..5).unwrap_or_default();
               let part2 = input.get(6..).unwrap_or_default();
               let fmt = loc(Key::CommonTimeFormat, tag, &[]);
               let part1 = NaiveTime::parse_from_str(part1, &fmt);
               let part2 = NaiveTime::parse_from_str(part2, &fmt);

               if part1.is_ok() && part2.is_ok() {
                  UpdateKind::Time(part1.unwrap(), part2.unwrap())
               } else {
                  // "Error, unable to convert '{}' while running type '07:00-21:00', value not changed"
                  let text = loc(Key::GearUpdateEdit2, tag, &[&input]);
                  return Ok(text)
               }
            }
            UpdateKind::Money(_) => {
               let res = input.parse::<usize>();
               if let Ok(int) = res {
                  UpdateKind::Money(int)
               } else {
                  // "Error, unable to convert '{}' to number, value not changed"
                  let text = loc(Key::GearUpdateEdit1, tag, &[&input]);
                  return Ok(text)
               }
            }
         };

         // Peek current node
         let stack = &mut state.prev_state.stack;
         let node = stack.last_mut().unwrap();

         // Update database
         let node_id = node.id;
         db::node_update(node_id, &state.update).await?;

         // If change in databse is successful, update the stack
         node.update(&state.update)?;

         let len = stack.len();
         if len > 1 {
            let parent = stack.get_mut(len - 2).unwrap();
            for child in &mut parent.children {
               if child.id == node_id {
                  child.update(&state.update)?;
                  break;
               }
            }
         }

         // "New value saved"
         loc(Key::CommonEditConfirm, tag, &[&input])
      };
      Ok(res)
   }

   // === main body
   let tag = state.prev_state.prev_state.tag;

   // Report result
   let chat_id = msg.chat.id;

   // Collect info about update, if no text there may be image id
   let input = match state.update.kind {
      UpdateKind::Picture(_) => {
         let picture = msg.photo();
         if let Some(sizes) = picture {
            sizes[0].file.id.clone()
         } else {
            loc(Key::CommonCancel, tag, &[]) // "/"
         }
      }
      _ => msg.text().unwrap_or(&loc(Key::CommonCancel, tag, &[])).to_string(),
   };

   let mut new_state = state.clone();
   let text = do_update(&mut new_state, input, tag).await?;

   bot.send_message(chat_id, text)
   .await?;

   // Reload node
   view(bot, msg, &new_state.prev_state).await?;
   dialogue.update(new_state.prev_state).await?;
   Ok(())
}

async fn enter_edit(bot: Bot, msg: Message, state: &GearStateEditing) -> HandlerResult {

   async fn do_enter(bot: Bot, chat_id: ChatId, text: String, markup : ReplyMarkup) -> HandlerResult {
      bot.send_message(chat_id, text)
      .reply_markup(markup)
      .await?;
      Ok(())
   }

   async fn do_enter_picture(bot: Bot, chat_id: ChatId, old_val: &Origin, tag: LocaleTag) -> HandlerResult {
      let opt: Option<String> = old_val.into();

      // "Submit an image (comments are ignored) or press / to cancel"
      let text = loc(Key::GearEnterEdit1, tag, &[]);

      if let Some(old_id) = opt {

         let photo = InputFile::file_id(old_id);

         // Try to send photo
         let res = bot.send_photo(chat_id, photo)
         .caption(&text)
         .reply_markup(cancel_markup(tag))
         .await;

         // In case of error send message
         if res.is_err() {
            // "{} (previous image not available)"
            let text = loc(Key::GearEnterEdit2, tag, &[&text]);
            bot.send_message(chat_id, text)
            .reply_markup(cancel_markup(tag))
            .await?;
         }
      } else {
         bot.send_message(chat_id, text)
         .reply_markup(cancel_markup(tag))
         .await?;
      }
      Ok(())
   }

   // === main body
   let tag = state.prev_state.prev_state.tag;
   let chat_id = msg.chat.id;
   match &state.update.kind {
      UpdateKind::Text(old_val) => {
         // "Current value '{}', enter new or / to cancel"
         let text = loc(Key::GearEnterEdit3, tag, &[&old_val]);
         do_enter(bot, chat_id, text, cancel_markup(tag)).await?
      }
      UpdateKind::Picture(old_val) => do_enter_picture(bot, chat_id, old_val, tag).await?,
      UpdateKind::Flag(old_val) => {
         // "Current value '{}', select new"
         let text = loc(Key::GearEnterEdit4, tag, &[&from_flag(*old_val, tag)]);
         do_enter(bot, chat_id, text, flag_markup(tag)).await?
      }
      UpdateKind::User(old_val) => {
         // "Current value '{}', enter new or / to cancel"
         let text = loc(Key::GearEnterEdit3, tag, &[&old_val]);
         do_enter(bot, chat_id, text, cancel_markup(tag)).await?
      }
      UpdateKind::Time(open, close) => {
         // "Current time '{}-{}', enter new or / to cancel"
         let fmt =  loc(Key::CommonTimeFormat, tag, &[]);
         let text = loc(Key::GearEnterEdit5, tag, &[&open.format(&fmt), &close.format(&fmt)]);
         do_enter(bot, chat_id, text, cancel_markup(tag)).await?
      }
      UpdateKind::Money(old_val) => {
         // "Current value '{}', enter new or / to cancel"
         let text = loc(Key::GearEnterEdit3, tag, &[&old_val]);
         do_enter(bot, chat_id, text, cancel_markup(tag)).await?
      }
   }

   Ok(())
}
