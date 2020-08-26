/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Режим едока, выбор блюда после выбора группы и ресторана. 09 June 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{
   prelude::*, 
   types::{InputFile, ReplyMarkup, CallbackQuery, InlineKeyboardButton, 
      ChatOrInlineMessage, InlineKeyboardMarkup, ChatId, InputMedia
   },
};
use arraylib::iter::IteratorExt;

use crate::commands as cmd;
use crate::database as db;
use crate::eater;
use crate::eat_group;
use crate::eat_group_now;
use crate::basket;
use crate::language as lang;
use crate::settings;

// Основная информация режима
pub async fn next_with_info(cx: cmd::Cx<(i32, i32, i32)>) -> cmd::Res {
   // Извлечём параметры
   let (cat_id, rest_num, group_num) = cx.dialogue;
   
   // Получаем информацию из БД сначала о группе
   match db::group(rest_num, group_num).await {
      None => {
         // Такая ситуация не должна возникнуть
         let s = String::from("Ошибка, информации о группе нет");
         let new_cx = DialogueDispatcherHandlerCx::new(cx.bot, cx.update, ());
         cmd::send_text(&new_cx, &s, cmd::EaterRest::markup()).await;
      }
      Some(group) => {
         // Сформируем информацию о группе
         let group_info = format!("{}. {}", group.title, group.info);

         // Получаем из БД список блюд
         let dishes_desc = match db::dish_list(db::DishesBy::Active(rest_num, group_num)).await {
            None => {
               String::from(lang::t("ru", lang::Res::EatDishEmpty))
            }
            Some(dishes) => {
               // Сформируем строку вида "название /ссылка\n"
               dishes.into_iter().map(|dish| (format!("   {} /dish{}\n", dish.title_with_price(), dish.num))).collect()
            }
         };
               
         // Формируем итоговую информацию - добавляем блюда к информации о группе
         let s = format!("{}\n{}", group_info, dishes_desc);

         // Отображаем список блюд
         cx.answer(s)
         .reply_markup(cmd::EaterGroup::markup())
         .disable_notification(true)
         .send()
         .await?;
      }
   }

   // Переходим (остаёмся) в режим выбора ресторана
   next(cmd::Dialogue::EatRestGroupDishSelectionMode(cat_id, rest_num, group_num))
}

// Показывает сообщение об ошибке/отмене без повторного вывода информации
async fn next_with_cancel(cx: cmd::Cx<(i32, i32, i32)>, text: &str) -> cmd::Res {
   cx.answer(text)
   .reply_markup(cmd::EaterDish::markup())
   .disable_notification(true)
   .send()
   .await?;

   // Извлечём параметры
   let (cat_id, rest_id, group_id) = cx.dialogue;

   // Остаёмся в прежнем режиме.
   next(cmd::Dialogue::EatRestGroupDishSelectionMode(cat_id, rest_id, group_id))
}

// Обработчик команд
pub async fn handle_commands(cx: cmd::Cx<(i32, i32, i32)>) -> cmd::Res {
   // Извлечём параметры
   let (cat_id, rest_id, group_id) = cx.dialogue;

   // Разбираем команду.
   match cx.update.text() {
      None => {
         next_with_cancel(cx, "Текстовое сообщение, пожалуйста!").await
      }
      Some(command) => {
         match cmd::EaterDish::from(command) {

            // В корзину
            cmd::EaterDish::Basket => {
               // Код едока
               let user_id = cx.update.from().unwrap().id;
               
               // Переходим в корзину
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               basket::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, user_id)).await
            }

            // В главное меню
            cmd::EaterDish::Main => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
               eater::start(DialogueDispatcherHandlerCx::new(bot, update, ()), false).await
            }

            // В предыдущее меню
            cmd::EaterDish::Return => {
               let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;

               // Попасть сюда могли двумя путями и это видно по коду категории
               if cat_id > 0 {
                  eat_group::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, (cat_id, rest_id))).await
               } else {
                  eat_group_now::next_with_info(DialogueDispatcherHandlerCx::new(bot, update, rest_id)).await
               }
            }

            // Выбор блюда
            cmd::EaterDish::Dish(dish_num) => show_dish(cx, dish_num).await,

            cmd::EaterDish::UnknownCommand => {
               // Сохраним текущее состояние для возврата
               let origin = Box::new(cmd::DialogueState{ d : cmd::Dialogue::EatRestGroupDishSelectionMode(cat_id, rest_id, group_id), m : cmd::EaterDish::markup()});

               // Возможно это общая команда
               if let Some(res) = eater::handle_common_commands(DialogueDispatcherHandlerCx::new(cx.bot.clone(), cx.update.clone(), ()), command, origin).await {return res;}
               else {
                  let s = String::from(command);
                  let DialogueDispatcherHandlerCx { bot, update, dialogue:_ } = cx;
                  next_with_cancel(DialogueDispatcherHandlerCx::new(bot, update, (cat_id, rest_id, group_id)), &format!("Вы в меню выбора блюда: неизвестная команда '{}'", s)).await
               }
            }
         }
      }
   }
}

// Формирование данных для инлайн-сообщения
struct InlineData {
   text: String,
   markup: InlineKeyboardMarkup,
   photo_id: String,
}
async fn inline_data(cat_id: i32, rest_num: i32, group_num: i32) -> InlineData {
   // Получаем информацию из БД сначала о группе
   let (text, markup) = match db::group(rest_num, group_num).await {
      None => {
         // Такая ситуация не должна возникнуть
         // Кнопка назад
         let buttons = vec![InlineKeyboardButton::callback(String::from("назад"), format!("rrg{}", db::make_key_3_int(rest_num, cat_id, 0)))];
         // Формируем меню
         let markup = InlineKeyboardMarkup::default()
         .append_row(buttons);

         // Сформированные данные
         (String::from("Ошибка, информации о группе нет"), markup)
      }
      Some(group) => {
         // Сформируем информацию о группе
         let group_info = format!("{}. {}", group.title, group.info);

         // Получаем из БД список блюд
         let markup = match db::dish_list(db::DishesBy::Active(rest_num, group_num)).await {
            None => {
               // Такая ситуация может возникнуть, если ресторатор скрыл группы только что
               let buttons = vec![InlineKeyboardButton::callback(String::from("назад"), format!("rca{}", db::make_key_3_int(cat_id, 0, 0)))];
               let markup = InlineKeyboardMarkup::default()
               .append_row(buttons);
               markup
            }
            Some(dishes) => {
               // Создадим кнопки
               let buttons: Vec<InlineKeyboardButton> = dishes.into_iter()
               .map(|dish| (InlineKeyboardButton::callback(dish.title_with_price(), format!("dis{}", db::make_key_3_int(rest_num, group_num, dish.num)))))
               .collect();

               // Поделим на длинные и короткие
               let (long, mut short) : (Vec<_>, Vec<_>) = buttons
               .into_iter()
               .partition(|n| n.text.chars().count() > 21);
            
               // Последняя непарная кнопка, если есть
               let last = if short.len() % 2 == 1 { short.pop() } else { None };
            
               // Сначала длинные кнопки по одной
               let markup = long.into_iter() 
               .fold(InlineKeyboardMarkup::default(), |acc, item| acc.append_row(vec![item]));
            
               // Короткие по две в ряд
               let markup = short.into_iter().array_chunks::<[_; 2]>()
               .fold(markup, |acc, [left, right]| acc.append_row(vec![left, right]));
            
               // Кнопка назад
               let button_back = InlineKeyboardButton::callback(String::from("назад"), format!("rrg{}", db::make_key_3_int(rest_num, cat_id, 0)));

               // Добавляем последнюю непарную кнопку и кнопку назад
               let markup = if let Some(last_button) = last {
                  markup.append_row(vec![last_button, button_back])
               } else {
                  markup.append_row(vec![button_back])
               };

               // Сформированные данные
               markup
            }
         };

         (group_info, markup)
      }
   };

   // Попробуем получить картинку ресторана и если её нет, то используем картинку по-умолчанию
   let photo_id = if let Some(rest) = db::restaurant(db::RestBy::Num(rest_num)).await {
      rest.image_or_default()
   }
   else {settings::default_photo_id()};
   
   InlineData{text, markup, photo_id}
}

// Выводит инлайн кнопки, редактируя предыдущее сообщение
pub async fn show_inline_interface(cx: &DispatcherHandlerCx<CallbackQuery>, cat_id: i32, rest_num: i32, group_num: i32) -> bool {

   // Если категория не задана, запросим её из базы
   let cat_id = if cat_id != 0 {cat_id}
   else if let Some(group) = db::group(rest_num, group_num).await {group.cat_id}
   // Если не получилось, выходим
   else {return false;};

   // Получаем информацию
   let data = inline_data(cat_id, rest_num, group_num).await;

   // Достаём chat_id
   let message = cx.update.message.as_ref().unwrap();
   let chat_message = ChatOrInlineMessage::Chat {
      chat_id: ChatId::Id(message.chat_id()),
      message_id: message.id,
   };

   // Приготовим структуру для редактирования
   let media = InputMedia::Photo{
      media: InputFile::file_id(data.photo_id),
      caption: Some(data.text),
      parse_mode: None,
   };

   // Отправляем изменения
   match cx.bot.edit_message_media(chat_message, media)
   .reply_markup(data.markup)
   .send()
   .await {
      Err(e) => {
         settings::log(&format!("Error eat_dish::show_inline_interface {}", e)).await;
         false
      }
      _ => true,
   }
}

// Выводит инлайн кнопки с новым сообщением
pub async fn force_inline_interface(cx: cmd::Cx<(i32, i32, i32)>) -> bool {
   // Извлечём параметры
   let (cat_id, rest_num, group_num) = cx.dialogue;
   
   // Если категория не задана, запросим её из базы
   let cat_id = if cat_id != 0 {cat_id}
   else if let Some(group) = db::group(rest_num, group_num).await {group.cat_id}
   // Если не получилось, выходим
   else {return false;};

   // Получаем информацию
   let data = inline_data(cat_id, rest_num, group_num).await;

   // Отправляем сообщение как фото
   let res = cx.answer_photo(InputFile::file_id(data.photo_id))
   .caption(data.text)
   .reply_markup(ReplyMarkup::InlineKeyboardMarkup(data.markup))
   .disable_notification(true)
   .send()
   .await;

   if let Ok(_) = res {true} else {false}
}

// Показывает информацию о блюде
pub async fn show_dish_inline(cx: &DispatcherHandlerCx<CallbackQuery>, rest_num: i32, group_num: i32, dish_num: i32) -> bool {
   // Идентифицируем пользователя
   let user_id = cx.update.from.id;

   // Получим данные
   let data = prepare_dish_data(user_id, rest_num, group_num, dish_num).await;
   // settings::log(&format!("rrg{}", db::make_key_3_int(est_num, group_num, cat_id))).await;

   // Картинка блюда
   let photo_id = match data.photo_id {
      Some(photo) => photo,
      None => settings::default_photo_id(),
   };

   // Создадим графический объект
   let image = InputFile::file_id(photo_id);

   // Достаём chat_id
   let message = cx.update.message.as_ref().unwrap();
   let chat_id = ChatId::Id(message.chat_id());

   // Отображаем информацию о блюде
   match cx.bot.send_photo(chat_id, image)
   .caption(data.text)
   .reply_markup(ReplyMarkup::InlineKeyboardMarkup(data.markup))
   .disable_notification(true)
   .send()
   .await {
      Err(e) => {
         settings::log(&format!("Error eat_dish::show_dish {}", e)).await;
         false
      }
      _ => true,
   }
}

// Показывает информацию о блюде
pub async fn force_dish_inline(cx: cmd::Cx<(i32, i32, i32)>) -> bool {
   // Извлечём параметры
   let (rest_num, group_num, dish_num) = cx.dialogue;
   let user_id = cx.update.from().unwrap().id;

   // Получим данные
   let data = prepare_dish_data(user_id, rest_num, group_num, dish_num).await;

   // Картинка блюда
   let photo_id = match data.photo_id {
      Some(photo) => photo,
      None => settings::default_photo_id(),
   };

   // Создадим графический объект
   let image = InputFile::file_id(photo_id);

   // Отображаем информацию
   let res = cx.answer_photo(image)
   .caption(data.text)
   .reply_markup(ReplyMarkup::InlineKeyboardMarkup(data.markup))
   .disable_notification(true)
   .send()
   .await;

   match res {
      Err(e) => {
         settings::log(&format!("Error eat_dish::show_dish {}", e)).await;
         false
      }
      Ok(_) => true,
   }
}

pub async fn show_dish(cx: cmd::Cx<(i32, i32, i32)>, dish_num :i32) -> cmd::Res {
   // Извлечём параметры
   let (cat_id, rest_id, group_id) = cx.dialogue;

   // Получаем информацию из БД
   let (info, dish_image_id) = match db::dish(db::DishBy::Active(rest_id, group_id, dish_num)).await {
      Some(dish) => (dish.info_for_eater(), dish.image_id),
      None => (String::from("Информация недоступна"), None)
   };

   // Идентифицируем пользователя
   let user_id = cx.update.from().unwrap().id;

   // Запросим из БД, сколько этих блюд пользователь уже выбрал
   let ordered_amount = db::amount_in_basket(rest_id, group_id, dish_num, user_id).await;

   // Создаём инлайн кнопки с указанием количества блюд
   let inline_keyboard = cmd::EaterDish::inline_markup(&db::make_key_3_int(rest_id, group_id, dish_num), ordered_amount);

   // Отображаем информацию о блюде и оставляем кнопки главного меню. Если для блюда задана картинка, то текст будет комментарием
   if let Some(image_id) = dish_image_id {
      // Создадим графический объект
      let image = InputFile::file_id(image_id);

      // Отправляем картинку и текст как комментарий
      cx.answer_photo(image)
      .caption(info)
      .reply_markup(ReplyMarkup::InlineKeyboardMarkup(inline_keyboard))
      .disable_notification(true)
      .send()
      .await?;
      
      next(cmd::Dialogue::EatRestGroupDishSelectionMode(cat_id, rest_id, group_id))
   } else {
      cx.answer(info)
      .reply_markup(inline_keyboard)
      .disable_notification(true)
      .send()
      .await?;

      next(cmd::Dialogue::EatRestGroupDishSelectionMode(cat_id, rest_id, group_id))
   }
}


// Информация о блюде, подготовленная к выводу
struct DishData {
   text: String,
   markup: InlineKeyboardMarkup,
   photo_id: Option<String>,
}

async fn prepare_dish_data(user_id: i32, rest_num: i32, group_num: i32, dish_num: i32) -> DishData {
   // Получаем информацию из БД
   let (text, photo_id) = match db::dish(db::DishBy::Active(rest_num, group_num, dish_num)).await {
      Some(dish) => (dish.info_for_eater(), dish.image_id),
      None => (String::from("Информация недоступна"), None)
   };

   // Запросим из БД, сколько этих блюд пользователь уже выбрал
   let ordered_amount = db::amount_in_basket(rest_num, group_num, dish_num, user_id).await;

   // Создаём инлайн кнопки с указанием количества блюд
   let button_back = InlineKeyboardButton::callback(String::from("Назад"), format!("rrd{}", db::make_key_3_int(rest_num, group_num, 0)));
   let markup = cmd::EaterDish::inline_markup(&db::make_key_3_int(rest_num, group_num, dish_num), ordered_amount)
   .append_to_row(button_back, 0);

   // Возвращаем результат
   DishData {text, markup, photo_id}
}