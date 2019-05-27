use std::io::BufReader;
use std::io::BufWriter;
use std::fs::File;
use std::fs;
use std::env;
use std::path::PathBuf;
use serde_json::{Result, Value};
use rand::Rng;
use std::io::Write;
use std::fs::OpenOptions;
use std::io::SeekFrom;
use std::io::Seek;
use std::collections::HashMap;
use serde_json::map::Map;

#[derive(Debug)]
#[derive(Clone)]
enum LogicDependency {
	Item(String),
	Tag(String),
	And(Vec<LogicDependency>),
	Or(Vec<LogicDependency>),
}

#[derive(Debug)]
#[derive(Clone)]
struct Recipe {
	ingredients: LogicDependency,
	result: String,
	file: String,
}


fn main() {
	let mut rng = rand::thread_rng();

	// part 1: get all recipe files from './recipes'
	let all_recipe_files = get_all_dir_files("recipes");
	// and all tag files
	let all_tag_files = get_all_dir_files("tags");
	
	// part 1.5: extract all tag logic from './tags'
	let tag_bindings = get_all_tags(&all_tag_files);
	// as well as all config age groups
	let age_groups: HashMap<String, Vec<String>> = get_all_age_groups();

	// part 2: extract all recipe inputs and outputs from those files
	let mut all_recipes = get_all_products(&all_recipe_files);
	// also collect all recipe results into one list
	let mut all_products = get_products_from_recipes(&all_recipes);
	// also get all dead-end products
	let dead_end_products = get_dead_end_products(&all_recipes, &all_products, &tag_bindings);

	// part 3: do the shuffle!
	let mut unlocked_items = age_groups.get("base_items")
		.expect("No base items in config!")
		.clone();

	let mut not_craftable_recipes = all_recipes.clone();
	let mut craftable_recipes: Vec<Recipe> = Vec::new();
	let mut scrambled_recipes: Vec<Recipe> = Vec::new();

	let mut remaining_products = all_products.clone();

	let mut iterations: i32 = 0;

	loop {
		find_craftable_recipes(&unlocked_items, &mut craftable_recipes, &mut not_craftable_recipes, &tag_bindings);

		if (craftable_recipes.len() == 0) {
			let mut random_num_3: usize = 0;
			let mut iterations: i32 = 0;
			loop {
				random_num_3 = rng.gen_range(0, scrambled_recipes.len());

				let result = &scrambled_recipes[random_num_3].result;

				if dead_end_products.contains(&result) {
					break;
				}

				iterations += 1;
				if iterations > 10 {
					random_num_3 = scrambled_recipes.len()-1;
					break;
					//panic!("I think we have a problem... {:?}", unlocked_items)
				}
			}

			let removed_recipe = scrambled_recipes.remove(random_num_3);
			find_and_remove(&mut unlocked_items, &removed_recipe.result);
			remaining_products.push(removed_recipe.result.clone());
			craftable_recipes.push(removed_recipe);

			//println!("Rerouting...");
			//panic!("ahh bad seeeed! {:?}", scrambled_recipes.len());
		}

		let random_num_1: usize = rng.gen_range(0, craftable_recipes.len());
		let random_num_2: usize = rng.gen_range(0, remaining_products.len());
		
		let chosen_item: String = remaining_products.remove(random_num_2).to_string();
		println!("{:?}", chosen_item);
		unlock_items_and_check(&mut unlocked_items, &chosen_item, &age_groups);
		//println!("{:?} -> {:?} ({:?})", craftable_recipes[random_num_1].file, chosen_item, craftable_recipes.len());
		craftable_recipes[random_num_1].result = chosen_item;
		scrambled_recipes.push(craftable_recipes.remove(random_num_1));
		
		if (remaining_products.len() == 0) {
			break;
		}
		iterations += 1;
	}

	for s in &scrambled_recipes {
		println!("{:?} -> {:?}", s.file, s.result);
	}

	// part 4: export values
	for recipe_file in all_recipe_files.clone().iter() {
		
		let f = OpenOptions::new()
			.read(true)
			.write(true)
			.open(recipe_file).unwrap();
		let mut reader = BufReader::new(&f);

		let mut data: Value = serde_json::from_reader(reader)
			.expect(&format!("Unable to read JSON in {:?}!", recipe_file));

		let result_item = Value::String(get_result_from_file(&recipe_file, &scrambled_recipes));

		match &mut data {
			Value::Object(ref mut obj) => {

				let mut result = obj.get_mut("result")
					.expect(&format!("No result in {:?}!", recipe_file));

				match result {
					Value::Object(ref mut result_obj) => {

						let mut item = result_obj.get_mut("item")
							.expect(&format!("No item in {:?}!", recipe_file));
						
						*item = result_item;
					}
					Value::String(s) => {
						*result = result_item;
					}
					_ => {panic!("malformed result in {:?}!", recipe_file);}
				}

			}
			_ => {panic!("malformed JSON in {:?}!", recipe_file);}
		}

		let new_json = serde_json::to_string(&data)
			.expect(&format!("error writing new JSON for {:?}!", recipe_file));

		f.set_len(0);

		let mut writer = BufWriter::new(f);
		writer.seek(SeekFrom::Start(0));
		writer.write(new_json.as_bytes());

	}
	


}

fn get_result_from_file(file: &PathBuf, recipes: &Vec<Recipe> ) -> String {

	let mut filename = file.file_name().unwrap().to_str().unwrap().to_string();
	//filename = filename[1..filename.len()-2].to_string();

	for r in recipes {
		println!("{:?} {:?}", r.file, filename);
		if r.file == filename {
			return r.result.to_string();
		}
	}
	panic!("Nothing with filename {:?} found! {:?}", filename, recipes);

}

fn find_and_remove(items: &mut Vec<String>, to_remove: &str) {
	for n in (0..items.len()).rev() {
		if items[n] == to_remove {
			items.remove(n);
		}
	}
}

fn get_dead_end_products(recipes: &Vec<Recipe>, products: &Vec<String>, tags: &HashMap<String, LogicDependency>) -> Vec<String> {
	let mut products_left = products.clone();
	
	for recipe in recipes {
		let requirements = rec_get_all_referenced_recipes(&recipe.ingredients, tags);

		for i in (0..products_left.len()).rev() {
			let product = products_left[i].clone();
			
			for r in requirements.clone() {
				if r == product {
					products_left.remove(i);
				}
			}

		}
	}
	products_left
}

fn rec_get_all_referenced_recipes (logic: &LogicDependency, tags: &HashMap<String, LogicDependency>) -> Vec<String> {
	match logic {
		LogicDependency::Item(s) => {
			let mut vec = Vec::new();
			vec.push(s.to_string());
			return vec;
		}
		LogicDependency::Tag(s) => {
			return rec_get_all_referenced_recipes(
				tags.get(s)
					.expect(&format!("Tag not found! {:?}", s)),
				tags
			)
		}
		LogicDependency::And(v) | LogicDependency::Or(v) => {
			let mut vec = Vec::new();
			for l in v {
				vec.extend(rec_get_all_referenced_recipes(l, tags)); 
			}
			return vec;
		}
	}
}

fn unlock_items_and_check (unlocked_items: &mut Vec<String>, chosen_item: &str, age_groups: &HashMap<String, Vec<String>>) {

	if unlock_item(unlocked_items, chosen_item) {

		if unlocked_items.contains(&"minecraft:fishing_rod".to_string()) {
			unlock_all_items(unlocked_items, age_groups.get("fishing_rod_items").unwrap());
		}
		else if unlocked_items.contains(&"minecraft:wooden_pickaxe".to_string()) {
			unlock_all_items(unlocked_items, age_groups.get("wood_pick_items").unwrap());
		}
		else if unlocked_items.contains(&"minecraft:stone_pickaxe".to_string()) || unlocked_items.contains(&"minecraft:gold_pickaxe".to_string()) {
			unlock_all_items(unlocked_items, age_groups.get("wood_pick_items").unwrap());
			unlock_all_items(unlocked_items, age_groups.get("stone_pick_items").unwrap());
		}
		else if unlocked_items.contains(&"minecraft:iron_pickaxe".to_string()) {
			unlock_all_items(unlocked_items, age_groups.get("wood_pick_items").unwrap());
			unlock_all_items(unlocked_items, age_groups.get("stone_pick_items").unwrap());
			unlock_all_items(unlocked_items, age_groups.get("iron_pick_items").unwrap());
		}
		else if unlocked_items.contains(&"minecraft:diamond_pickaxe".to_string()) {
			unlock_all_items(unlocked_items, age_groups.get("wood_pick_items").unwrap());
			unlock_all_items(unlocked_items, age_groups.get("stone_pick_items").unwrap());
			unlock_all_items(unlocked_items, age_groups.get("iron_pick_items").unwrap());
			if unlocked_items.contains(&"minecraft:flint_and_steel".to_string()) || unlocked_items.contains(&"minecraft:fire_charge".to_string()) {
				unlock_all_items(unlocked_items, age_groups.get("nether_items").unwrap());
			}
		}
		else if unlocked_items.contains(&"minecraft:enchanting_table".to_string()) && unlocked_items.contains(&"minecraft:bookshelf".to_string()) {
			unlock_all_items(unlocked_items, age_groups.get("enchantment_items").unwrap());
		}
		else if unlocked_items.contains(&"minecraft:shears".to_string()) {
			unlock_all_items(unlocked_items, age_groups.get("shears_items").unwrap());
		}
		else if unlocked_items.contains(&"minecraft:wooden_hoe".to_string()) || unlocked_items.contains(&"minecraft:stone_hoe".to_string())
			|| unlocked_items.contains(&"minecraft:gold_hoe".to_string()) || unlocked_items.contains(&"minecraft:iron_hoe".to_string())
			|| unlocked_items.contains(&"minecraft:diamond_hoe".to_string()) {
			unlock_all_items(unlocked_items, age_groups.get("hoe_items").unwrap());
		}
		else if unlocked_items.contains(&"minecraft:wooden_shovel".to_string()) || unlocked_items.contains(&"minecraft:stone_shovel".to_string())
			|| unlocked_items.contains(&"minecraft:gold_shovel".to_string()) || unlocked_items.contains(&"minecraft:iron_shovel".to_string())
			|| unlocked_items.contains(&"minecraft:diamond_shovel".to_string()) {
			unlock_all_items(unlocked_items, age_groups.get("shovel_items").unwrap());
		}
		else if unlocked_items.contains(&"minecraft:ender_eye".to_string()) {
			unlock_all_items(unlocked_items, age_groups.get("end_items").unwrap());
		}

	}
}

fn unlock_all_items (unlocked_items: &mut Vec<String>, chosen_items: &Vec<String>) {
	for item in chosen_items {

		unlock_item(unlocked_items, item);

	}
}

fn unlock_item (unlocked_items: &mut Vec<String>, chosen_item: &str) -> bool {

	if unlocked_items.contains(&chosen_item.to_string()) {
		return false;
	}

	unlocked_items.push(chosen_item.to_string());

	true
}

fn find_craftable_recipes (unlocked_items: &Vec<String>, craftable_recipes: &mut Vec<Recipe>, not_craftable_recipes: &mut Vec<Recipe>, tags: &HashMap<String, LogicDependency>) {

	for n in (0..not_craftable_recipes.len()).rev() {
		let recipe = &not_craftable_recipes[n];

		if rec_solve_logic(&recipe.ingredients, unlocked_items, tags) {

			craftable_recipes.push(not_craftable_recipes.remove(n));

		}
	}

}

fn get_products_from_recipes(recipes: &Vec<Recipe>) -> Vec<String> {
	let mut result: Vec<String> = Vec::new();

	for recipe in recipes {
		result.push(recipe.result.clone());
	}

	result
}

fn rec_solve_logic(logic: &LogicDependency, items: &Vec<String>, tags: &HashMap<String, LogicDependency>) -> bool {
	match logic {
		LogicDependency::Item(s) => {
			return items.contains(&s);
		}
		LogicDependency::Tag(s) => {
			return rec_solve_logic(
				tags.get(s)
					.expect(&format!("Tag not found! {:?}", s)),
				items,
				tags
			)
		}
		LogicDependency::And(v) => {
			for l in v {
				if !rec_solve_logic(l, items, tags) {
					return false;
				}
			}
			return true;
		}
		LogicDependency::Or(v) => {
			for l in v {
				if rec_solve_logic(l, items, tags) {
					return true;
				}
			}
			return false;
		}
	}
}

fn get_all_products(all_recipe_files: &Vec<PathBuf>) -> Vec<Recipe> {
	
	let mut all_products: Vec<Recipe> = Vec::new();

	for recipe_file in all_recipe_files.clone().iter() {
		
		let f = File::open(recipe_file).unwrap();
		let mut reader = BufReader::new(f);

		let data: Value = serde_json::from_reader(reader)
			.expect(&format!("Unable to read JSON in {:?}!", recipe_file));

		match data {
			Value::Object(obj) => {
				
				let r = Recipe { 
					ingredients: get_logic_from_data(&obj, recipe_file.to_str().unwrap()), 
					result: get_result_from_data(obj, recipe_file.to_str().unwrap()),
					file: recipe_file.file_name().unwrap().to_str().unwrap().to_string() };
				all_products.push(r);
				
			}
			_ => {panic!("malformed JSON in {:?}!", recipe_file);}
		}

	}

	all_products
}

fn get_crafting_pattern_size (pattern: &Vec<Value>) -> u8 {
	if pattern.len() > 2 || pattern[0].as_str().unwrap().len() > 2 {
		return 3;
	}
	2
}

fn get_logic_from_data(obj: &Map<String, Value>, recipe_file: &str) -> LogicDependency {
	let rtype = obj.get("type")
		.expect(&format!("No type tag in {:?}!", recipe_file))
		.as_str()
		.expect(&format!("Incorrect type tag in {:?}!", recipe_file));

	if rtype == "minecraft:crafting_shaped" {

		let key = obj.get("key")
			.expect(&format!("No key tag in {:?}!", recipe_file))
			.as_object()
			.expect(&format!("Broken key tag in {:?}!", recipe_file));
		
		let mut and_compound: Vec<LogicDependency> = Vec::new();
		and_compound.push(get_logic_from_key(key, recipe_file));

		let pattern_arr = &obj.get("pattern").unwrap().as_array().unwrap();
		if get_crafting_pattern_size(pattern_arr) == 3 { 
			and_compound.push(LogicDependency::Item("minecraft:crafting_table".to_string()));
		}

		return LogicDependency::And(and_compound);

	}
	else if rtype == "minecraft:crafting_shapeless" {
		match obj.get("ingredients") {
			Option::Some(Value::Array(i)) => {
				let ret =  get_logic_from_options(i, recipe_file, true);
				match ret.clone() {
					LogicDependency::And(i) => {
						if i.len() > 4 {
							let mut v = Vec::new();
							v.push(ret);
							v.push(LogicDependency::Item("minecraft:crafting_table".to_string()));
							return LogicDependency::And(v);
						}
						ret
					}
					_ => {
						return ret;
					}
				}
			}
			_ => {
				panic!("Broken ingredients specifier in {:?}!", recipe_file);
			}
		}
	}
	else {
		let mut v = Vec::new();
		
		if rtype == "minecraft:stonecutting" {
			v.push(LogicDependency::Item("minecraft:stonecutter".to_string()));
		}
		else if rtype == "minecraft:smelting" {
			v.push(LogicDependency::Item("minecraft:furnace".to_string()));
		}
		else if rtype == "minecraft:campfire_cooking" {
			v.push(LogicDependency::Item("minecraft:campfire".to_string()));
		}
		else if rtype == "minecraft:smoking" {
			v.push(LogicDependency::Item("minecraft:smoker".to_string()));
		}
		else if rtype == "minecraft:blasting" {
			v.push(LogicDependency::Item("minecraft:blast_furnace".to_string()));
		}
		else {
			panic!("No recipe binding for this {:?}!", recipe_file);
		}


		match obj.get("ingredients") {
			Option::Some(i) => {
				v.push(get_logic_from_item(i, recipe_file));
				return LogicDependency::And(v);
			}
			_ => {

				match obj.get("ingredient") {
					Option::Some(i) => {
						v.push(get_logic_from_item(i, recipe_file));
						return LogicDependency::And(v);
					}
					_ => {
						panic!("Broken ingredients specifier in {:?}!", recipe_file);
					}
				}

			}
		}

	}

}

fn get_logic_from_key(key: &Map<String, Value>, recipe_file: &str) -> LogicDependency  {
	let mut dependencies: Vec<LogicDependency> = Vec::new();
	
	for i in key.values() {

		dependencies.push(get_logic_from_item(i, recipe_file));

	}

	LogicDependency::And(dependencies)
}

fn get_logic_from_options(tags: &Vec<Value>, recipe_file: &str, shapeless: bool) -> LogicDependency  {
	let mut dependencies: Vec<LogicDependency> = Vec::new();
	
	for tag in tags {
		dependencies.push(get_logic_from_item(tag, recipe_file));
	}

	if shapeless {
		LogicDependency::And(dependencies)
	}
	else {
		LogicDependency::Or(dependencies)
	}
}

fn get_logic_from_item(tag: &Value, recipe_file: &str) -> LogicDependency  {
	
	if tag.is_array() {
		return get_logic_from_options(tag.as_array().unwrap(), recipe_file, false);
	}

	let item = tag.as_object()
		.expect(&format!("Broken item tag in {:?}!", recipe_file));

	match item.get("item") {
		Option::Some(Value::String(s)) => {

			return LogicDependency::Item(s.to_string());

		}
		_ => {

			match item.get("tag") {
				Option::Some(Value::String(s)) => {

					return LogicDependency::Tag(s.to_string());

				}
				_ => {
					panic!("Broken item tag in {:?}!", recipe_file);
				}
			}

		}
	}

}

fn get_result_from_data(obj: Map<String, Value>, recipe_file: &str) -> String {
	let result = obj.get("result")
		.expect(&format!("No result in {:?}!", recipe_file));

	match result {
		Value::Object(result_obj) => {

			let item = result_obj.get("item")
				.expect(&format!("No item in {:?}!", recipe_file));
			
			return item.as_str().unwrap().to_string();
		}
		Value::String(s) => {
			return s.as_str().to_string();
		}
		_ => {panic!("malformed result in {:?}!", recipe_file);}
	}
}

fn get_all_tags(all_tag_files: &Vec<PathBuf>) -> HashMap<String, LogicDependency> {
	let mut all_dependencies: HashMap<String, LogicDependency> = HashMap::new();

	for tag_file in all_tag_files.clone().iter() {
		let f = File::open(tag_file).unwrap();
		let mut reader = BufReader::new(f);

		let data: Value = serde_json::from_reader(reader)
			.expect(&format!("Unable to read JSON in {:?}!", tag_file));

		let vals = data.get("values")
			.expect(&format!("No values {:?}!", tag_file));
		
		let mut dependencies: Vec<LogicDependency> = Vec::new();

		// still no clue why I need a double iterator here!
		for a in vals.as_array().iter() {
			for tag in a.iter() {
				dependencies.push(LogicDependency::Item(tag.as_str().unwrap().to_string()));
			}
		}

		let filename = tag_file.file_name().unwrap().to_str().unwrap().to_string();
		let len = filename.len();
		let new_filename = format!("minecraft:{}", (filename[0..len-5]).to_string());

		all_dependencies.insert(new_filename, LogicDependency::Or(dependencies));
		
	}

	all_dependencies
}

fn get_all_dir_files(folder: &str) -> Vec<PathBuf> {
	
	let mut recipes_dir = env::current_dir().unwrap();
	recipes_dir.push(folder);

	if !recipes_dir.exists() {
		panic!("The recipes directory {:?} doesn't exist!", recipes_dir)
	}

	let mut all_recipe_files: Vec<PathBuf> = Vec::new();

	// no idea why two loops are necessary.
	for a in fs::read_dir(recipes_dir) {
		for entry in a {
			all_recipe_files.push(entry.unwrap().path());
		}
	}

	all_recipe_files
}

fn get_all_age_groups() -> HashMap<String, Vec<String>> {
	let mut all_age_groups: HashMap<String, Vec<String>> = HashMap::new();

	let f = File::open("./config.json")
		.expect("Unable to find the config file!");
	let mut reader = BufReader::new(f);

	let data: Value = serde_json::from_reader(reader)
		.expect("Unable to read JSON in config!");
	let data_obj = data.as_object().unwrap();

	for k in data_obj.keys() {

		let mut items: Vec<String> = Vec::new();
		let list = data_obj.get(k).unwrap().as_array()
			.expect("Malformed config file!");

		for item in list {
			let final_string = format!("minecraft:{}", item.as_str().unwrap().to_string());
			items.push(final_string);
		}

		all_age_groups.insert(k.to_string(), items);

	}

	all_age_groups
}