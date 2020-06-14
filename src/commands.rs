/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Команды бота и меню. 31 May 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{
    prelude::*, 
    types::{KeyboardButton, ReplyKeyboardMarkup, InlineKeyboardMarkup, InlineKeyboardButton},
};


// ============================================================================
// [Common]
// ============================================================================
#[derive(SmartDefault)]
pub enum Dialogue {
    #[default]
    Start,
    UserMode,
    EatRestSelectionMode(i32), // cat_id
    EatRestGroupSelectionMode(i32, i32), // cat_id, rest_id
    EatRestGroupDishSelectionMode(i32, i32, i32), // cat_id, rest_id, group_id
    EatRestNowSelectionMode,
    EatRestGroupNowSelectionMode(i32), // rest_id
    CatererMode(i32), // rest_id
    CatEditRestTitle(i32), // rest_id
    CatEditRestInfo(i32), // rest_id
    CatEditRestImage(i32), // rest_id
    CatEditGroup(i32, i32), // rest_id, group_id
    CatAddGroup(i32), // rest_id
    CatEditGroupTitle(i32, i32), // rest_id, group_id (cat_group)
    CatEditGroupInfo(i32, i32), // rest_id, group_id (cat_group)
    CatEditGroupCategory(i32, i32), // rest_id, group_id (cat_group)
    CatEditGroupTime(i32, i32), // rest_id, group_id (cat_group)
    CatAddDish(i32, i32), // rest_id, dish_id (cat_group)
    CatEditDish(i32, i32, i32), // rest_num, group_num, dish_num (dish)
    CatEditDishTitle(i32, i32, i32), // rest_num, group_num, dish_num (dish)), // rest_id, dish_id (dish)
    CatEditDishInfo(i32, i32, i32), // rest_num, group_num, dish_num (dish)), // rest_id, dish_id (dish)
    CatEditDishGroup(i32, i32, i32), // rest_num, group_num, dish_num (dish)), // rest_id, dish_id (dish)
    CatEditDishPrice(i32, i32, i32), // rest_num, group_num, dish_num (dish)), // rest_id, dish_id (dish)
    CatEditDishImage(i32, i32, i32), // rest_num, group_num, dish_num (dish)), // rest_id, dish_id (dish)
}

pub type Cx<State> = DialogueDispatcherHandlerCx<Message, State>;
pub type Res = ResponseResult<DialogueStage<Dialogue>>;



// ============================================================================
// [Client menu]
// ============================================================================
#[derive(Copy, Clone)]
pub enum User {
    // Команды главного меню
    Category(i32),   // cat_id 
    OpenedNow,
    Basket,
    CatererMode, 
    UnknownCommand,
    RegisterCaterer(i32), // user_id
    HoldCaterer(i32), // user_id
    Sudo(i32), // rest_num
    List,
}

impl User {
   pub fn from(input: &str) -> User {
      match input {
         // Сначала проверим на цельные команды.
         "Соки воды" => User::Category(1),
         "Еда" => User::Category(2),
         "Алкоголь" => User::Category(3),
         "Развлечения" => User::Category(4),
         "Сейчас" => User::OpenedNow,
         "Корзина" => User::Basket,
         "Добавить" => User::CatererMode,
         "/list" => User::List,
         _ => {
            // Ищем среди команд с цифровыми суффиксами - аргументами
            match input.get(..5).unwrap_or_default() {
               "/regi" => User::RegisterCaterer(input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
               "/hold" => User::HoldCaterer(input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
               "/sudo" => User::Sudo(input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
               _ => User::UnknownCommand,
            }
         }
      }
   }

    pub fn main_menu_markup() -> ReplyKeyboardMarkup {
        ReplyKeyboardMarkup::default()
            .append_row(vec![
                KeyboardButton::new("Соки воды"),
                KeyboardButton::new("Еда"),
                KeyboardButton::new("Алкоголь"),
                KeyboardButton::new("Развлечения"),
            ])
            .append_row(vec![
                KeyboardButton::new("Сейчас"),
                KeyboardButton::new("Корзина"),
                KeyboardButton::new("Добавить"),
            ])
            .resize_keyboard(true)
    }
}

// ============================================================================
// [Restaurant owner main menu]
// ============================================================================
#[derive(Copy, Clone)]
pub enum Caterer {
    // Команды главного меню
    Main(i32), // rest_id
    Exit, 
    UnknownCommand,
    // Добавляет нового ресторатора user_id или возобновляет его доступ.
    //Registration(u32),
    // Приостанавливает доступ ресторатора user_id и скрывает его меню.
    //Hold(u32),
    // Изменить название ресторана
    EditTitle(i32), // rest_id
    // Изменить описание ресторана
    EditInfo(i32), // rest_id
    // Доступность меню, определяемая самим пользователем
    TogglePause(i32), // rest_id
    // Фото престорана
    EditImage(i32), // rest_id
    // Переход к редактированию указанной группы блюд.
    EditGroup(i32, i32), // rest_id, group_id
    // Добавляет новую группу
    AddGroup(i32), // rest_id
}

impl Caterer {

    pub fn from(rest_id: i32, input: &str) -> Caterer {
        match input {
            // Сначала проверим на цельные команды.
            "Главная" => Caterer::Main(rest_id),
            "Выход" => Caterer::Exit,
            "/EditTitle" => Caterer::EditTitle(rest_id),
            "/EditInfo" => Caterer::EditInfo(rest_id),
            "/Toggle" => Caterer::TogglePause(rest_id),
            "/EditImg" => Caterer::EditImage(rest_id),
            "/AddGroup" => Caterer::AddGroup(rest_id),
            _ => {
                // Ищем среди команд с цифровыми суффиксами - аргументами
                match input.get(..5).unwrap_or_default() {
                    "/EdGr" => Caterer::EditGroup(rest_id, input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
                    _ => Caterer::UnknownCommand,
                }
            }
        }
    }

    pub fn main_menu_markup() -> ReplyKeyboardMarkup {
        ReplyKeyboardMarkup::default()
            .append_row(vec![
                KeyboardButton::new("Главная"),
                KeyboardButton::new("Выход"),
            ])
            .resize_keyboard(true)
            //.one_time_keyboard(true)
    }

    pub fn slash_markup() -> ReplyKeyboardMarkup {
        ReplyKeyboardMarkup::default()
            .append_row(vec![
                KeyboardButton::new("/"),
            ])
            .resize_keyboard(true)
    }
}

// ============================================================================
// [Some]
// ============================================================================
pub async fn remove_slash(s: &str) -> String {
    s.replace("/", "")
}


// ============================================================================
// [Restaurant group editing menu]
// ============================================================================
#[derive(Copy, Clone)]
pub enum CatGroup {
    // Команды главного меню
    Main(i32), // rest_id
    Exit, 
    UnknownCommand,
    // Изменить название группы
    EditTitle(i32, i32), // rest_id, group_id
    // Изменить описание группы
    EditInfo(i32, i32), // rest_id, group_id
    // Переключить доступность группы
    TogglePause(i32, i32), // rest_id, group_id
    // Изменить категорию группы
    EditCategory(i32, i32), // rest_id, group_id
    // Изменить время доступности группы
    EditTime(i32, i32), // rest_id, group_id
    // Удалить группу
    RemoveGroup(i32, i32), // rest_id, group_id
    // Добавление нового блюда
    AddDish(i32, i32), // rest_id, group_id
    // Редактирование блюда
    EditDish(i32, i32, i32), // rest_id, group_id, dish_id
}

impl CatGroup {

    pub fn from(rest_id: i32, group_id: i32, input: &str) -> CatGroup {
        match input {
            // Сначала проверим на цельные команды.
            "Главная" => CatGroup::Main(rest_id),
            "Выход" => CatGroup::Exit,
            "/EditTitle" => CatGroup::EditTitle(rest_id, group_id),
            "/EditInfo" => CatGroup::EditInfo(rest_id, group_id),
            "/Toggle" => CatGroup::TogglePause(rest_id, group_id),
            "/EditCat" => CatGroup::EditCategory(rest_id, group_id),
            "/EditTime" => CatGroup::EditTime(rest_id, group_id),
            "/Remove" => CatGroup::RemoveGroup(rest_id, group_id),
            "/AddDish" => CatGroup::AddDish(rest_id, group_id),
            _ => {
                // Ищем среди команд с цифровыми суффиксами - аргументами
                match input.get(..5).unwrap_or_default() {
                    "/EdDi" => CatGroup::EditDish(rest_id, group_id, input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
                    _ => CatGroup::UnknownCommand,
                }
            }
        }
    }

    pub fn category_markup() -> ReplyKeyboardMarkup {
        ReplyKeyboardMarkup::default()
            .append_row(vec![
                KeyboardButton::new("Соки воды"),
                KeyboardButton::new("Еда"),
                KeyboardButton::new("Алкоголь"),
                KeyboardButton::new("Развлечения"),
            ])
            .resize_keyboard(true)
    }
}


// ============================================================================
// [Restaurant dish editing menu]
// ============================================================================
#[derive(Copy, Clone)]
pub enum CatDish {
    // Команды главного меню
    Main(i32), // rest_id
    Exit, 
    UnknownCommand,
    // Изменить название
    EditTitle(i32, i32, i32), // rest_id, group_id, dish_id
    // Изменить описание
    EditInfo(i32, i32, i32), // rest_id, group_id, dish_id
    // Переключить доступность
    TogglePause(i32, i32, i32), // rest_id, group_id, dish_id
    // Изменить группу
    EditGroup(i32, i32, i32), // rest_id, group_id, dish_id
    // Изменить цену
    EditPrice(i32, i32, i32), // rest_id, group_id, dish_id
    // Изменить цену
    EditImage(i32, i32, i32), // rest_id, group_id, dish_id
    // Удалить
    Remove(i32, i32, i32), // rest_id, group_id, dish_id
}

impl CatDish {

    pub fn from(rest_id: i32, group_id: i32, dish_id: i32, input: &str) -> CatDish {
        match input {
            // Сначала проверим на цельные команды.
            "Главная" => CatDish::Main(rest_id),
            "Выход" => CatDish::Exit,
            "/EditTitle" => CatDish::EditTitle(rest_id, group_id, dish_id),
            "/EditInfo" => CatDish::EditInfo(rest_id, group_id, dish_id),
            "/Toggle" => CatDish::TogglePause(rest_id, group_id, dish_id),
            "/EditGroup" => CatDish::EditGroup(rest_id, group_id, dish_id),
            "/EditPrice" => CatDish::EditPrice(rest_id, group_id, dish_id),
            "/EditImg" => CatDish::EditImage(rest_id, group_id, dish_id),
            "/Remove" => CatDish::Remove(rest_id, group_id, dish_id),
            _ => CatDish::UnknownCommand,
        }
    }
}


// ============================================================================
// [Eater menu, restaurant selection]
// ============================================================================
#[derive(Copy, Clone)]
pub enum EaterRest {
    Main,
    UnknownCommand,
    Restaurant(i32),   // cat_id 
}

impl EaterRest {
   pub fn from(input: &str) -> EaterRest {
      match input {
         // Сначала проверим на цельные команды.
         "В начало" => EaterRest::Main,
         _ => {
             // Ищем среди команд с цифровыми суффиксами - аргументами
             match input.get(..5).unwrap_or_default() {
                 "/rest" => EaterRest::Restaurant(input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
                 _ => EaterRest::UnknownCommand,
             }
         }
     }
   }

   pub fn markup() -> ReplyKeyboardMarkup {
      ReplyKeyboardMarkup::default()
          .append_row(vec![
              KeyboardButton::new("В начало"),
          ])
          .resize_keyboard(true)
  }
}

// ============================================================================
// [Eater menu, group selection]
// ============================================================================
#[derive(Copy, Clone)]
pub enum EaterGroup {
    Main,
    Return,
    UnknownCommand,
    Group(i32),   // cat_id 
}

impl EaterGroup {
   pub fn from(input: &str) -> EaterGroup {
      match input {
         // Сначала проверим на цельные команды.
         "В начало" => EaterGroup::Main,
         "Назад" => EaterGroup::Return,
         _ => {
             // Ищем среди команд с цифровыми суффиксами - аргументами
             match input.get(..5).unwrap_or_default() {
                 "/grou" => EaterGroup::Group(input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
                 _ => EaterGroup::UnknownCommand,
             }
         }
     }
   }

   pub fn markup() -> ReplyKeyboardMarkup {
      ReplyKeyboardMarkup::default()
         .append_row(vec![
            KeyboardButton::new("В начало"),
            KeyboardButton::new("Назад"),
         ])
         .resize_keyboard(true)
  }
}

// ============================================================================
// [Eater menu, dish selection]
// ============================================================================
#[derive(Copy, Clone)]
pub enum EaterDish {
    Main,
    Return,
    UnknownCommand,
    Dish(i32),   // group_id
}

impl EaterDish {
   pub fn from(input: &str) -> EaterDish {
      match input {
         // Сначала проверим на цельные команды.
         "В начало" => EaterDish::Main,
         "Назад" => EaterDish::Return,
         _ => {
             // Ищем среди команд с цифровыми суффиксами - аргументами
             match input.get(..5).unwrap_or_default() {
                 "/dish" => EaterDish::Dish(input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
                 _ => EaterDish::UnknownCommand,
             }
         }
     }
   }

   pub fn markup() -> ReplyKeyboardMarkup {
      ReplyKeyboardMarkup::default()
      .append_row(vec![
         KeyboardButton::new("В начало"),
         KeyboardButton::new("Назад"),
      ])
      .resize_keyboard(true)
   }

   pub fn inline_markup(key: &str, amount: i32) -> InlineKeyboardMarkup {
      // Если количество не пустое, добавим кнопку для убавления
      if amount > 0 {
         InlineKeyboardMarkup::default()
         .append_row(vec![
            InlineKeyboardButton::callback("🧺".to_string(), format!("bas{}", key)),
            InlineKeyboardButton::callback(format!("+1 (в корзине {})", amount), format!("add{}", key)),
         ])
      } else {
         InlineKeyboardMarkup::default()
         .append_row(vec![
            InlineKeyboardButton::callback("🧺".to_string(), format!("bas{}", key)),
            InlineKeyboardButton::callback(format!("+1 (в корзине {})", amount), format!("add{}", key)),
            InlineKeyboardButton::callback("-1".to_string(), format!("del{}", key)),
         ])
      }
   }
}
