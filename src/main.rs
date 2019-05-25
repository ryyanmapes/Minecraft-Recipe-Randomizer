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

fn main() {
	let mut rng = rand::thread_rng();

	// part 1: get all recipe files from './recipes'
	let all_recipe_files = get_all_recipe_files();

	// part 2: extract all recipe outputs from those files
	let mut all_products = get_all_products(&all_recipe_files);

	// part 3: add back values randomly
	for recipe_file in all_recipe_files.clone().iter() {
		
		let f = OpenOptions::new()
			.read(true)
			.write(true)
			.open(recipe_file).unwrap();
		let mut reader = BufReader::new(&f);

		let mut data: Value = serde_json::from_reader(reader)
			.expect(&format!("Unable to read JSON in {:?}!", recipe_file));

		match &mut data {
			Value::Object(ref mut obj) => {

				let mut result = obj.get_mut("result")
					.expect(&format!("No result in {:?}!", recipe_file));

				let random_num: usize = rng.gen_range(0, all_products.len());
				let random_item = Value::String(all_products.get(random_num).unwrap().to_string());


				match result {
					Value::Object(ref mut result_obj) => {

						let mut item = result_obj.get_mut("item")
							.expect(&format!("No item in {:?}!", recipe_file));
						
						*item = random_item;
					}
					Value::String(s) => {
						*result = random_item;
					}
					_ => {panic!("malformed result in {:?}!", recipe_file);}
				}

				all_products.remove(random_num);

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

fn get_all_products(all_recipe_files: &Vec<PathBuf>) -> Vec<String> {
	
	let mut all_products: Vec<String> = Vec::new();

	for recipe_file in all_recipe_files.clone().iter() {
		
		let f = File::open(recipe_file).unwrap();
		let mut reader = BufReader::new(f);

		let data: Value = serde_json::from_reader(reader)
			.expect(&format!("Unable to read JSON in {:?}!", recipe_file));

		match data {
			Value::Object(obj) => {

				let result = obj.get("result")
					.expect(&format!("No result in {:?}!", recipe_file));

				match result {
					Value::Object(result_obj) => {

						let item = result_obj.get("item")
							.expect(&format!("No item in {:?}!", recipe_file));
						
						all_products.push(item.as_str().unwrap().to_string());
					}
					Value::String(s) => {
						all_products.push(s.as_str().to_string());
					}
					_ => {panic!("malformed result in {:?}!", recipe_file);}
				}

			}
			_ => {panic!("malformed JSON in {:?}!", recipe_file);}
		}

	}

	all_products
}

fn get_all_recipe_files() -> Vec<PathBuf> {
	
	let mut recipes_dir = env::current_dir().unwrap();
	recipes_dir.push("recipes");

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
