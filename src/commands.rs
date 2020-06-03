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
    types::{KeyboardButton, ReplyKeyboardMarkup},
};


// ============================================================================
// [Common]
// ============================================================================
#[derive(SmartDefault)]
pub enum Dialogue {
    #[default]
    Start,
    UserMode,
    CatererMode,
    CatEditRestTitle(i32), // rest_id
    CatEditRestInfo(i32), // rest_id
    CatEditGroup(i32, i32), // rest_id, group_id
    CatAddGroup(i32), // rest_id
    CatEditGroupTitle(i32, i32), // rest_id, group_id (cat_group)
    CatEditGroupInfo(i32, i32), // rest_id, group_id (cat_group)
    CatEditGroupCategory(i32, i32), // rest_id, group_id (cat_group)
    CatEditGroupTime(i32, i32), // rest_id, group_id (cat_group)
    CatEditDish(i32, i32), // rest_id, dish_id (dish)
    CatEditDishTitle(i32, i32), // rest_id, dish_id (dish)
    CatEditDishInfo(i32, i32), // rest_id, dish_id (dish)
    CatEditDishGroup(i32, i32), // rest_id, dish_id (dish)
}

pub type Cx<State> = DialogueDispatcherHandlerCx<Message, State>;
pub type Res = ResponseResult<DialogueStage<Dialogue>>;



// ============================================================================
// [Client menu]
// ============================================================================
#[derive(Copy, Clone)]
pub enum User {
    // Команды главного меню
    Water, 
    Food, 
    Alcohol, 
    Entertainment,
    OpenedNow,
    Repeat,
    CatererMode, 
    UnknownCommand,
    // Показать список блюд в указанной категории ресторана /rest#___ cat_id, rest_id, 
    RestaurantMenuInCategory(u32, u32),
    // Показать информацию о блюде /dish___ dish_id
    DishInfo(u32),
    // Показать список доступных сейчас категорий меню ресторана /menu___ rest_id
    RestaurantOpenedCategories(u32),
}

impl User {
    pub fn from(input: &str) -> User {
        match input {
            // Сначала проверим на цельные команды.
            "Соки воды" => User::Water,
            "Еда" => User::Food,
            "Алкоголь" => User::Alcohol,
            "Развлечения" => User::Entertainment,
            "Сейчас" => User::OpenedNow,
            "Повтор" => User::Repeat,
            "Добавить" => User::CatererMode,
            _ => {
                // Ищем среди команд с цифровыми суффиксами - аргументами
                match input.get(..5).unwrap_or_default() {
                    "/rest" => {
                        // Извлекаем аргументы (сначала подстроку, потом число).
                        let arg1 = input.get(5..6).unwrap_or_default().parse().unwrap_or_default();
                        let arg2 = input.get(6..).unwrap_or_default().parse().unwrap_or_default();

                        // Возвращаем команду.
                        User::RestaurantMenuInCategory(arg1, arg2)
                    }
                    "/dish" => User::DishInfo(input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
                    "/menu" => User::RestaurantOpenedCategories(input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
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
                KeyboardButton::new("Повтор"),
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
    // Переход к редактированию указанной группы блюд.
    EditGroup(i32, i32), // rest_id, group_id
    // Добавляет новую группу
    AddGroup(i32), // rest_id
}

impl Caterer {

    // Приветствие
    pub const WELCOME_MSG: &'static str = "Добро пожаловать в режим ввода меню!
Изначально всё заполнено значениями по-умолчанию, отредактируйте их.";

    pub fn from(rest_id: i32, input: &str) -> Caterer {
        match input {
            // Сначала проверим на цельные команды.
            "Главная" => Caterer::Main(rest_id),
            "Выход" => Caterer::Exit,
            "/EditTitle" => Caterer::EditTitle(rest_id),
            "/EditInfo" => Caterer::EditInfo(rest_id),
            "/Toggle" => Caterer::TogglePause(rest_id),
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
            _ => {
                // Ищем среди команд с цифровыми суффиксами - аргументами
                /*match input.get(..5).unwrap_or_default() {
                    "/EdGr" => Caterer::EditGroup(rest_id, input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
                    _ => */CatGroup::UnknownCommand
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
    EditTitle(i32, i32), // rest_id, group_id
    // Изменить описание
    EditInfo(i32, i32), // rest_id, group_id
    // Переключить доступность
    TogglePause(i32, i32), // rest_id, group_id
    // Изменить группу
    EditGroup(i32, i32), // rest_id, group_id
    // Удалить
    Remove(i32, i32), // rest_id, group_id
}

impl CatDish {

    pub fn from(rest_id: i32, dish_id: i32, input: &str) -> CatDish {
        match input {
            // Сначала проверим на цельные команды.
            "Главная" => CatDish::Main(rest_id),
            "Выход" => CatDish::Exit,
            "/EditTitle" => CatDish::EditTitle(rest_id, dish_id),
            "/EditInfo" => CatDish::EditInfo(rest_id, dish_id),
            "/Toggle" => CatDish::TogglePause(rest_id, dish_id),
            "/EditGroup" => CatDish::EditGroup(rest_id, dish_id),
            "/Remove" => CatDish::Remove(rest_id, dish_id),
            _ => {
                // Ищем среди команд с цифровыми суффиксами - аргументами
                /*match input.get(..5).unwrap_or_default() {
                    "/EdGr" => Caterer::EditGroup(rest_id, input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
                    _ => */CatDish::UnknownCommand
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