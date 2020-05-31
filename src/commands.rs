/* ===============================================================================
Бот для сбора меню у рестораторов и выдача их желающим покушать.
Команды бота и меню. 31 May 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use teloxide::{
    types::{KeyboardButton, ReplyKeyboardMarkup},
};


// ============================================================================
// [User menu]
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
// [Restaurant's owner menu]
// ============================================================================
#[derive(Copy, Clone)]
pub enum Caterer {
    // Команды главного меню
    CatererMain,
    CatererExit, 
    UnknownCommand,
    // Добавляет нового ресторатора user_id или возобновляет его доступ.
    //Registration(u32),
    // Приостанавливает доступ ресторатора user_id и скрывает его меню.
    //Hold(u32),
    // Изменить название ресторана
    EditRestTitle,
    // Изменить описание ресторана
    EditRestInfo,
    // Доступность меню, определяемая самим пользователем
    ToggleRestPause,
    // Переход к редактированию основной группы блюд.
    EditMainGroup,
    // Переход к редактированию указанной группы блюд.
    EditGroup(i32),
    // Добавляет новую группу
    AddGroup,
}

impl Caterer {

    // Приветствие
    pub const WELCOME_MSG: &'static str = "Добро пожаловать в режим для ввода меню!
Изначально всё заполнено значениями по-умолчанию, отредактируйте их. Подсказка - если вы случайно вошли в режим изменения названия и не хотите его менять, то просто введите /";

    pub fn from(input: &str) -> Caterer {
        match input {
            // Сначала проверим на цельные команды.
            "Главная" => Caterer::CatererMain,
            "Выход" => Caterer::CatererExit,
            "/EditTitle" => Caterer::EditRestTitle,
            "/EditInfo" => Caterer::EditRestInfo,
            "/Toggle" => Caterer::ToggleRestPause,
            "/AddGroup" => Caterer::AddGroup,
            "/EditGroup" => Caterer::EditMainGroup,
            _ => {
                // Ищем среди команд с цифровыми суффиксами - аргументами
                match input.get(..5).unwrap_or_default() {
                    "/EdGr" => Caterer::EditGroup(input.get(5..).unwrap_or_default().parse().unwrap_or_default()),
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
    }
}