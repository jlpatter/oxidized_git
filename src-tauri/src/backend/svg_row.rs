/*
Example:
<circle cx="20" cy="20" r="10" stroke="#00CC19" stroke-width="1" fill="#00CC19"></circle>
 */

use std::collections::HashMap;
use serde::{Serialize, Serializer};

enum SVGRowPropertyAttrs {
    SomeString(String),
    SomeInt(usize),
}

impl Serialize for SVGRowPropertyAttrs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match &self {
            SVGRowPropertyAttrs::SomeString(st) => st.serialize(serializer),
            SVGRowPropertyAttrs::SomeInt(i) => i.serialize(serializer),
        }
    }
}

enum SVGRowProperty {
    SomeString(String),
    SomeHashMap(HashMap<String, SVGRowPropertyAttrs>),
}

impl Serialize for SVGRowProperty {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match &self {
            SVGRowProperty::SomeString(st) => st.serialize(serializer),
            SVGRowProperty::SomeHashMap(hm) => hm.serialize(serializer),
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
const FONT_SIZE: &str = "16px";

struct SVGRow {
    sha: String,
    summary: String,
    branches_and_tags: Vec<(String, String)>,
    x: usize,
    y: usize,
    width: usize,
}

impl SVGRow {
    pub fn new(sha: String, summary: String, branches_and_tags: Vec<(String, String)>, x: usize, y: usize, width: usize) -> Self {
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
                None => return Err("Commit had parents or children that are not present from the revwalk.".into()),
            };
        }
        Ok(svg_row_values)
    }

    fn get_color_string(x: usize) -> String {
        let color_num = x % 4;
        if color_num == 0 {
            String::from("#00CC19")
        } else if color_num == 1 {
            String::from("#0198A6")
        } else if color_num == 2 {
            String::from("#FF7800")
        } else {
            String::from("#FF0D00")
        }
    }

    pub fn get_draw_properties(&mut self, main_table: &mut HashMap<usize, HashMap<usize, bool>>, parent_svg_rows: Vec<(usize, usize)>, child_svg_rows: Vec<(usize, usize)>) -> Result<Vec<HashMap<String, SVGRowProperty>>, Box<dyn std::error::Error>> {
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

        // Set the space of the line from the current node to its parents as occupied.
        for (_, parent_svg_row_y) in parent_svg_rows {
            for i in (self.y + 1)..parent_svg_row_y {
                match main_table.get_mut(&i) {
                    Some(hm) => {
                        if !hm.contains_key(&self.x) {
                            hm.insert(self.x, true);
                        }
                    },
                    None => {
                        let mut temp_hm: HashMap<usize, bool> = HashMap::new();
                        temp_hm.insert(self.x, true);
                        main_table.insert(i, temp_hm);
                    },
                };
            }
        }

        let mut draw_properties: Vec<HashMap<String, SVGRowProperty>> = vec![];

        let pixel_x = self.x * X_SPACING + X_OFFSET;
        let pixel_y = self.y * Y_SPACING + Y_OFFSET;
        let color = SVGRow::get_color_string(self.x);
        // Draw the lines from the current node to its children.
        for (child_svg_row_x, child_svg_row_y) in child_svg_rows {
            let child_pixel_x = child_svg_row_x * X_SPACING + X_OFFSET;
            let child_pixel_y = child_svg_row_y * Y_SPACING + Y_OFFSET;
            let before_pixel_y = (self.y - 1) * Y_SPACING + Y_OFFSET;
            if before_pixel_y != child_pixel_y {
                let mut style_str = String::from("stroke:");
                style_str.push_str(&*SVGRow::get_color_string(child_svg_row_x));
                style_str.push_str(";stroke-width:4");
                let line_attrs: HashMap<String, SVGRowPropertyAttrs> = HashMap::from([
                    (String::from("x1"), SVGRowPropertyAttrs::SomeInt(child_pixel_x)),
                    (String::from("y1"), SVGRowPropertyAttrs::SomeInt(child_pixel_y)),
                    (String::from("x2"), SVGRowPropertyAttrs::SomeInt(child_pixel_x)),
                    (String::from("y2"), SVGRowPropertyAttrs::SomeInt(before_pixel_y)),
                    (String::from("style"), SVGRowPropertyAttrs::SomeString(style_str)),
                ]);
                draw_properties.push(HashMap::from([
                    (String::from("tag"), SVGRowProperty::SomeString(String::from("line"))),
                    (String::from("attrs"), SVGRowProperty::SomeHashMap(line_attrs)),
                ]));
            }
            let mut style_str = String::from("stroke:");
            style_str.push_str(&*SVGRow::get_color_string(child_svg_row_x));
            style_str.push_str(";stroke-width:4");
            let line_attrs: HashMap<String, SVGRowPropertyAttrs> = HashMap::from([
                (String::from("x1"), SVGRowPropertyAttrs::SomeInt(child_pixel_x)),
                (String::from("y1"), SVGRowPropertyAttrs::SomeInt(before_pixel_y)),
                (String::from("x2"), SVGRowPropertyAttrs::SomeInt(pixel_x)),
                (String::from("y2"), SVGRowPropertyAttrs::SomeInt(pixel_y)),
                (String::from("style"), SVGRowPropertyAttrs::SomeString(style_str)),
            ]);
            draw_properties.push(HashMap::from([
                (String::from("tag"), SVGRowProperty::SomeString(String::from("line"))),
                (String::from("attrs"), SVGRowProperty::SomeHashMap(line_attrs)),
            ]));
        }

        // Now get the circle
        let circle_attrs: HashMap<String, SVGRowPropertyAttrs> = HashMap::from([
            (String::from("cx"), SVGRowPropertyAttrs::SomeInt(pixel_x)),
            (String::from("cy"), SVGRowPropertyAttrs::SomeInt(pixel_y)),
            (String::from("r"), SVGRowPropertyAttrs::SomeInt(CIRCLE_RADIUS)),
            (String::from("stroke"), SVGRowPropertyAttrs::SomeString(color.clone())),
            (String::from("stroke-width"), SVGRowPropertyAttrs::SomeInt(1)),
            (String::from("fill"), SVGRowPropertyAttrs::SomeString(color.clone())),
        ]);
        draw_properties.push(HashMap::from([
            (String::from("tag"), SVGRowProperty::SomeString(String::from("circle"))),
            (String::from("attrs"), SVGRowProperty::SomeHashMap(circle_attrs)),
        ]));

        // Get the branch text
        let empty_hm = HashMap::new();
        let largest_occupied_x = main_table.get(&self.y).unwrap_or(&empty_hm).keys().max().unwrap_or(&0);
        let current_x = (largest_occupied_x + 1) * X_SPACING + X_OFFSET;
        for (branch_name, branch_type) in &self.branches_and_tags {
            let text_attrs: HashMap<String, SVGRowPropertyAttrs> = HashMap::from([
                (String::from("x"), SVGRowPropertyAttrs::SomeInt(current_x)),
                (String::from("y"), SVGRowPropertyAttrs::SomeInt(pixel_y + TEXT_Y_ALIGNMENT)),
                (String::from("fill"), SVGRowPropertyAttrs::SomeString(String::from("white"))),
                (String::from("font-size"), SVGRowPropertyAttrs::SomeString(String::from(FONT_SIZE))),
            ]);
            draw_properties.push(HashMap::from([
                (String::from("tag"), SVGRowProperty::SomeString(String::from("text"))),
                (String::from("attrs"), SVGRowProperty::SomeHashMap(text_attrs)),
                (String::from("textContent"), SVGRowProperty::SomeString(branch_name.clone())),
            ]));
            let mut branch_rect_id = String::from("branch_rect_");
        }

        Ok(draw_properties)
    }
}