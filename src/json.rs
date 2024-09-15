use std::collections::{HashMap, HashSet};
use std::fs;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct ItemQuantity {
    pub item: String,
    pub amount: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Recipe {
    pub name: String,
    #[serde(rename = "className")]
    pub class_name: String,
    pub alternate: bool,
    pub time: f64,
    #[serde(rename = "forBuilding")]
    pub for_building: bool,
    pub ingredients: Vec<ItemQuantity>,
    pub products: Vec<ItemQuantity>,
    #[serde(rename = "producedIn")]
    pub produced_in: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Item {
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Resource {
    #[serde(rename = "item")]
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GameData {
    pub recipes: HashMap<String, Recipe>,
    pub items: HashMap<String, Item>,
    pub resources: HashMap<String, Resource>,
}

impl GameData {
    pub fn new(path: &str) -> GameData {
        let data_text: String = fs::read_to_string(path).expect("Couldn't read file");
        let mut data: GameData = serde_json::from_str(&data_text).expect("Couldn't parse json");
        let mut removes: Vec<String> = Vec::new();
        for (recipe_id, recipe) in data.recipes.iter() {
            if recipe.for_building || recipe_id.contains("ackage") {
                removes.push(recipe_id.clone());
            }
        }
        for recipe_id in removes {
            data.recipes.remove(&recipe_id);
        }
        return data;
    }

    pub fn get_item(&self, item_id: &str) -> &Item {
        return self.items.get(item_id).unwrap();
    }

    pub fn get_recipe(&self, recipe_id: &str) -> &Recipe {
        return self.recipes.get(recipe_id).unwrap();
    }

    pub fn get_item_name(&self, item_id: &str) -> String {
        return self.items.get(item_id).unwrap().name.clone();
    }

    pub fn get_recipe_name(&self, recipe_id: &str) -> String {
        return self.recipes.get(recipe_id).unwrap().name.clone();
    }

    pub fn get_ingredients(&self, recipe_id: &str) -> Vec<String> {
        return self.recipes.get(recipe_id).unwrap().ingredients
            .iter()
            .map(|ingredient| ingredient.item.clone())
            .collect::<Vec<String>>();
    }

    pub fn get_products(&self, recipe_id: &str) -> Vec<String> {
        return self.recipes.get(recipe_id).unwrap().products
            .iter()
            .map(|product| product.item.clone())
            .collect::<Vec<String>>();
    }

    pub fn get_item_creators(&self, item_id: &str, disallowed_recipes: &HashSet<String>) -> Vec<String> {
        let mut creators: Vec<String> = Vec::new();
        for recipe_id in self.recipes.keys().filter(|&recipe_id| !disallowed_recipes.contains(recipe_id)) {
            for product in self.get_products(recipe_id) {
                if product == item_id {
                    creators.push(recipe_id.clone   ());
                }
            }
        }
        return creators;
    }

    pub fn get_item_users(&self, item_id: &str, disallowed_recipes: &HashSet<String>) -> Vec<String> {
        let mut users: Vec<String> = Vec::new();
        for recipe_id in self.recipes.keys().filter(|&recipe_id| !disallowed_recipes.contains(recipe_id)) {
            for ingredient in self.get_ingredients(recipe_id) {
                if ingredient == item_id {
                    users.push(recipe_id.clone());
                }
            }
        }
        return users;
    }
}

