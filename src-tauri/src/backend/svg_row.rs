use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use serde::{Serialize, Serializer};

#[derive(Clone)]
pub enum SVGRowPropertyAttrs {
    SomeString(String),
    SomeInt(isize),
}

impl Serialize for SVGRowPropertyAttrs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match &self {
            SVGRowPropertyAttrs::SomeString(st) => st.serialize(serializer),
            SVGRowPropertyAttrs::SomeInt(i) => i.serialize(serializer),
        }
    }
}

#[derive(Clone)]
pub enum SVGRowProperty {
    SomeInt(isize),
    SomeString(String),
    SomeHashMap(HashMap<String, SVGRowPropertyAttrs>),
}

impl Serialize for SVGRowProperty {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match &self {
            SVGRowProperty::SomeInt(i) => i.serialize(serializer),
            SVGRowProperty::SomeString(st) => st.serialize(serializer),
            SVGRowProperty::SomeHashMap(hm) => hm.serialize(serializer),
        }
    }
}

#[derive(Clone)]
pub enum DrawProperty {
    SomeHashMap(HashMap<String, SVGRowProperty>),
    SomeVector(Vec<HashMap<String, SVGRowProperty>>),
    SomeVectorVector(Vec<Vec<HashMap<String, SVGRowProperty>>>),
}

impl Serialize for DrawProperty {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match &self {
            DrawProperty::SomeHashMap(hm) => hm.serialize(serializer),
            DrawProperty::SomeVector(v) => v.serialize(serializer),
            DrawProperty::SomeVectorVector(v) => v.serialize(serializer),
        }
    }
}

const Y_OFFSET: isize = 20;
const X_OFFSET: isize = 20;  // If changing, be sure to update on front-end too
const X_SPACING: isize = 20;  // If changing, be sure to update on front-end too
const Y_SPACING: isize = 30;
const TEXT_Y_ALIGNMENT: isize = 6;
const CIRCLE_RADIUS: isize = 10;
const RECT_Y_OFFSET: isize = -12;
const RECT_HEIGHT: isize = 24;

#[derive(Clone)]
pub struct SVGRow {
    sha: String,
    summary: String,
    branches_and_tags: Vec<(String, String)>,
    parent_oids: Vec<String>,
    child_oids: Vec<String>,
    x: isize,
    y: isize,
}

impl SVGRow {
    pub fn new(sha: String, summary: String, branches_and_tags: Vec<(String, String)>, parent_oids: Vec<String>, child_oids: Vec<String>, x: isize, y: isize) -> Self {
        Self {
            sha,
            summary,
            branches_and_tags,
            parent_oids,
            child_oids,
            x,
            y,
        }
    }

    pub fn get_parent_or_child_svg_row_values(&self, all_svg_rows: &HashMap<String, Rc<RefCell<SVGRow>>>, sha_type: String) -> Result<Vec<Rc<RefCell<SVGRow>>>, Box<dyn std::error::Error>> {
        let mut svg_row_values: Vec<Rc<RefCell<SVGRow>>> = vec![];
        let shas;
        if sha_type == "parents" {
            shas = &self.parent_oids;
        } else if sha_type == "children" {
            shas = &self.child_oids;
        } else {
            return Err("Please use parents or children for sha_type.".into());
        }
        for sha in shas {
            match all_svg_rows.get(&*sha) {
                Some(svg_row_rc) => {
                    svg_row_values.push(svg_row_rc.clone());
                },
                // If a parent or child is not present, ignore it. It may be outside the revwalk range.
                None => (),
            };
        }
        Ok(svg_row_values)
    }

    fn get_color_string(x: isize) -> String {
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

    pub fn get_draw_properties(&mut self, main_table: &mut HashMap<isize, HashMap<isize, bool>>, parent_svg_rows: Vec<Rc<RefCell<SVGRow>>>, child_svg_rows: Vec<Rc<RefCell<SVGRow>>>) -> Vec<DrawProperty> {
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
                let mut temp_hm: HashMap<isize, bool> = HashMap::new();
                temp_hm.insert(self.x, true);
                main_table.insert(self.y, temp_hm);
            },
        };

        // Set the space of the line from the current node to its parents as occupied.
        for parent_svg_row in parent_svg_rows {
            for i in (self.y + 1)..parent_svg_row.borrow().y {
                match main_table.get_mut(&i) {
                    Some(hm) => {
                        if !hm.contains_key(&self.x) {
                            hm.insert(self.x, true);
                        }
                    },
                    None => {
                        let mut temp_hm: HashMap<isize, bool> = HashMap::new();
                        temp_hm.insert(self.x, true);
                        main_table.insert(i, temp_hm);
                    },
                };
            }
        }

        let mut draw_properties: Vec<DrawProperty> = vec![];

        let pixel_x = self.x * X_SPACING + X_OFFSET;
        let pixel_y = self.y * Y_SPACING + Y_OFFSET;
        let color = SVGRow::get_color_string(self.x);
        let mut child_lines: Vec<HashMap<String, SVGRowProperty>> = vec![];
        // Draw the lines from the current node to its children.
        for child_svg_row in child_svg_rows {
            let child_svg_row_b = child_svg_row.borrow();
            let child_pixel_x = child_svg_row_b.x * X_SPACING + X_OFFSET;
            let child_pixel_y = child_svg_row_b.y * Y_SPACING + Y_OFFSET;
            let before_y = self.y - 1;
            let before_pixel_y = before_y * Y_SPACING + Y_OFFSET;
            if before_pixel_y != child_pixel_y {
                for i in child_svg_row_b.y..before_y {
                    let top_pixel_y = i * Y_SPACING + Y_OFFSET;
                    let bottom_pixel_y = (i + 1) * Y_SPACING + Y_OFFSET;

                    let mut style_str = String::from("stroke:");
                    style_str.push_str(&*SVGRow::get_color_string(child_svg_row_b.x));
                    style_str.push_str(";stroke-width:4");
                    let line_attrs: HashMap<String, SVGRowPropertyAttrs> = HashMap::from([
                        (String::from("x1"), SVGRowPropertyAttrs::SomeInt(child_pixel_x)),
                        (String::from("y1"), SVGRowPropertyAttrs::SomeInt(top_pixel_y)),
                        (String::from("x2"), SVGRowPropertyAttrs::SomeInt(child_pixel_x)),
                        (String::from("y2"), SVGRowPropertyAttrs::SomeInt(bottom_pixel_y)),
                        (String::from("style"), SVGRowPropertyAttrs::SomeString(style_str)),
                    ]);
                    child_lines.push(HashMap::from([
                        (String::from("tag"), SVGRowProperty::SomeString(String::from("line"))),
                        (String::from("attrs"), SVGRowProperty::SomeHashMap(line_attrs)),
                        (String::from("row-y"), SVGRowProperty::SomeInt(i + 1)),
                    ]));
                }
            }
            let mut style_str = String::from("stroke:");
            let row_y;
            if child_svg_row_b.x >= self.x {
                // Sets the color for "branching" lines and straight lines
                style_str.push_str(&*SVGRow::get_color_string(child_svg_row_b.x));
                row_y = self.y;
            } else {
                // Sets the color for "merging" lines
                style_str.push_str(&*SVGRow::get_color_string(self.x));
                row_y = before_y;
            }
            style_str.push_str(";fill:transparent;stroke-width:4");
            if child_pixel_x == pixel_x {
                let line_attrs: HashMap<String, SVGRowPropertyAttrs> = HashMap::from([
                    (String::from("x1"), SVGRowPropertyAttrs::SomeInt(child_pixel_x)),
                    (String::from("y1"), SVGRowPropertyAttrs::SomeInt(before_pixel_y)),
                    (String::from("x2"), SVGRowPropertyAttrs::SomeInt(pixel_x)),
                    (String::from("y2"), SVGRowPropertyAttrs::SomeInt(pixel_y)),
                    (String::from("style"), SVGRowPropertyAttrs::SomeString(style_str)),
                ]);
                child_lines.push(HashMap::from([
                    (String::from("tag"), SVGRowProperty::SomeString(String::from("line"))),
                    (String::from("attrs"), SVGRowProperty::SomeHashMap(line_attrs)),
                    (String::from("row-y"), SVGRowProperty::SomeInt(row_y)),
                ]));
            } else {
                let mut d_str = format!("M {child_pixel_x} {before_pixel_y} C ");
                if child_pixel_x < pixel_x {
                    let start_control_point_x = child_pixel_x + X_SPACING * 3 / 4;
                    let end_control_point_y = pixel_y - Y_SPACING * 3 / 4;
                    d_str.push_str(&*format!("{start_control_point_x} {before_pixel_y}, {pixel_x} {end_control_point_y}, "));
                } else {
                    let start_control_point_y = before_pixel_y + Y_SPACING * 3 / 4;
                    let end_control_point_x = pixel_x + X_SPACING * 3 / 4;
                    d_str.push_str(&*format!("{child_pixel_x} {start_control_point_y}, {end_control_point_x} {pixel_y}, "));
                }
                d_str.push_str(&*format!("{pixel_x} {pixel_y}"));
                let path_attrs: HashMap<String, SVGRowPropertyAttrs> = HashMap::from([
                    (String::from("d"), SVGRowPropertyAttrs::SomeString(d_str)),
                    (String::from("style"), SVGRowPropertyAttrs::SomeString(style_str)),
                ]);
                child_lines.push(HashMap::from([
                    (String::from("tag"), SVGRowProperty::SomeString(String::from("path"))),
                    (String::from("attrs"), SVGRowProperty::SomeHashMap(path_attrs)),
                    (String::from("row-y"), SVGRowProperty::SomeInt(row_y)),
                ]));
            }
        }
        draw_properties.push(DrawProperty::SomeVector(child_lines));

        // Now get the circle
        let circle_attrs: HashMap<String, SVGRowPropertyAttrs> = HashMap::from([
            (String::from("cx"), SVGRowPropertyAttrs::SomeInt(pixel_x)),
            (String::from("cy"), SVGRowPropertyAttrs::SomeInt(pixel_y)),
            (String::from("r"), SVGRowPropertyAttrs::SomeInt(CIRCLE_RADIUS)),
            (String::from("stroke"), SVGRowPropertyAttrs::SomeString(color.clone())),
            (String::from("stroke-width"), SVGRowPropertyAttrs::SomeInt(1)),
            (String::from("fill"), SVGRowPropertyAttrs::SomeString(color.clone())),
        ]);
        draw_properties.push(DrawProperty::SomeHashMap(HashMap::from([
            (String::from("tag"), SVGRowProperty::SomeString(String::from("circle"))),
            (String::from("attrs"), SVGRowProperty::SomeHashMap(circle_attrs)),
        ])));

        // Get the branch text
        let empty_hm = HashMap::new();
        let largest_occupied_x = main_table.get(&self.y).unwrap_or(&empty_hm).keys().max().unwrap_or(&0);
        let mut branch_and_tags: Vec<Vec<HashMap<String, SVGRowProperty>>> = vec![];
        for (branch_name, branch_type) in self.branches_and_tags.clone().into_iter() {
            let mut branch_and_tag_properties: Vec<HashMap<String, SVGRowProperty>> = vec![];
            let text_attrs: HashMap<String, SVGRowPropertyAttrs> = HashMap::from([
                (String::from("x"), SVGRowPropertyAttrs::SomeInt(0)),
                (String::from("y"), SVGRowPropertyAttrs::SomeInt(pixel_y + TEXT_Y_ALIGNMENT)),
                (String::from("fill"), SVGRowPropertyAttrs::SomeString(String::from("white"))),
            ]);
            branch_and_tag_properties.push(HashMap::from([
                (String::from("tag"), SVGRowProperty::SomeString(String::from("text"))),
                (String::from("attrs"), SVGRowProperty::SomeHashMap(text_attrs)),
                (String::from("textContent"), SVGRowProperty::SomeString(branch_name.clone())),
                (String::from("largestXValue"), SVGRowProperty::SomeInt(*largest_occupied_x)),
            ]));

            let mut branch_rect_color = "yellow";
            if branch_type == "local" {
                branch_rect_color = "red";
            } else if branch_type == "remote" {
                branch_rect_color = "green";
            } else if branch_type == "tag" {
                branch_rect_color = "grey";
            }

            let mut style_str = String::from("fill:");
            style_str.push_str(branch_rect_color);
            style_str.push_str(";fill-opacity:0.5;");
            let rect_attrs: HashMap<String, SVGRowPropertyAttrs> = HashMap::from([
                (String::from("x"), SVGRowPropertyAttrs::SomeInt(0)),
                (String::from("y"), SVGRowPropertyAttrs::SomeInt(pixel_y + RECT_Y_OFFSET)),
                (String::from("rx"), SVGRowPropertyAttrs::SomeInt(10)),
                (String::from("ry"), SVGRowPropertyAttrs::SomeInt(10)),
                (String::from("width"), SVGRowPropertyAttrs::SomeInt(0)),
                (String::from("height"), SVGRowPropertyAttrs::SomeInt(RECT_HEIGHT)),
                (String::from("style"), SVGRowPropertyAttrs::SomeString(style_str)),
            ]);
            branch_and_tag_properties.push(HashMap::from([
                (String::from("tag"), SVGRowProperty::SomeString(String::from("rect"))),
                (String::from("attrs"), SVGRowProperty::SomeHashMap(rect_attrs)),
            ]));
            branch_and_tags.push(branch_and_tag_properties);
        }
        draw_properties.push(DrawProperty::SomeVectorVector(branch_and_tags));

        // Get summary text
        let text_attrs: HashMap<String, SVGRowPropertyAttrs> = HashMap::from([
            (String::from("x"), SVGRowPropertyAttrs::SomeInt(0)),
            (String::from("y"), SVGRowPropertyAttrs::SomeInt(pixel_y + TEXT_Y_ALIGNMENT)),
            (String::from("fill"), SVGRowPropertyAttrs::SomeString(String::from("white"))),
        ]);
        draw_properties.push(DrawProperty::SomeHashMap(HashMap::from([
            (String::from("tag"), SVGRowProperty::SomeString(String::from("text"))),
            (String::from("attrs"), SVGRowProperty::SomeHashMap(text_attrs)),
            (String::from("textContent"), SVGRowProperty::SomeString(self.summary.clone())),
            (String::from("largestXValue"), SVGRowProperty::SomeInt(*largest_occupied_x)),
        ])));

        // Get background rectangle
        let rect_attrs: HashMap<String, SVGRowPropertyAttrs> = HashMap::from([
            (String::from("class"), SVGRowPropertyAttrs::SomeString(String::from("backgroundRect"))),
            (String::from("x"), SVGRowPropertyAttrs::SomeInt(pixel_x)),
            (String::from("y"), SVGRowPropertyAttrs::SomeInt(pixel_y + RECT_Y_OFFSET)),
            (String::from("width"), SVGRowPropertyAttrs::SomeInt(0)),
            (String::from("height"), SVGRowPropertyAttrs::SomeInt(RECT_HEIGHT)),
            (String::from("style"), SVGRowPropertyAttrs::SomeString(String::from("fill:white;fill-opacity:0.1;"))),
        ]);
        draw_properties.push(DrawProperty::SomeHashMap(HashMap::from([
            (String::from("tag"), SVGRowProperty::SomeString(String::from("rect"))),
            (String::from("attrs"), SVGRowProperty::SomeHashMap(rect_attrs)),
        ])));

        draw_properties
    }
}
