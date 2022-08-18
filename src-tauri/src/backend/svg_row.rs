/*
Example:
<circle cx="20" cy="20" r="10" stroke="#00CC19" stroke-width="1" fill="#00CC19"></circle>
 */

use std::collections::HashMap;
use serde::{Serialize, Serializer};

enum SVGRowProperty {
    SomeString(String),
    SomeInt(usize),
}

impl Serialize for SVGRowProperty {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match &self {
            SVGRowProperty::SomeString(st) => st.serialize(serializer),
            SVGRowProperty::SomeInt(i) => i.serialize(serializer),
        }
    }
}

const Y_OFFSET: usize = 20;
const X_OFFSET: usize = 20;
const X_SPACING: usize = 20;
const Y_SPACING: usize = 30;
const TEXT_Y_ALIGNMENT: usize = 6;
const CIRCLE_RADIUS: usize = 10;
const RECT_Y_OFFSET: i32 = -12;
const RECT_HEIGHT: usize = 24;
const BRANCH_TEXT_SPACING: usize = 5;

struct SVGRow {
    sha: String,
    summary: String,
    branches_and_tags: Vec<String>,
    x: usize,
    y: usize,
    width: usize,
}

impl SVGRow {
    pub fn new(sha: String, summary: String, branches_and_tags: Vec<String>, x: usize, y: usize, width: usize) -> Self {
        Self {
            sha,
            summary,
            branches_and_tags,
            x,
            y,
            width,
        }
    }

    // Gets parent and children svg row values from parent and children shas
    fn get_parent_or_child_svg_row_values(&self, all_svg_rows: HashMap<String, SVGRow>, shas: Vec<String>) -> Result<Vec<(usize, usize)>, Box<dyn std::error::Error>> {
        let mut svg_row_values: Vec<(usize, usize)> = vec![];
        for sha in shas {
            match all_svg_rows.get(&*sha) {
                Some(s) => {
                    svg_row_values.push((s.x, s.y));
                },
                None => return Err("Commit had parents that are not present from the revwalk.".into()),
            };
        }
        Ok(svg_row_values)
    }

    pub fn get_draw_properties(&mut self, mut main_table: HashMap<usize, HashMap<usize, bool>>) -> Result<HashMap<String, SVGRowProperty>, Box<dyn std::error::Error>> {
        let mut draw_properties: HashMap<String, SVGRowProperty> = HashMap::new();

        // Set the current node position as occupied (or find a position that's unoccupied and occupy it).
        match main_table.get_mut(&self.y) {
            Some(hm) => {
                match hm.get(&self.x) {
                    Some(is_occupied) => {
                        if *is_occupied == true {
                            let mut found_empty = false;
                            while !found_empty {
                                self.x += 1;
                                if !hm.contains_key(&self.x) {
                                    found_empty = true;
                                    hm.insert(self.x, true);
                                }
                            }
                        }
                    },
                    None => {
                        hm.insert(self.x, true);
                    },
                };
            },
            None => {
                let mut temp_hm: HashMap<usize, bool> = HashMap::new();
                temp_hm.insert(self.x, true);
                main_table.insert(self.y, temp_hm);
            },
        };



        let pixel_x = self.x * X_SPACING + X_OFFSET;
        let pixel_y = self.y * Y_SPACING + Y_OFFSET;

        Ok(draw_properties)
    }
}