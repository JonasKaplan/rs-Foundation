use std::collections::{HashMap, HashSet};

use minilp::{ComparisonOp, LinearExpr, OptimizationDirection, Problem, Solution, Variable};

use crate::json::{GameData, ItemQuantity, Recipe};

#[derive(Debug)]
pub struct Resource {
    limit: f64,
    weight: f64,
}

#[derive(Debug, Clone)]
struct Link {
    item: String,
    destination: String,
    variable: Variable,
}

impl Link {
    fn new(item: &str, destination: &str, variable: Variable) -> Link {
        return Link {
            item: item.to_string(),
            destination: destination.to_string(),
            variable,
        };
    }
}

#[derive(Debug)]
struct LinkSet {
    links: HashMap<String, Vec<Link>>,
}

impl LinkSet {
    fn new() -> LinkSet {
        return LinkSet {
            links: HashMap::new(),
        }
    }

    fn add_simple_variable(&mut self, source: &str, destination: &str, item: &str, problem: &mut Problem) -> () {
        let link: Link = Link::new(item, destination, problem.add_var(1.0, (0.0, f64::MAX)));
        if self.links.contains_key(source) {
            self.links.get_mut(source).unwrap().push(link);
        } else {
            self.links.insert(source.to_string(), vec![link]);
        }
    }

    fn add_weighted_variable(&mut self, source: &str, destination: &str, item: &str, weight: f64, problem: &mut Problem) -> () {
        let link: Link = Link::new(item, destination, problem.add_var(weight, (0.0, f64::MAX)));
        if self.links.contains_key(source) {
            self.links.get_mut(source).unwrap().push(link);
        } else {
            self.links.insert(source.to_string(), vec![link]);
        }
    }

    fn add_resource_variable(&mut self, resource: &Resource, destination: &str, item: &str, problem: &mut Problem) -> () {
        let link: Link = Link::new(item, destination, problem.add_var(resource.weight, (0.0, resource.limit)));
        if self.links.contains_key(item) {
            self.links.get_mut(item).unwrap().push(link);
        } else {
            self.links.insert(item.to_string(), vec![link]);
        }
    }

    fn get_incoming_for_item(&self, destination: &str, item: &str) -> Vec<&Link> {
        let mut incoming: Vec<&Link> = Vec::new();
        for links in self.links.values() {
            for link in links.iter() {
                if (link.destination == destination) && (link.item == item) {
                    incoming.push(&link);
                }
            }
        }
        return incoming;
    }

    fn get_outgoing_for_item(&self, source: &str, item: &str) -> Vec<&Link> {
        let mut outgoing: Vec<&Link> = Vec::new();
        for link in self.links.get(source).unwrap().iter() {
            if link.item == item {
                outgoing.push(&link);
            }
        }
        return outgoing;
    }
}

#[derive(Debug)]
pub struct Solver {
    data: GameData,
    resources: HashMap<String, Resource>,
    disallowed_recipes: HashSet<String>,
    preserved_recipes: HashSet<String>,
    byproduct_coefficient: f64,
    targets: HashMap<String, f64>,

    problem: Problem,
    links: LinkSet,
}

impl Solver {
    pub fn new(data: GameData, targets: HashMap<String, f64>) -> Solver {
        return Solver {
            data,
            resources: HashMap::from([
                ("Desc_Coal_C".to_string(),         Resource { limit: 30120.0,  weight: 10.0    } ),
                ("Desc_LiquidOil_C".to_string(),    Resource { limit: 11700.0,  weight: 25.0    } ),
                ("Desc_NitrogenGas_C".to_string(),  Resource { limit: 12000.0,  weight: 50.0    } ),
                ("Desc_OreBauxite_C".to_string(),   Resource { limit: 9780.0,   weight: 50.0    } ),
                ("Desc_OreCopper_C".to_string(),    Resource { limit: 28860.0,  weight: 10.0    } ),
                ("Desc_OreGold_C".to_string(),      Resource { limit: 11040.0,  weight: 25.0    } ),
                ("Desc_OreIron_C".to_string(),      Resource { limit: 70380.0,  weight: 10.0    } ),
                ("Desc_OreUranium_C".to_string(),   Resource { limit: 2100.0,   weight: 100.0   } ),
                ("Desc_RawQuartz_C".to_string(),    Resource { limit: 10500.0,  weight: 50.0    } ),
                ("Desc_Stone_C".to_string(),        Resource { limit: 52860.0,  weight: 10.0    } ),
                ("Desc_Sulfur_C".to_string(),       Resource { limit: 6840.0,   weight: 50.0    } ),
                ("Desc_Water_C".to_string(),        Resource { limit: f64::MAX, weight: 1.0     } ),
            ]),
            disallowed_recipes: HashSet::new(),
            preserved_recipes: HashSet::new(),
            byproduct_coefficient: 1000.0,
            targets,

            problem: Problem::new(OptimizationDirection::Minimize),
            links: LinkSet::new(),
        };
    }

    pub fn add_resource(&mut self, resource: &str, amount: f64) -> () {
        self.resources.insert(resource.to_string(), Resource { limit: amount, weight: 0.0 });
    }

    pub fn remove_recipe(&mut self, recipe_id: &str) -> () {
        self.disallowed_recipes.insert(recipe_id.to_string());
    }

    pub fn preserve_recipe(&mut self, recipe_id: &str) -> () {
        self.preserved_recipes.insert(recipe_id.to_string());
    }

    pub fn remove_alternates(&mut self) -> () {
        for recipe in self.data.recipes.values() {
            if recipe.alternate && !self.preserved_recipes.contains(&recipe.class_name) {
                self.disallowed_recipes.insert(recipe.class_name.clone());
            }
        }
    }

    fn is_feasible(&self, recipe_id: &str) -> bool {
        let mut resource_provided: bool;
        let mut no_recipes_exist: bool;
        for ingredient in self.data.get_ingredients(recipe_id) {
            resource_provided = self.resources.contains_key(&ingredient);
            no_recipes_exist = self.data.get_item_creators(&ingredient, &self.disallowed_recipes).is_empty();
            if !resource_provided && no_recipes_exist  {
                return false;
            }
        }
        return true;
    }

    fn trim_parents(&mut self, recipe_id: &str) -> () {
        for product in self.data.get_products(recipe_id) {
            for parent_id in self.data.get_item_users(&product, &self.disallowed_recipes) {
                if !self.is_feasible(&parent_id) {
                    self.disallowed_recipes.insert(parent_id.clone());
                    self.trim_parents(&parent_id);
                }
            }
        }
    }

    fn add_variables(&mut self, recipe_id: &str) -> () {
        for ingredient in self.data.get_ingredients(recipe_id) {
            if self.resources.contains_key(&ingredient) {
                let resource: &Resource = self.resources.get(&ingredient).unwrap();
                self.links.add_resource_variable(resource, recipe_id, &ingredient, &mut self.problem);
            }
            for child_recipe_id in self.data.get_item_creators(&ingredient, &self.disallowed_recipes) {
                let exists: bool = self.links.links.contains_key(&child_recipe_id);
                self.links.add_simple_variable(&child_recipe_id, recipe_id, &ingredient, &mut self.problem);
                if !exists {
                    self.add_variables(&child_recipe_id);
                }
            }
        }
    }

    fn get_underclock(&self, recipe_id: &str, item_id: &str, rate: f64) -> f64 {
        if recipe_id.starts_with("Desc_") {
            return rate;
        }
        let recipe: &Recipe = self.data.get_recipe(recipe_id);
        let mut amount: f64 = 0.0;
        for product in recipe.products.iter() {
            if product.item == item_id {
                amount = product.amount;
            }
        }
        return rate / ((rate * recipe.time) / (60.0 * amount)).ceil();
    }

    pub fn solve(mut self) -> Factory {
        let mut infeasible_recipes: Vec<String> = Vec::new();
        for recipe_id in self.data.recipes.keys().filter(|&recipe_id| !self.disallowed_recipes.contains(recipe_id)) {
            if !self.is_feasible(recipe_id) {
                infeasible_recipes.push(recipe_id.clone());
            }
        }
        for recipe_id in infeasible_recipes {
            self.remove_recipe(&recipe_id);
            self.trim_parents(&recipe_id);
        }
        let mut recipes_to_add: Vec<String> = Vec::new();
        for (output_item_id, output_item_rate) in self.targets.iter() {
            if let None = self.data.items.get(output_item_id) {
                panic!("\"{}\" is not an item", output_item_id);
            }
            for output_recipe_id in self.data.get_item_creators(output_item_id, &self.disallowed_recipes) {
                self.links.add_simple_variable(&output_recipe_id, output_item_id, output_item_id, &mut self.problem);
                recipes_to_add.push(output_recipe_id);
            }
            self.problem.add_constraint(self.links.get_incoming_for_item(output_item_id, output_item_id).iter().map(|&l| (l.variable, 1.0)), ComparisonOp::Eq, *output_item_rate);
        }
        for recipe_id in recipes_to_add {
            self.add_variables(&recipe_id);
        }
        //This is kinda ass
        for (node_id, links) in self.links.links.clone().iter() {
            if node_id.starts_with("Desc_") {
                continue;
            }
            'outer: for product in self.data.get_products(node_id) {
                for link in links {
                    if link.item == product {
                        continue 'outer;
                    }
                }
                self.links.add_weighted_variable(node_id, &product, &product, self.byproduct_coefficient, &mut self.problem);
            }
        }
        for (node_id, _) in self.links.links.iter() {
            if node_id.starts_with("Desc_") {
                continue;
            }
            let recipe: &Recipe = self.data.recipes.get(node_id).unwrap();
            for ItemQuantity { item: ingredient, amount: in_rate } in recipe.ingredients.iter() {
                for ItemQuantity { item: product, amount: out_rate } in recipe.products.iter() {
                    let inputs: Vec<&Link> = self.links.get_incoming_for_item(node_id, ingredient);
                    let outputs: Vec<&Link> = self.links.get_outgoing_for_item(node_id, product);
                    let initial_lhs: Vec<(&Link, f64)> = inputs.iter().map(|&l| (l, *out_rate)).chain(outputs.iter().map(|&l| (l, -*in_rate))).collect::<Vec<(&Link, f64)>>();
                    let mut temp_lhs: HashMap<usize, Vec<(&Link, f64)>> = HashMap::new();
                    for (link, coefficient) in initial_lhs {
                        let idx: usize = link.variable.idx();
                        if temp_lhs.contains_key(&idx) {
                            temp_lhs.get_mut(&idx).unwrap().push((link, coefficient));
                        } else {
                            temp_lhs.insert(idx, vec![(link, coefficient)]);
                        }
                    }
                    let mut lhs: LinearExpr = LinearExpr::empty();
                    for var_group in temp_lhs.values() {
                        let coefficient: f64 = var_group.iter().fold(0.0, |acc, &(_, coefficient)| acc + coefficient);
                        lhs.add(var_group.get(0).unwrap().0.variable, coefficient);
                    }
                    self.problem.add_constraint(lhs, ComparisonOp::Eq, 0.0);
                }
            }
        }
        let solution: Solution = self.problem.solve().unwrap();
        let mut factory: Factory = Factory::new();
        let mut node_type: NodeType;
        for (node_id, _) in self.links.links.iter() {
            if node_id.starts_with("Desc_") {
                if self.targets.contains_key(node_id) {
                    node_type = NodeType::Output;
                } else {
                    node_type = NodeType::Input;
                }
                factory.add_node(node_id, &self.data.get_item_name(node_id), node_type);
            } else {
                factory.add_node(node_id, &self.data.get_recipe_name(node_id), NodeType::Production);
            }
        }
        for (node_id, links) in self.links.links.iter() {
            for link in links.iter() {
                let item_name: &String = &self.data.get_item_name(&link.item);
                let item_rate: f64 = solution[link.variable];
                let other_node_name: String = if link.destination.starts_with("Desc_") { self.data.get_item_name(&link.destination) } else { self.data.get_recipe_name(&link.destination) };
                let this_node_name: String = if node_id.starts_with("Desc_") { self.data.get_item_name(node_id) } else { self.data.get_recipe_name(node_id) };
                factory.add_node_output(node_id, &link.destination, ItemRate { name: item_name.clone(), other_node_name, rate: item_rate, underclock: self.get_underclock(node_id, &link.item, item_rate) });
                if !factory.nodes.contains_key(&link.destination) {
                    factory.add_node(&link.destination, &self.data.get_item_name(&link.destination), NodeType::Output);
                }
                factory.add_node_input(&link.destination, node_id, ItemRate { name: item_name.clone(), other_node_name: this_node_name, rate: item_rate, underclock: 0.0 })
            }
        }
        for (node_id, node) in factory.nodes.clone().iter() {
            let production_node: &mut Node = factory.nodes.get_mut(node_id).unwrap();
            for (item_id, item_rate) in node.inputs.iter() {
                if item_rate.rate.abs() <= 0.00001 {
                    production_node.inputs.remove(item_id);
                }
            }
            for (item_id, item_rate) in node.outputs.iter() {
                if item_rate.rate.abs() <= 0.00001 {
                    production_node.outputs.remove(item_id);
                }
            }
            if production_node.inputs.is_empty() && production_node.outputs.is_empty() {
                factory.nodes.remove(node_id);
            }
        }
        return factory;
    }
}

#[derive(Clone)]
pub struct ItemRate {
    pub name: String,
    pub other_node_name: String,
    pub rate: f64,
    pub underclock: f64,
}

#[derive(Clone)]
pub enum NodeType {
    Input,
    Production,
    Output,
}

#[derive(Clone)]
pub struct Node {
    pub node_type: NodeType,
    pub name: String,
    pub inputs: HashMap<String, ItemRate>,
    pub outputs: HashMap<String, ItemRate>,
}

pub struct Factory {
    pub nodes: HashMap<String, Node>,
}

impl Factory {
    fn new() -> Factory {
        return Factory {
            nodes: HashMap::new(),
        };
    }

    fn add_node(&mut self, node_id: &str, name: &str, node_type: NodeType) -> () {
        self.nodes.insert(node_id.to_string(), Node { node_type, name: name.to_string(), inputs: HashMap::new(), outputs: HashMap::new() });
    }

    fn add_node_input(&mut self, node_id: &str, source: &str, input: ItemRate) {
        self.nodes.get_mut(node_id).unwrap().inputs.insert(source.to_string(), input);
    }

    fn add_node_output(&mut self, node_id: &str, destination: &str, output: ItemRate) {
        self.nodes.get_mut(node_id).unwrap().outputs.insert(destination.to_string(), output);
    }
}