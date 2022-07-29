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
use std::str::FromStr;
use strum::{AsRefStr, EnumString, EnumMessage, };
use chrono::{NaiveTime};
use enum_default::EnumDefault;

use crate::states::*;
use crate::database as db;
use crate::node::*;
use crate::environment as env;


// ============================================================================
// [Main entry]
// ============================================================================
// Main commands
#[derive(AsRefStr, EnumString)]
enum Command {
   #[strum(to_string = "Добавить")]
   Add, // add a new node
   #[strum(to_string = "Удалить")]
   Delete, // delete node
   #[strum(to_string = "Выход")]
   Exit, // return to start menu
   #[strum(to_string = "Назад")]
   Return, // return to parent node
   #[strum(to_string = "/pas")]
   Pass(i32), // make the specified node active
   Edit(EditCmd),
   Unknown,
}

// Main commands
#[derive(AsRefStr, EnumString, EnumMessage, EnumDefault)]
enum EditCmd {
   #[strum(to_string = "Название", message = "title")] // Button caption and db field name
   Title,
   #[strum(to_string = "Описание", message = "descr")]
   Descr,
   #[strum(to_string = "Картинка", message = "picture")]
   Picture,
   #[strum(to_string = "Реклама")]
   Advert,
   #[strum(to_string = "Доступ", message = "enabled")]
   Enable,
   #[strum(to_string = "Бан", message = "banned")]
   Ban,
   #[strum(to_string = "ID 1", message = "owner1")]
   Owner1,
   #[strum(to_string = "ID 2", message = "owner2")]
   Owner2,
   #[strum(to_string = "ID 3", message = "owner3")]
   Owner3,
   #[strum(to_string = "Время", message = "time")] // really in db there open and close fields
   Time,
   #[strum(to_string = "Цена", message = "price")]
   Price,
}

impl Command {
   fn parse(s: &str) -> Self {
      // Try as edit subcommand
      if let Ok(edit) = EditCmd::from_str(s) {
         Self::Edit(edit)
      } else {
         // Try as main command
         Self::from_str(s)
         .unwrap_or_else(|_| {
            // Looking for the commands with arguments
            if s.get(..4).unwrap_or_default() == Self::Pass(0).as_ref() {
               let r_part = s.get(4..).unwrap_or_default();
               Command::Pass(r_part.parse().unwrap_or_default())
            } else {
               Command::Unknown
            }
         })
      }
   }
}

#[derive(Clone)]
pub struct GearState {
   pub prev_state: MainState,
   stack: Vec<Node>, // from start to current displaying node
}

pub async fn enter(bot: AutoSend<Bot>, msg: Message, dialogue: MyDialogue, state: MainState) -> HandlerResult {

   // Define start node
   let mode = if state.is_admin {
      // Root node
      db::LoadNode::Id(0)
   } else {
      // Find node for owner
      db::LoadNode::Owner(state.user_id.0 as i64)
   };

   // Load node with children
   let node = db::node(mode).await
      .map_err(|s| map_req_err(s))?;

   // Display
   if node.is_some() {
      let new_state = GearState { prev_state: state, stack: vec![node.unwrap()] };
      view(bot, msg, &new_state).await?;
      dialogue.update(new_state).await?;
   } else {
      let contact = env::admin_contact_info();
      let text = format!("Для доступа в режим ввода информации обратитесь к '{}' и сообщите ему id={}", contact, state.user_id);
      let chat_id = msg.chat.id;
      bot.send_message(chat_id, text).await?;
   }
   Ok(())
}


pub async fn update(bot: AutoSend<Bot>, msg: Message, dialogue: MyDialogue, state: GearState) -> HandlerResult {

   async fn do_return(bot: AutoSend<Bot>, msg: Message, dialogue: MyDialogue, state: GearState) -> HandlerResult {
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

   // Parse and handle commands
   let cmd_text = msg.text().unwrap_or_default();
   let cmd = Command::parse(cmd_text);
   match cmd {
      Command::Add => {
         // Extract current node from stack
         let mut new_state = state.clone();
         let node = new_state.stack.last_mut().unwrap();

         // Store a new child node in database with updating id
         let mut child = Node::new(node.id);
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

      Command::Pass(index) => {
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
            let text = format!("Неверно указан номер записи '{}', нельзя перейти", index);
            bot.send_message(chat_id, text)
            .reply_markup(markup(&state))
            .await?;
         }

         // Stay in place
         Ok(())
      }

      Command::Delete => {
         let len = state.stack.len();

         // Root/start node cannot to delete
         if len <= 1 {
            bot.send_message(chat_id, "Нельзя удалить начальный узел")
            .reply_markup(markup(&state))
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
            let text = format!("У записи '{}' есть {} дочерних, для защиты от случайного удаления большого объёма информации удалите сначала их", node.title, children_num);
            bot.send_message(chat_id, text)
            .reply_markup(markup(&state))
            .await?;
            Ok(())
         } else {
            // Delete from database
            let node_id = node.id;
            db::node_delete(node_id).await?;

            let text = format!("Запись '{}' удалена, переходим на уровень выше", node.title);
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
               let text = "Подсказка - если в описании всего один символ, оно не отображается";
               bot.send_message(chat_id, text).await?;
               UpdateKind::Text(node.descr.clone())
            }
            EditCmd::Picture => UpdateKind::Picture(node.picture.clone()),
            EditCmd::Advert => return send_advert(bot, msg, state).await,
            EditCmd::Enable => UpdateKind::Flag(node.enabled),
            EditCmd::Ban => UpdateKind::Flag(node.banned),
            EditCmd::Owner1 => UpdateKind::Int(node.owners.0),
            EditCmd::Owner2 => UpdateKind::Int(node.owners.1),
            EditCmd::Owner3 => UpdateKind::Int(node.owners.2),
            EditCmd::Time => UpdateKind::Time(node.time.0, node.time.1),
            EditCmd::Price => UpdateKind::Money(node.price),
         };

         // Appropriate database field name
         let field = String::from(cmd.get_message().unwrap());

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
         let text = format!("Непонятно, к чему отнести {}. Выберите сначала команду из нижнего меню", cmd_text);
         bot.send_message(chat_id, text)
         .reply_markup(markup(&state))
         .await?;
         Ok(())
      }
   }
}


async fn view(bot: AutoSend<Bot>, msg: Message, state: &GearState) -> HandlerResult {

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
   title = format!("{}\n{}: {}, {}: {}\n{}: {}-{}\n{}: {}",
      title,
      EditCmd::Enable.as_ref(), from_flag(node.enabled),
      EditCmd::Ban.as_ref(), from_flag(node.banned),
      EditCmd::Time.as_ref(), node.time.0.format("%H:%M"), node.time.1.format("%H:%M"),
      EditCmd::Picture.as_ref(), if let Origin::Own(_) = node.picture { "имеется" } else  { "отсутствует" }
   );

   let text = state.stack
   .last().unwrap()
   .children.iter()
   .enumerate()
   .fold(title, |acc, n| format!("{}\n{}{} {}", acc, Command::Pass(0).as_ref(), n.0 + 1, n.1.title));

   let chat_id = msg.chat.id;
   bot.send_message(chat_id, text)
   .reply_markup(markup(&state))
   .await?;

   Ok(())
}

fn markup(state: &GearState) -> ReplyMarkup {
   let mut row1 = vec![
      String::from(Command::Add.as_ref()),
      String::from(EditCmd::Title.as_ref()),
      String::from(EditCmd::Descr.as_ref()),
   ];
   let row2 = vec![
      String::from(EditCmd::Enable.as_ref()),
      String::from(EditCmd::Time.as_ref()),
      String::from(EditCmd::Picture.as_ref()),
      String::from(EditCmd::Advert.as_ref()),
   ];
   let mut row3 = vec![
      String::from(Command::Exit.as_ref()),
   ];

   // Condition-dependent menu items
   if state.stack.len() > 1 {
      row1.insert(1, String::from(Command::Delete.as_ref()));
      row3.push(String::from(EditCmd::Price.as_ref()));
      row3.push(String::from(Command::Return.as_ref()));
   }

   let mut keyboard = vec![row1, row2, row3];

   if state.prev_state.is_admin {
      let row_admin = vec![
         String::from(EditCmd::Ban.as_ref()),
         String::from(EditCmd::Owner1.as_ref()),
         String::from(EditCmd::Owner2.as_ref()),
         String::from(EditCmd::Owner3.as_ref()),
      ];
      keyboard.insert(2, row_admin);
   }

   kb_markup(keyboard)
}

struct TitleAndPicture {
   title: String,
   picture: String,
}

async fn send_advert(bot: AutoSend<Bot>, msg: Message, state: GearState) -> HandlerResult {

   fn do_create_pair(node: &Node) -> Option<TitleAndPicture> {
      if let Origin::Own(id) = &node.picture {
         let title = node.title_with_price();
         let picture = id.clone();
         Some(TitleAndPicture{ title, picture })
      } else { None }
   }

   // === main body
   let chat_id = msg.chat.id;
   let text = "Вы можете использовать для пересылки сообщение ниже или взять из него только ссылку, при открытии которой клиенты попадут сразу на эту запись";
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
   let markup = markup(&state);
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

pub async fn update_edit(bot: AutoSend<Bot>, msg: Message, dialogue: MyDialogue, state: GearStateEditing) -> HandlerResult {
   async fn do_update(state: &mut GearStateEditing, input: String) -> Result<String, String> {
      let res = if input == String::from("/") {
         String::from("Отмена, значение не изменено")
      } else {
         // Store new value
         state.update.kind = match state.update.kind {
            UpdateKind::Text(_) => UpdateKind::Text(input),
            UpdateKind::Picture(_) => {
               // Delete previous if new id too short
               let id = if input.len() >= 3 { Origin::Own(input) } else { Origin::None };
               UpdateKind::Picture(id)
            }
            UpdateKind::Flag(_) => {
               let flag = to_flag(input)?;
               UpdateKind::Flag(flag)
            }
            UpdateKind::Int(_) => {
               let res = input.parse::<i64>();
               if let Ok(int) = res {
                  UpdateKind::Int(int)
               } else {
                  return Ok(format!("Ошибка, не удаётся '{}' преобразовать в число, значение не изменено", input))
               }
            }
            UpdateKind::Time(_, _) => {
               let part1 = input.get(..5).unwrap_or_default();
               let part2 = input.get(6..).unwrap_or_default();
               let part1 = NaiveTime::parse_from_str(part1, "%H:%M");
               let part2 = NaiveTime::parse_from_str(part2, "%H:%M");

               if part1.is_ok() && part2.is_ok() {
                  UpdateKind::Time(part1.unwrap(), part2.unwrap())
               } else {
                  return Ok(format!("Ошибка, не удаётся '{}' преобразовать во время работы типа '07:00-21:00', значение не изменено", input))
               }
            }
            UpdateKind::Money(_) => {
               let res = input.parse::<usize>();
               if let Ok(int) = res {
                  UpdateKind::Money(int)
               } else {
                  return Ok(format!("Ошибка, не удаётся '{}' преобразовать в число, значение не изменено", input))
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

         String::from("Новое значение сохранено")
      };
      Ok(res)
   }

   // === main body

   // Report result
   let chat_id = msg.chat.id;

   // Collect info about update, if no text there may be image id
   let str = format!("MessageKind={:?}", msg.kind);
   env::log(&str).await;
   let input = match msg.text() {
      Some(text) => String::from(text),
      None => {
         if let Some(photos) = msg.photo() {
            env::log("here2").await;
            photos[0].file_id.to_owned()
         } else {
            env::log("here3").await;
            String::default()
         }
      }
   };
   env::log(&format!("input={}", &input)).await;

   let mut new_state = state.clone();
   let text = do_update(&mut new_state, input).await?;

   bot.send_message(chat_id, text)
   .await?;

   // Reload node
   view(bot, msg, &new_state.prev_state).await?;
   dialogue.update(new_state.prev_state).await?;
   Ok(())
}

async fn enter_edit(bot: AutoSend<Bot>, msg: Message, state: &GearStateEditing) -> HandlerResult {

   async fn do_enter(bot: AutoSend<Bot>, chat_id: ChatId, text: String, markup : ReplyMarkup) -> HandlerResult {
      bot.send_message(chat_id, text)
      .reply_markup(markup)
      .await?;
      Ok(())
   }

   async fn do_enter_picture(bot: AutoSend<Bot>, chat_id: ChatId, old_val: &Origin) -> HandlerResult {
      let opt: Option<String> = old_val.into();
      if let Some(old_id) = opt {

         let text = "Отправьте изображение (комментарии игнорируются) или нажмите / для отмены";
         let photo = InputFile::file_id(old_id);

         bot.send_photo(chat_id, photo)
         .caption(text)
         .reply_markup(cancel_markup())
         .await?;
      } else {
         let text = "Отправьте изображение (комментарии игнорируются) или нажмите / для отмены";
         bot.send_message(chat_id, text)
         .reply_markup(cancel_markup())
         .await?;
      }
      Ok(())
   }

   // === main body
   let chat_id = msg.chat.id;
   match &state.update.kind {
      UpdateKind::Text(old_val) => do_enter(bot, chat_id, format!("Текущее значение '{}', введите новое или / для отмены", old_val), cancel_markup()).await?,
      UpdateKind::Picture(old_val) => do_enter_picture(bot, chat_id, old_val).await?,
      UpdateKind::Flag(old_val) => do_enter(bot, chat_id, format!("Текущее значение '{}', выберите новое", from_flag(*old_val)), flag_markup()).await?,
      UpdateKind::Int(old_val) => do_enter(bot, chat_id, format!("Текущее значение user id='{}', введите новое или / для отмены", old_val), cancel_markup()).await?,
      UpdateKind::Time(open, close) => do_enter(bot, chat_id, format!("Текущее время '{}-{}', введите новое или / для отмены", open.format("%H:%M"), close.format("%H:%M")), cancel_markup()).await?,
      UpdateKind::Money(old_val) => do_enter(bot, chat_id, format!("Текущее значение '{}', введите новое или / для отмены", old_val), cancel_markup()).await?,
   }

   Ok(())
}
