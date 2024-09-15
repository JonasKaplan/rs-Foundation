mod json;
mod solver;
use std::collections::HashMap;

use json::GameData;
use solver::{Factory, Solver};

fn main() -> () {
    let data: GameData = GameData::new("./static/data-old.json");
    let mut solver: Solver = Solver::new(data, HashMap::from([
        ("Desc_MotorLightweight_C".to_string(), 20.0),

    ]));
    solver.preserve_recipe("Recipe_Alternate_SteelRod_C");
    solver.preserve_recipe("Recipe_Alternate_ReinforcedIronPlate_2_C"); //stitched iron plate
    solver.preserve_recipe("Recipe_Alternate_Wire_2_C"); //caterium wire
    solver.preserve_recipe("Recipe_Alternate_CopperAlloyIngot_C");
    solver.preserve_recipe("Recipe_Alternate_Screw_C"); //cast screw
    solver.preserve_recipe("Recipe_Alternate_Rotor_C"); //steel rotor
    solver.preserve_recipe("Recipe_Alternate_HighSpeedWiring_C"); //automated speed wiring
    solver.remove_alternates();
    use std::time::Instant;
    let now = Instant::now();
    let factory: Factory = solver.solve();
    let elapsed = now.elapsed();
    println!("Solve time: {:.2?}", elapsed);
    for node in factory.nodes.values() {
        println!("\n{}:\n\tInputs:", node.name);
        for item_rate in node.inputs.values() {
            println!("\t\t{:.9} {} from node {}", item_rate.rate, item_rate.name, item_rate.other_node_name);
        }
        println!("\tOutputs:");
        for item_rate in node.outputs.values() {
            println!("\t\t{:.9} {} to node {} (clock: {:.9})", item_rate.rate, item_rate.name, item_rate.other_node_name, item_rate.underclock);
        }
    }
}
